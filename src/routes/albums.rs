use log::{info, error};
use rocket::serde::json::Json;
use rocket::State;
use std::fs;
use std::path::Path;

use crate::database::AlbumsDatabase;
use crate::models::Album;
use crate::response::{AlbumsResponse, AlbumResponse, SetCoverResponse, DeleteAlbumResponse};
use crate::AppConfig;

async fn albums_response<F, Fut>(fetch: F) -> Json<AlbumsResponse>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = mongodb::error::Result<Vec<Album>>>,
{
    match fetch().await {
        Ok(albums) => Json(AlbumsResponse {
            success: true,
            msg: None,
            albums,
        }),
        Err(e) => {
            error!("Failed to retrieve albums: {}", e);
            Json(AlbumsResponse {
                success: false,
                msg: Some("Failed to retrieve albums".to_string()),
                albums: vec![],
            })
        }
    }
}

#[get("/")]
pub fn index() -> &'static str {
    info!("Index endpoint accessed");
    "Hello, world!"
}

#[get("/api/albums")]
pub async fn get_albums(db: &State<AlbumsDatabase>) -> Json<AlbumsResponse> {
    albums_response(|| db.get_all_albums()).await
}

// TODO: add authentication guard for admin endpoints
#[get("/api/admin/albums")]
pub async fn get_all_albums_admin(db: &State<AlbumsDatabase>) -> Json<AlbumsResponse> {
    info!("Admin get all albums endpoint accessed");
    albums_response(|| db.get_all_albums_unfiltered()).await
}

#[get("/api/featured-albums")]
pub async fn get_featured_albums(db: &State<AlbumsDatabase>) -> Json<AlbumsResponse> {
    albums_response(|| db.get_featured_albums()).await
}

#[get("/api/album/<id>")]
pub async fn get_album_by_id(db: &State<AlbumsDatabase>, id: String) -> Json<AlbumResponse> {
    match db.get_album_by_id(&id).await {
        Ok(album) => Json(AlbumResponse {
            success: true,
            msg: None,
            album,
        }),
        Err(e) => {
            error!("Failed to retrieve album {}: {}", id, e);
            Json(AlbumResponse {
                success: false,
                msg: Some("Failed to retrieve album".to_string()),
                album: None,
            })
        }
    }
}

#[put("/api/album/<album_id>/cover", data = "<cover_data>")]
pub async fn set_album_cover(db: &State<AlbumsDatabase>, album_id: String, cover_data: Json<serde_json::Value>) -> Json<SetCoverResponse> {
    info!("Set album cover endpoint accessed for album: {}", album_id);

    let cover_filename = match cover_data.get("cover").and_then(|v| v.as_str()) {
        Some(filename) => filename.to_string(),
        None => {
            return Json(SetCoverResponse {
                success: false,
                msg: Some("Missing or invalid cover filename".to_string()),
            });
        }
    };

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

    album.cover = cover_filename.clone();

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
pub async fn delete_album(db: &State<AlbumsDatabase>, album_id: String, config: &State<AppConfig>) -> Json<DeleteAlbumResponse> {
    info!("Delete album endpoint accessed for album: {}", album_id);

    match db.delete_album(&album_id).await {
        Ok(true) => {
            let album_dir = Path::new(&config.upload_dir).join(&album_id);
            if album_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&album_dir) {
                    error!("Album deleted from DB but failed to remove directory {:?}: {}", album_dir, e);
                } else {
                    info!("Deleted album directory: {:?}", album_dir);
                }
            }
            info!("Successfully deleted album: {}", album_id);
            Json(DeleteAlbumResponse {
                success: true,
                msg: Some("Album successfully deleted".to_string()),
            })
        },
        Ok(false) => {
            Json(DeleteAlbumResponse {
                success: false,
                msg: Some("Album not found".to_string()),
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
