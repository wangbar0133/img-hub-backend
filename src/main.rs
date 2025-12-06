mod albums;
mod datebase;
mod response;
mod pic;

#[macro_use] extern crate rocket;
use log::info;
use rocket::fs::FileServer;
use rocket::serde::json::Json;
use rocket::{State, Data};
use rocket::http::ContentType;
use response::{AlbumsRespones, AlbumRespones, SetCoverResponse, DeleteAlbumResponse, AsyncUploadResponse, TaskStatusResponse, UploadTask, UploadTaskStatus};
use datebase::AlbumsDatesbase;
use rocket_multipart_form_data::{MultipartFormData, MultipartFormDataOptions, MultipartFormDataField, Repetition};
use std::fs;
use std::path::Path;
use uuid::Uuid;

use crate::albums::Album;
use crate::pic::ImageCompressor;
use chrono::Utc;

#[derive(Clone)]
pub struct AppConfig {
    pub upload_dir: String,
}

#[get("/")]
fn index() -> &'static str {
    info!("Index endpoint accessed");
    "Hello, world!"
}

#[get("/api/albums")]
async fn get_albums(db: &State<AlbumsDatesbase>) -> Json<AlbumsRespones> {
    match db.get_all_albums().await {
        Ok(albums) => Json(AlbumsRespones {
            success: true,
            msg: Option::None,
            albums,
        }),
        Err(_) => Json(AlbumsRespones {
            success: false,
            msg: Some("Failed to retrieve albums".to_string()),
            albums: vec![],
        }),
    }
}

#[get("/api/featured-albums")]
async fn get_featured_albums(db: &State<AlbumsDatesbase>) -> Json<AlbumsRespones> {
    match db.get_featured_albums().await {
        Ok(albums) => Json(AlbumsRespones {
            success: true,
            msg: Option::None,
            albums,
        }),
        Err(_) => Json(AlbumsRespones {
            success: false,
            msg: Some("Failed to retrieve albums".to_string()),
            albums: vec![],
        }),
    }
}

#[get("/api/album/<id>")]
async fn get_album_by_id(db: &State<AlbumsDatesbase>, id: String) -> Json<AlbumRespones> {
    match db.get_album_by_id(&id).await {
            Ok(album) => Json(AlbumRespones {
                success: true,
                msg: Option::None,
                album,
            }),
            Err(_) => Json(AlbumRespones {
                success: false,
                msg: Some("Failed to retrieve albums".to_string()),
                album: Option::None,
            }),
        }
}

#[post("/api/upload", data = "<data>")]
async fn upload_images_async(db: &State<AlbumsDatesbase>, content_type: &ContentType, data: Data<'_>, config: &State<AppConfig>) -> Json<AsyncUploadResponse> {
    info!("Async image upload endpoint accessed");

    // 生成任务ID
    let task_id = format!("upload-{}", Uuid::new_v4());

    // Get upload directory from global state
    let upload_dir = &config.upload_dir;
    if let Err(e) = fs::create_dir_all(upload_dir) {
        info!("Failed to create upload directory: {}", e);
        return Json(AsyncUploadResponse {
            success: false,
            task_id: task_id,
            msg: Some(format!("Failed to create upload directory err: {}", e).to_string()),
        });
    }

    // Configure multipart form data options
    let options = MultipartFormDataOptions::with_multipart_form_data_fields(
        vec![
            MultipartFormDataField::file("images").repetition(Repetition::infinite()),
            MultipartFormDataField::text("id"),
            MultipartFormDataField::text("title"),
            MultipartFormDataField::text("category"),
            MultipartFormDataField::text("featured"),
            MultipartFormDataField::text("hidden"),
        ]
    );

    let multipart_form_data = match MultipartFormData::parse(content_type, data, options).await {
        Ok(form_data) => form_data,
        Err(e) => {
            info!("Failed to parse multipart form data: {}", e);
            return Json(AsyncUploadResponse {
                success: false,
                task_id: task_id,
                msg: Some(format!("Failed to parse upload data err: {}", e).to_string()),
            });
        }
    };

    // Extract form text fields
    let album_id = multipart_form_data.texts.get("id")
        .and_then(|texts| texts.first())
        .map(|text| text.text.clone())
        .unwrap_or_else(|| format!("album-{}", Uuid::new_v4()));

    let title = multipart_form_data.texts.get("title")
        .and_then(|texts| texts.first())
        .map(|text| text.text.clone())
        .unwrap_or_else(|| "Untitled Album".to_string());

    let category = multipart_form_data.texts.get("category")
        .and_then(|texts| texts.first())
        .map(|text| text.text.clone())
        .unwrap_or_else(|| "cosplay".to_string());

    let featured = multipart_form_data.texts.get("featured")
        .and_then(|texts| texts.first())
        .map(|text| {
            let value = text.text.trim().to_lowercase();
            value == "true" || value == "1" || value == "yes"
        })
        .unwrap_or(false);

    let hidden = multipart_form_data.texts.get("hidden")
        .and_then(|texts| texts.first())
        .map(|text| {
            let value = text.text.trim().to_lowercase();
            value == "true" || value == "1" || value == "yes"
        })
        .unwrap_or(false);

    // 获取文件数量
    let total_files = multipart_form_data.files.get("images")
        .map(|files| files.len())
        .unwrap_or(0);

    if total_files == 0 {
        return Json(AsyncUploadResponse {
            success: false,
            task_id: task_id,
            msg: Some("No files to upload".to_string()),
        });
    }

    // 创建上传任务记录
    let upload_task = UploadTask {
        task_id: task_id.clone(),
        status: UploadTaskStatus::Processing,
        total_files,
        processed_files: 0,
        failed_files: 0,
        album_id: Some(album_id.clone()),
        error_message: None,
        created_at: Utc::now(),
        completed_at: None,
    };

    // 保存任务到数据库
    if let Err(e) = db.create_upload_task(upload_task).await {
        return Json(AsyncUploadResponse {
            success: false,
            task_id: task_id,
            msg: Some(format!("Failed to create upload task: {}", e)),
        });
    }

    // 启动后台处理任务
    let db_clone = db.inner().clone();
    let config_clone = config.inner().clone();
    let task_id_clone = task_id.clone();

    tokio::spawn(async move {
        process_upload_task(
            multipart_form_data,
            db_clone,
            config_clone,
            task_id_clone,
            album_id,
            title,
            category,
            featured,
            hidden,
        ).await;
    });

    // 立即返回任务ID
    Json(AsyncUploadResponse {
        success: true,
        task_id: task_id,
        msg: Some(format!("Upload task created. Processing {} files in background.", total_files)),
    })
}

// 后台处理上传任务的函数
async fn process_upload_task(
    multipart_form_data: MultipartFormData,
    db: AlbumsDatesbase,
    config: AppConfig,
    task_id: String,
    album_id: String,
    title: String,
    category: String,
    featured: bool,
    hidden: bool,
) {
    info!("Starting background processing for task: {}", task_id);

    let upload_dir = &config.upload_dir;
    let mut uploaded_files = vec![];
    let mut failed_files = vec![];
    let mut photo_vec = vec![];

    if let Some(files) = multipart_form_data.files.get("images") {
        info!("Processing {} files", files.len());

        let upload_tasks: Vec<_> = files.iter().enumerate().map(|(index, file_field)| {
            let filename = match &file_field.file_name {
                Some(name) => name.clone(),
                None => format!("{}.jpg", Uuid::new_v4()),
            };

            let upload_dir = upload_dir.clone();
            let temp_path = file_field.path.clone();
            let task_id = task_id.clone();
            let db = db.clone();

            tokio::spawn(async move {
                info!("Processing file {}: {}", index + 1, filename);

                // 原图路径
                let file_path = Path::new(&upload_dir).join(&filename);

                // 保存原图
                let copy_result = tokio::fs::copy(&temp_path, &file_path).await;

                let result = match copy_result {
                    Ok(_) => {
                        info!("Successfully copied file: {}", filename);

                        // 生成多尺寸图片
                        let output_dir = Path::new(&upload_dir).to_path_buf();
                        match ImageCompressor::generate_multiple_sizes(file_path, output_dir, &filename).await {
                            Ok(photo) => {
                                info!("Successfully processed file: {}", filename);
                                Ok((filename, photo))
                            },
                            Err(e) => {
                                error!("Failed to generate sizes for {}: {}", filename, e);
                                Err((filename, format!("Size generation failed: {}", e)))
                            }
                        }
                    },
                    Err(e) => {
                        error!("Failed to copy file {}: {}", filename, e);
                        Err((filename, format!("File copy failed: {}", e)))
                    }
                };

                // 更新任务进度
                if let Ok(Some(mut task)) = db.get_upload_task(&task_id).await {
                    match &result {
                        Ok(_) => task.processed_files += 1,
                        Err(_) => task.failed_files += 1,
                    }
                    let _ = db.update_upload_task(&task_id, task).await;
                }

                result
            })
        }).collect();

        // 等待所有任务完成
        for task in upload_tasks {
            match task.await {
                Ok(Ok((filename, photo))) => {
                    uploaded_files.push(filename);
                    photo_vec.push(photo);
                },
                Ok(Err((filename, _error))) => {
                    failed_files.push(filename);
                },
                Err(e) => {
                    error!("Task join error: {}", e);
                    failed_files.push("unknown_file".to_string());
                }
            }
        }
    }

    // 创建相册（如果有成功处理的照片）
    let mut final_task_status = UploadTaskStatus::Completed;
    let mut error_message = None;

    if !photo_vec.is_empty() {
        let first_photo = &photo_vec[0];
        let shot_time = first_photo.info.created_at.unwrap_or_else(|| Utc::now());

        let new_album = Album {
            id: album_id.clone(),
            title,
            cover: first_photo.medium.clone(),
            category,
            shot_time,
            updata_time: Utc::now(),
            photos: photo_vec,
            featured,
            hidden
        };

        match db.add_new_album(new_album).await {
            Ok(_) => {
                info!("Successfully created album for task: {}", task_id);
            },
            Err(e) => {
                error!("Failed to create album for task {}: {}", task_id, e);
                final_task_status = UploadTaskStatus::Failed;
                error_message = Some(format!("Failed to create album: {}", e));
            }
        }
    } else {
        final_task_status = UploadTaskStatus::Failed;
        error_message = Some("No photos were successfully processed".to_string());
    }

    // 更新最终任务状态
    if let Ok(Some(mut task)) = db.get_upload_task(&task_id).await {
        task.status = final_task_status;
        task.completed_at = Some(Utc::now());
        task.error_message = error_message;
        let _ = db.update_upload_task(&task_id, task).await;
    }

    info!("Background processing completed for task: {}. Success: {}, Failed: {}",
          task_id, uploaded_files.len(), failed_files.len());
}

#[get("/api/upload-status/<task_id>")]
async fn get_upload_status(db: &State<AlbumsDatesbase>, task_id: String) -> Json<TaskStatusResponse> {
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

#[put("/api/album/<album_id>/cover", data = "<cover_data>")]
async fn set_album_cover(db: &State<AlbumsDatesbase>, album_id: String, cover_data: Json<serde_json::Value>) -> Json<SetCoverResponse> {
    info!("Set album cover endpoint accessed for album: {}", album_id);

    // 从请求体中提取封面文件名
    let cover_filename = match cover_data.get("cover").and_then(|v| v.as_str()) {
        Some(filename) => filename.to_string(),
        None => {
            return Json(SetCoverResponse {
                success: false,
                msg: Some("Missing or invalid cover filename".to_string()),
            });
        }
    };

    // 首先获取现有相册
    let mut album = match db.get_album_by_id(&album_id).await {
        Ok(Some(album)) => album,
        Ok(None) => {
            return Json(SetCoverResponse {
                success: false,
                msg: Some("Album not found".to_string()),
            });
        },
        Err(e) => {
            return Json(SetCoverResponse {
                success: false,
                msg: Some(format!("Database error: {}", e)),
            });
        }
    };

    // 验证封面文件是否存在于相册的照片中
    let cover_exists = album.photos.iter().any(|photo| {
        photo.src == cover_filename || 
        photo.detail == cover_filename ||
        photo.medium == cover_filename ||
        photo.thumbnail == cover_filename
    });

    if !cover_exists {
        return Json(SetCoverResponse {
            success: false,
            msg: Some("Cover file does not exist in this album".to_string()),
        });
    }

    // 更新封面
    album.cover = cover_filename.clone();

    // 保存到数据库
    match db.update_album(&album_id, album).await {
        Ok(_) => {
            info!("Successfully updated album cover for album: {}", album_id);
            Json(SetCoverResponse {
                success: true,
                msg: Some(format!("Album cover updated to: {}", cover_filename)),
            })
        },
        Err(e) => {
            Json(SetCoverResponse {
                success: false,
                msg: Some(format!("Failed to update album: {}", e)),
            })
        }
    }
}

#[delete("/api/album/<album_id>")]
async fn delete_album(db: &State<AlbumsDatesbase>, album_id: String, config: &State<AppConfig>) -> Json<DeleteAlbumResponse> {
    info!("Delete album endpoint accessed for album: {}", album_id);

    // First, get the album to retrieve photo filenames for cleanup
    let album = match db.get_album_by_id(&album_id).await {
        Ok(Some(album)) => album,
        Ok(None) => {
            return Json(DeleteAlbumResponse {
                success: false,
                msg: Some("Album not found".to_string()),
            });
        },
        Err(e) => {
            return Json(DeleteAlbumResponse {
                success: false,
                msg: Some(format!("Database error: {}", e)),
            });
        }
    };

    // Delete photo files from filesystem
    let upload_dir = &config.upload_dir;
    let mut deleted_files = 0;
    let mut failed_deletions = Vec::new();

    for photo in &album.photos {
        let files_to_delete = vec![
            Path::new(upload_dir).join(&photo.src),
            Path::new(upload_dir).join(&photo.detail),
            Path::new(upload_dir).join(&photo.medium),
            Path::new(upload_dir).join(&photo.thumbnail),
        ];

        for file_path in files_to_delete {
            if file_path.exists() {
                match fs::remove_file(&file_path) {
                    Ok(_) => {
                        deleted_files += 1;
                        info!("Deleted file: {:?}", file_path);
                    },
                    Err(e) => {
                        let file_name = file_path.file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("unknown");
                        failed_deletions.push(file_name.to_string());
                        info!("Failed to delete file {:?}: {}", file_path, e);
                    }
                }
            }
        }
    }

    // Delete album from database
    match db.delete_album(&album_id).await {
        Ok(_) => {
            let msg = if failed_deletions.is_empty() {
                Some(format!("Album successfully deleted. Removed {} files.", deleted_files))
            } else {
                Some(format!(
                    "Album deleted from database. Removed {} files, failed to delete {} files: {}",
                    deleted_files,
                    failed_deletions.len(),
                    failed_deletions.join(", ")
                ))
            };
            
            info!("Successfully deleted album: {}", album_id);
            Json(DeleteAlbumResponse {
                success: true,
                msg,
            })
        },
        Err(e) => {
            Json(DeleteAlbumResponse {
                success: false,
                msg: Some(format!("Failed to delete album from database: {}", e)),
            })
        }
    }
}

#[launch]
async fn rocket() -> _ {
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Info)
        .init();
    info!("Starting img-hub-backend server");
    
    let db = match AlbumsDatesbase::new().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to initialize database: {}", e);
            std::process::exit(1);
        }
    };

    let config = AppConfig {
        upload_dir: std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "static/".to_string()),
    };

    // 配置Rocket服务器地址和端口
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .unwrap_or(8000);

    let figment = rocket::Config::figment()
        .merge(("address", host))
        .merge(("port", port));

    rocket::custom(figment)
        .manage(db)
        .manage(config)
        .mount(
            "/",
            routes![
                index,
                get_albums,
                get_featured_albums,
                get_album_by_id,
                upload_images_async,
                get_upload_status,
                set_album_cover,
                delete_album
            ]
        )
        .mount("/public", FileServer::from("static/"))
}