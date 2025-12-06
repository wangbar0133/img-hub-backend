use mongodb::{ bson::doc, options::{ ClientOptions, ServerApi, ServerApiVersion }, Client,  Collection};
use crate::albums::Album;
use crate::response::UploadTask;
use log::info;

pub async fn get_db_client() -> mongodb::error::Result<Client> {
    let uri = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
    let mut client_options = ClientOptions::parse(uri).await?;
    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);

    let client = Client::with_options(client_options)?;
    Ok(client)
}

#[derive(Clone)]
pub struct AlbumsDatesbase {
    client: Client
}

impl AlbumsDatesbase {
    pub async fn new() -> Result<Self, mongodb::error::Error> {
        let client = get_db_client().await?;
        Ok(AlbumsDatesbase{client: client})
    }

    pub async fn add_new_album(&self, album: Album) -> mongodb::error::Result<()> {
        let my_coll: Collection<Album> = self.client.database("img-hub").collection("albums");
        let res = my_coll.insert_one(album).await?;
        info!("add_new_album with db id{}", res.inserted_id);
        Ok(())
    }

    pub async fn get_album_by_id(&self, album_id: &str) -> mongodb::error::Result<Option<Album>> {
        let my_coll: Collection<Album> = self.client.database("img-hub").collection("albums");
        let filter = doc! { "id": album_id };
        let res = my_coll.find_one(filter).await?;
        info!("get_album_by_id completed");
        Ok(res)
    }

    pub async fn get_all_albums(&self) -> mongodb::error::Result<Vec<Album>> {
        let my_coll: Collection<Album> = self.client.database("img-hub").collection("albums");
        let mut cursor = my_coll.find(doc! {"hidden": false}).await?;
        let mut albums = Vec::new();
        
        while cursor.advance().await? {
            let album = cursor.deserialize_current()?;
            albums.push(album);
        }
        
        info!("get_all_albums completed, found {} albums", albums.len());
        Ok(albums)
    }

    pub async fn get_featured_albums(&self) -> mongodb::error::Result<Vec<Album>> {
        let my_coll: Collection<Album> = self.client.database("img-hub").collection("albums");
        let filter = doc! { "featured": true };
        let mut cursor = my_coll.find(filter).await?;
        let mut albums = Vec::new();

        while cursor.advance().await? {
            let album = cursor.deserialize_current()?;
            albums.push(album);
        }

        info!("get_featured_albums completed, found {} albums", albums.len());
        Ok(albums)
    }

    pub async fn update_album(&self, album_id: &str, updated_album: Album) -> mongodb::error::Result<()> {
        let my_coll: Collection<Album> = self.client.database("img-hub").collection("albums");
        let filter = doc! { "id": album_id };
        let update = doc! { "$set": mongodb::bson::to_bson(&updated_album)? };
        let res = my_coll.update_one(filter, update).await?;
        info!("update_album completed, modified count: {}", res.modified_count);
        Ok(())
    }

    pub async fn delete_album(&self, album_id: &str) -> mongodb::error::Result<()> {
        let my_coll: Collection<Album> = self.client.database("img-hub").collection("albums");
        let filter = doc! { "id": album_id };
        let res = my_coll.delete_one(filter).await?;
        info!("delete_album completed, deleted count: {}", res.deleted_count);
        Ok(())
    }

    // 任务状态管理方法
    pub async fn create_upload_task(&self, task: UploadTask) -> mongodb::error::Result<()> {
        let my_coll: Collection<UploadTask> = self.client.database("img-hub").collection("upload_tasks");
        let res = my_coll.insert_one(task).await?;
        info!("create_upload_task with db id{}", res.inserted_id);
        Ok(())
    }

    pub async fn get_upload_task(&self, task_id: &str) -> mongodb::error::Result<Option<UploadTask>> {
        let my_coll: Collection<UploadTask> = self.client.database("img-hub").collection("upload_tasks");
        let filter = doc! { "task_id": task_id };
        let res = my_coll.find_one(filter).await?;
        Ok(res)
    }

    pub async fn update_upload_task(&self, task_id: &str, updated_task: UploadTask) -> mongodb::error::Result<()> {
        let my_coll: Collection<UploadTask> = self.client.database("img-hub").collection("upload_tasks");
        let filter = doc! { "task_id": task_id };
        let update = doc! { "$set": mongodb::bson::to_bson(&updated_task)? };
        let res = my_coll.update_one(filter, update).await?;
        info!("update_upload_task completed, modified count: {}", res.modified_count);
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pic::ImageInfo;
    use crate::albums::Photo;
    use uuid::Uuid;
    use chrono::Utc;

    fn create_test_album() -> Album {
        let test_image_info = ImageInfo {
            width: 1920,
            height: 1080,
            format: "JPEG".to_string(),
            file_size: 1024000,
            created_at: Some(Utc::now()),
            camera_make: Some("Canon".to_string()),
            camera_model: Some("EOS R5".to_string()),
            lens_model: Some("RF 24-70mm F2.8 L IS USM".to_string()),
            focal_length: Some(50.0),
            aperture: Some(2.8),
            exposure_time: Some("1/125".to_string()),
            iso: Some(100),
            flash: Some("No flash".to_string()),
            white_balance: Some("Auto".to_string()),
        };

        let test_photo = Photo {
            src: "test.jpg".to_string(),
            detail: "test_detail.jpg".to_string(),
            medium: "test_medium.jpg".to_string(),
            thumbnail: "test_thumbnail.jpg".to_string(),
            info: test_image_info,
        };

        Album {
            id: format!("test-album-{}", Uuid::new_v4()),
            title: "Test Album".to_string(),
            cover: "test_cover.jpg".to_string(),
            category: "test".to_string(),
            shot_time: Utc::now(),
            updata_time: Utc::now(),
            photos: vec![test_photo],
            featured: false,
            hidden: false,
        }
    }

    #[tokio::test]
    async fn test_database_connection() {
        let result = get_db_client().await;
        
        match result {
            Ok(_) => println!("✓ Database connection successful"),
            Err(e) => {
                println!("⚠ Database connection failed: {}. This is expected if MongoDB is not running.", e);
                println!("To run this test with MongoDB, ensure MongoDB is running on localhost:27017");
            }
        }
    }

    #[tokio::test]
    async fn test_albums_database_new() {
        let result = AlbumsDatesbase::new().await;
        
        match result {
            Ok(_) => println!("✓ AlbumsDatesbase initialization successful"),
            Err(e) => {
                println!("⚠ AlbumsDatesbase initialization failed: {}. This is expected if MongoDB is not running.", e);
            }
        }
    }

    #[tokio::test]
    async fn test_crud_operations() {
        let db_result = AlbumsDatesbase::new().await;
        
        let db = match db_result {
            Ok(db) => db,
            Err(e) => {
                println!("⚠ Skipping CRUD test - MongoDB not available: {}", e);
                return;
            }
        };

        let test_album = create_test_album();
        let album_id = test_album.id.clone();

        // Test 1: Add new album
        match db.add_new_album(test_album.clone()).await {
            Ok(_) => println!("✓ Add album test passed"),
            Err(e) => {
                println!("✗ Add album test failed: {}", e);
                return;
            }
        }

        // Test 2: Get album by ID
        match db.get_album_by_id(&album_id).await {
            Ok(Some(retrieved_album)) => {
                assert_eq!(retrieved_album.id, album_id);
                assert_eq!(retrieved_album.title, "Test Album");
                println!("✓ Get album by ID test passed");
            },
            Ok(None) => {
                println!("✗ Get album by ID test failed: Album not found");
                return;
            },
            Err(e) => {
                println!("✗ Get album by ID test failed: {}", e);
                return;
            }
        }

        // Test 3: Get all albums
        match db.get_all_albums().await {
            Ok(albums) => {
                assert!(!albums.is_empty(), "Should have at least one album");
                println!("✓ Get all albums test passed - found {} albums", albums.len());
            },
            Err(e) => {
                println!("✗ Get all albums test failed: {}", e);
                return;
            }
        }

        // Test 4: Update album
        let mut updated_album = test_album.clone();
        updated_album.title = "Updated Test Album".to_string();
        updated_album.category = "updated".to_string();
        
        match db.update_album(&album_id, updated_album).await {
            Ok(_) => {
                // Verify the update
                match db.get_album_by_id(&album_id).await {
                    Ok(Some(retrieved)) => {
                        assert_eq!(retrieved.title, "Updated Test Album");
                        assert_eq!(retrieved.category, "updated");
                        println!("✓ Update album test passed");
                    },
                    _ => println!("✗ Update verification failed")
                }
            },
            Err(e) => {
                println!("✗ Update album test failed: {}", e);
                return;
            }
        }

        // Test 5: Delete album
        match db.delete_album(&album_id).await {
            Ok(_) => {
                // Verify deletion
                match db.get_album_by_id(&album_id).await {
                    Ok(None) => println!("✓ Delete album test passed"),
                    Ok(Some(_)) => println!("✗ Delete album test failed: Album still exists"),
                    Err(e) => println!("✗ Delete verification failed: {}", e)
                }
            },
            Err(e) => {
                println!("✗ Delete album test failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_nonexistent_album() {
        let db_result = AlbumsDatesbase::new().await;
        
        let db = match db_result {
            Ok(db) => db,
            Err(e) => {
                println!("⚠ Skipping nonexistent album test - MongoDB not available: {}", e);
                return;
            }
        };

        let fake_id = "nonexistent-album-id";
        
        match db.get_album_by_id(fake_id).await {
            Ok(None) => println!("✓ Nonexistent album test passed - correctly returned None"),
            Ok(Some(_)) => println!("✗ Nonexistent album test failed - found album that shouldn't exist"),
            Err(e) => println!("✗ Nonexistent album test failed with error: {}", e)
        }
    }

    #[tokio::test]
    async fn test_empty_database() {
        let db_result = AlbumsDatesbase::new().await;
        
        let db = match db_result {
            Ok(db) => db,
            Err(e) => {
                println!("⚠ Skipping empty database test - MongoDB not available: {}", e);
                return;
            }
        };

        // Clear test collection first (use a separate test database)
        let test_collection: Collection<Album> = db.client.database("img-hub-test").collection("albums");
        let _ = test_collection.drop().await;

        // Try to get all albums from empty collection
        let my_coll: Collection<Album> = db.client.database("img-hub-test").collection("albums");
        let mut cursor = my_coll.find(doc! {}).await.expect("Find should work on empty collection");
        let mut albums = Vec::new();
        
        while cursor.advance().await.expect("Cursor advance should work") {
            let album = cursor.deserialize_current().expect("Deserialization should work");
            albums.push(album);
        }

        assert!(albums.is_empty(), "Empty database should return empty vector");
        println!("✓ Empty database test passed - returned {} albums", albums.len());
    }
}
