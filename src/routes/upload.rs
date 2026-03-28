use log::{info, error};
use rocket::serde::json::Json;
use rocket::{State, Data};
use rocket::http::ContentType;
use std::path::Path;
use uuid::Uuid;
use chrono::Utc;
use rocket_multipart_form_data::{MultipartFormData, MultipartFormDataOptions, MultipartFormDataField, Repetition};

use crate::database::AlbumsDatabase;
use crate::models::{Album, Photo, UploadTask, UploadTaskStatus};
use crate::response::{AsyncUploadResponse, TaskStatusResponse};
use crate::pic::ImageCompressor;
use crate::AppConfig;

struct UploadFormData {
    album_id: String,
    title: String,
    category: String,
    featured: bool,
    hidden: bool,
}

fn extract_text_field(form: &MultipartFormData, key: &str, default: &str) -> String {
    form.texts.get(key)
        .and_then(|texts| texts.first())
        .map(|text| text.text.clone())
        .unwrap_or_else(|| default.to_string())
}

fn extract_bool_field(form: &MultipartFormData, key: &str) -> bool {
    form.texts.get(key)
        .and_then(|texts| texts.first())
        .map(|text| {
            let value = text.text.trim().to_lowercase();
            value == "true" || value == "1" || value == "yes"
        })
        .unwrap_or(false)
}

fn parse_upload_form(form: &MultipartFormData) -> UploadFormData {
    UploadFormData {
        album_id: format!("album-{}", Uuid::new_v4()),
        title: extract_text_field(form, "title", "Untitled Album"),
        category: extract_text_field(form, "category", "cosplay"),
        featured: extract_bool_field(form, "featured"),
        hidden: extract_bool_field(form, "hidden"),
    }
}

#[post("/api/upload", data = "<data>")]
pub async fn upload_images_async(db: &State<AlbumsDatabase>, content_type: &ContentType, data: Data<'_>, config: &State<AppConfig>) -> Json<AsyncUploadResponse> {
    info!("Async image upload endpoint accessed");

    let task_id = format!("upload-{}", Uuid::new_v4());

    if let Err(e) = std::fs::create_dir_all(&config.upload_dir) {
        error!("Failed to create upload directory: {}", e);
        return Json(AsyncUploadResponse {
            success: false,
            task_id,
            msg: Some(format!("Failed to create upload directory err: {}", e)),
        });
    }

    let options = MultipartFormDataOptions::with_multipart_form_data_fields(vec![
        MultipartFormDataField::file("images").repetition(Repetition::infinite()),
        MultipartFormDataField::text("title"),
        MultipartFormDataField::text("category"),
        MultipartFormDataField::text("featured"),
        MultipartFormDataField::text("hidden"),
    ]);

    let multipart_form_data = match MultipartFormData::parse(content_type, data, options).await {
        Ok(form_data) => form_data,
        Err(e) => {
            error!("Failed to parse multipart form data: {}", e);
            return Json(AsyncUploadResponse {
                success: false,
                task_id,
                msg: Some(format!("Failed to parse upload data err: {}", e)),
            });
        }
    };

    let form_data = parse_upload_form(&multipart_form_data);
    let total_files = multipart_form_data.files.get("images").map(|f| f.len()).unwrap_or(0);

    if total_files == 0 {
        return Json(AsyncUploadResponse {
            success: false,
            task_id,
            msg: Some("No files to upload".to_string()),
        });
    }

    let upload_task = UploadTask {
        task_id: task_id.clone(),
        status: UploadTaskStatus::Processing,
        total_files,
        processed_files: 0,
        failed_files: 0,
        album_id: Some(form_data.album_id.clone()),
        error_message: None,
        created_at: Utc::now(),
        completed_at: None,
    };

    if let Err(e) = db.create_upload_task(upload_task).await {
        return Json(AsyncUploadResponse {
            success: false,
            task_id,
            msg: Some(format!("Failed to create upload task: {}", e)),
        });
    }

    let db_clone = db.inner().clone();
    let config_clone = config.inner().clone();
    let task_id_clone = task_id.clone();

    tokio::spawn(async move {
        process_upload_task(multipart_form_data, db_clone, config_clone, task_id_clone, form_data).await;
    });

    Json(AsyncUploadResponse {
        success: true,
        task_id,
        msg: Some(format!("Upload task created. Processing {} files in background.", total_files)),
    })
}

async fn process_single_file(
    index: usize,
    temp_path: std::path::PathBuf,
    album_dir: String,
    task_id: String,
    album_id: String,
    db: AlbumsDatabase,
) -> Result<(usize, String, Photo), (String, String)> {
    let file_id = Uuid::new_v4().to_string();
    let src_filename = format!("{}.jpg", file_id);
    info!("Processing file {}: {} -> {}", index + 1, temp_path.display(), file_id);

    let file_path = Path::new(&album_dir).join(&src_filename);

    match tokio::fs::copy(&temp_path, &file_path).await {
        Ok(_) => {
            // 验证文件是否为合法图片
            let validate_path = file_path.clone();
            let is_valid = tokio::task::spawn_blocking(move || image::open(&validate_path).is_ok())
                .await
                .unwrap_or(false);
            if !is_valid {
                let _ = tokio::fs::remove_file(&file_path).await;
                error!("File {} is not a valid image, rejected", file_id);
                update_task_progress(&db, &task_id, false).await;
                return Err((file_id, "Not a valid image file".to_string()));
            }

            info!("Successfully copied file: {}", src_filename);
            let output_dir = Path::new(&album_dir).to_path_buf();
            match ImageCompressor::generate_multiple_sizes(file_path, output_dir, &src_filename).await {
                Ok(mut photo) => {
                    photo.src = format!("{}/{}", album_id, photo.src);
                    photo.detail = format!("{}/{}", album_id, photo.detail);
                    photo.medium = format!("{}/{}", album_id, photo.medium);
                    photo.thumbnail = format!("{}/{}", album_id, photo.thumbnail);
                    info!("Successfully processed file: {}", file_id);
                    update_task_progress(&db, &task_id, true).await;
                    Ok((index, file_id, photo))
                },
                Err(e) => {
                    error!("Failed to generate sizes for {}: {}", file_id, e);
                    update_task_progress(&db, &task_id, false).await;
                    Err((file_id, format!("Size generation failed: {}", e)))
                }
            }
        },
        Err(e) => {
            error!("Failed to copy file {}: {}", file_id, e);
            update_task_progress(&db, &task_id, false).await;
            Err((file_id, format!("File copy failed: {}", e)))
        }
    }
}

async fn update_task_progress(db: &AlbumsDatabase, task_id: &str, success: bool) {
    if let Err(e) = db.increment_task_progress(task_id, success).await {
        error!("Failed to update task progress {}: {}", task_id, e);
    }
}

async fn create_album_from_photos(
    db: &AlbumsDatabase,
    form: &UploadFormData,
    photo_vec: Vec<Photo>,
) -> (UploadTaskStatus, Option<String>) {
    if photo_vec.is_empty() {
        return (UploadTaskStatus::Failed, Some("No photos were successfully processed".to_string()));
    }

    let first_photo = &photo_vec[0];
    let shot_time = first_photo.info.created_at.unwrap_or_else(Utc::now);

    let new_album = Album {
        id: form.album_id.clone(),
        title: form.title.clone(),
        cover: first_photo.medium.clone(),
        category: form.category.clone(),
        shot_time,
        update_time: Utc::now(),
        photos: photo_vec,
        featured: form.featured,
        hidden: form.hidden,
    };

    match db.add_new_album(new_album).await {
        Ok(_) => {
            info!("Successfully created album: {}", form.album_id);
            (UploadTaskStatus::Completed, None)
        },
        Err(e) => {
            error!("Failed to create album {}: {}", form.album_id, e);
            (UploadTaskStatus::Failed, Some(format!("Failed to create album: {}", e)))
        }
    }
}

async fn process_upload_task(
    multipart_form_data: MultipartFormData,
    db: AlbumsDatabase,
    config: AppConfig,
    task_id: String,
    form: UploadFormData,
) {
    info!("Starting background processing for task: {}", task_id);

    let album_dir = Path::new(&config.upload_dir).join(&form.album_id);
    if let Err(e) = tokio::fs::create_dir_all(&album_dir).await {
        error!("Failed to create album directory: {}", e);
        if let Err(e) = db.finalize_upload_task(&task_id, "Failed", Some(format!("Failed to create album directory: {}", e))).await {
            error!("Failed to update task status {}: {}", task_id, e);
        }
        return;
    }
    let album_dir_str = album_dir.to_string_lossy().to_string();

    let mut uploaded_files = vec![];
    let mut failed_files = vec![];
    let mut photo_vec: Vec<(usize, Photo)> = vec![];

    if let Some(files) = multipart_form_data.files.get("images") {
        info!("Processing {} files", files.len());

        let tasks: Vec<_> = files.iter().enumerate().map(|(index, file_field)| {
            let album_dir = album_dir_str.clone();
            let temp_path = file_field.path.clone();
            let task_id = task_id.clone();
            let album_id = form.album_id.clone();
            let db = db.clone();
            tokio::spawn(process_single_file(index, temp_path, album_dir, task_id, album_id, db))
        }).collect();

        for task in tasks {
            match task.await {
                Ok(Ok((index, filename, photo))) => {
                    uploaded_files.push(filename);
                    photo_vec.push((index, photo));
                },
                Ok(Err((filename, _))) => failed_files.push(filename),
                Err(e) => {
                    error!("Task join error: {}", e);
                    failed_files.push("unknown_file".to_string());
                }
            }
        }
    }

    // 按原始上传顺序排序
    photo_vec.sort_by_key(|(index, _)| *index);
    let photo_vec: Vec<Photo> = photo_vec.into_iter().map(|(_, photo)| photo).collect();

    let (status, error_message) = create_album_from_photos(&db, &form, photo_vec).await;

    // 任务失败时清理 album 目录
    if matches!(status, UploadTaskStatus::Failed) && album_dir.exists() {
        if let Err(e) = tokio::fs::remove_dir_all(&album_dir).await {
            error!("Failed to cleanup album directory {:?}: {}", album_dir, e);
        }
    }

    let status_str = match status {
        UploadTaskStatus::Completed => "Completed",
        UploadTaskStatus::Failed => "Failed",
        UploadTaskStatus::Processing => "Processing",
    };
    if let Err(e) = db.finalize_upload_task(&task_id, status_str, error_message).await {
        error!("Failed to update final task status {}: {}", task_id, e);
    }

    info!("Background processing completed for task: {}. Success: {}, Failed: {}",
          task_id, uploaded_files.len(), failed_files.len());
}

#[get("/api/upload-status/<task_id>")]
pub async fn get_upload_status(db: &State<AlbumsDatabase>, task_id: String) -> Json<TaskStatusResponse> {
    match db.get_upload_task(&task_id).await {
        Ok(Some(task)) => Json(TaskStatusResponse {
            success: true,
            task: Some(task),
            msg: None,
        }),
        Ok(None) => Json(TaskStatusResponse {
            success: false,
            task: None,
            msg: Some("Task not found".to_string()),
        }),
        Err(e) => Json(TaskStatusResponse {
            success: false,
            task: None,
            msg: Some(format!("Database error: {}", e)),
        }),
    }
}
