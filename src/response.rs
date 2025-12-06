use serde::{Serialize, Deserialize};

use crate::albums::Album;

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AlbumsRespones {
    pub success: bool,
    pub msg: Option<String>,
    pub albums: Vec<Album>
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AlbumRespones {
    pub success: bool,
    pub msg: Option<String>,
    pub album: Option<Album>
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct UploadResponse {
    pub success: bool,
    pub msg: Option<String>,
    pub uploaded_files: Vec<String>,
    pub failed_files: Vec<String>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub enum UploadTaskStatus {
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct UploadTask {
    pub task_id: String,
    pub status: UploadTaskStatus,
    pub total_files: usize,
    pub processed_files: usize,
    pub failed_files: usize,
    pub album_id: Option<String>,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
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