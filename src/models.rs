use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- Image & Photo ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub file_size: u64,
    pub created_at: Option<DateTime<Utc>>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub focal_length: Option<f64>,
    pub aperture: Option<f64>,
    pub exposure_time: Option<String>,
    pub iso: Option<u32>,
    pub flash: Option<String>,
    pub white_balance: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Photo {
    pub src: String,
    pub detail: String,
    pub medium: String,
    pub thumbnail: String,
    pub info: ImageInfo,
}

impl Photo {
    pub fn new(clean_base_name: &str, info: ImageInfo) -> Self {
        Self {
            src: format!("{}.jpg", clean_base_name),
            detail: format!("{}_detail.jpg", clean_base_name),
            medium: format!("{}_medium.jpg", clean_base_name),
            thumbnail: format!("{}_thumbnail.jpg", clean_base_name),
            info,
        }
    }
}

// --- Album ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub cover: String,
    pub category: String,
    pub shot_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub photos: Vec<Photo>,
    pub featured: bool,
    pub hidden: bool,
}

// --- Upload Task ---

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
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
