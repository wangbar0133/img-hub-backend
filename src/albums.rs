
use chrono::{DateTime, Utc};
use serde::{ Deserialize, Serialize };

use crate::pic::ImageInfo;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Photo {
    pub src:  String,
    pub detail: String,
    pub medium: String,
    pub thumbnail: String,
    pub info: ImageInfo
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub cover: String,
    pub category: String,
    pub shot_time: DateTime<Utc>,
    pub updata_time: DateTime<Utc>,
    pub photos: Vec<Photo>,
    pub featured: bool,
    pub hidden: bool
}

impl Photo {

    pub fn new(clean_base_name: &str, info: ImageInfo) -> Self {
        Self { 
            src: format!("{}.jpg", clean_base_name), 
            detail: format!("{}_detail.jpg", clean_base_name), 
            medium: format!("{}_medium.jpg", clean_base_name), 
            thumbnail: format!("{}_thumbnail.jpg", clean_base_name), 
            info: info
        }
    }
}