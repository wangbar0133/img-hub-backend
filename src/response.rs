use serde::Serialize;
use crate::models::{Album, UploadTask};

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AlbumsResponse {
    pub success: bool,
    pub msg: Option<String>,
    pub albums: Vec<Album>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AlbumResponse {
    pub success: bool,
    pub msg: Option<String>,
    pub album: Option<Album>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SetCoverResponse {
    pub success: bool,
    pub msg: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct DeleteAlbumResponse {
    pub success: bool,
    pub msg: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AsyncUploadResponse {
    pub success: bool,
    pub task_id: String,
    pub msg: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TaskStatusResponse {
    pub success: bool,
    pub task: Option<UploadTask>,
    pub msg: Option<String>,
}
