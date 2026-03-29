mod models;
mod database;
mod response;
mod pic;
mod routes;

#[macro_use] extern crate rocket;
use log::info;
use rocket::fs::FileServer;
use rocket_cors::{AllowedOrigins, CorsOptions};
use database::AlbumsDatabase;

#[derive(Clone)]
pub struct AppConfig {
    pub upload_dir: String,
}

#[launch]
async fn rocket() -> _ {
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Stdout)
        .filter_level(log::LevelFilter::Info)
        .init();
    info!("Starting img-hub-backend server");

    let database_name = std::env::var("DATABASE_NAME").unwrap_or_else(|_| "img-hub".to_string());

    let db = match AlbumsDatabase::new(database_name).await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to initialize database: {}", e);
            std::process::exit(1);
        }
    };

    let config = AppConfig {
        upload_dir: std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "static/".to_string()),
    };

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .unwrap_or(8000);

    let figment = rocket::Config::figment()
        .merge(("address", host))
        .merge(("port", port));

    let cors = CorsOptions {
        allowed_origins: AllowedOrigins::all(),
        allow_credentials: true,
        ..Default::default()
    }.to_cors().expect("Failed to create CORS fairing");

    let static_dir = config.upload_dir.clone();

    rocket::custom(figment)
        .attach(cors)
        .manage(db)
        .manage(config)
        .mount(
            "/",
            routes![
                routes::albums::index,
                routes::albums::get_albums,
                routes::albums::get_all_albums_admin,
                routes::albums::get_featured_albums,
                routes::albums::get_album_by_id,
                routes::albums::set_album_cover,
                routes::albums::delete_album,
                routes::upload::upload_images_async,
                routes::upload::get_upload_status,
            ]
        )
        .mount("/public", FileServer::from(static_dir))
}
