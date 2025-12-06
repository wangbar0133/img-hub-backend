use image::{ImageFormat, ImageResult, DynamicImage, imageops::FilterType};
use log::{info, error};
use std::path::Path;
use std::fs::File;
use std::io::BufWriter;
extern crate exif;
use exif::{Exif, Tag, In, Value};
use chrono::{DateTime, Utc, NaiveDateTime};

use serde::{Serialize, Deserialize};

use crate::albums::Photo;

#[derive(Debug, Clone)]
pub struct CompressionConfig {
    pub max_width: u32,
    pub max_height: u32,
    pub quality: u8,
    pub format: ImageFormat,
}

impl CompressionConfig {

    fn detail() -> Self {
        Self {
            max_width: 1920,
            max_height: 1080,
            quality: 100,
            format: ImageFormat::Jpeg,
        }
    }

    pub fn medium() -> Self {
        Self {
            max_width: 800,
            max_height: 600,
            quality: 100,
            format: ImageFormat::Jpeg,
        }
    }

    pub fn thumbnail() -> Self {
        Self {
            max_width: 300,
            max_height: 300,
            quality: 100,
            format: ImageFormat::Jpeg,
        }
    }
}

pub struct ImageCompressor;

impl ImageCompressor {
    pub async fn compress_image<P: AsRef<Path> + Send + 'static>(
        input_path: P,
        output_path: P,
        config: CompressionConfig,
    ) -> ImageResult<()> {
        let input_path_clone = input_path.as_ref().to_path_buf();
        let output_path_clone = output_path.as_ref().to_path_buf();
        
        info!("Starting compression: {:?} -> {:?}", 
              input_path_clone.display(), 
              output_path_clone.display());

        // Run CPU-intensive image processing in a separate thread
        tokio::task::spawn_blocking(move || {
            let img = image::open(&input_path_clone)?;
            info!("Original size: {}x{}", img.width(), img.height());

            let (new_width, new_height) = Self::calculate_new_dimensions(
                img.width(),
                img.height(),
                config.max_width,
                config.max_height,
            );

            let resized_img = if new_width != img.width() || new_height != img.height() {
                img.resize(new_width, new_height, FilterType::Lanczos3)
            } else {
                img
            };

            Self::save_with_quality(&resized_img, &output_path_clone, &config)?;
            info!("Compression completed");
            Ok::<(), image::ImageError>(())
        }).await.map_err(|e| image::ImageError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?
    }

    pub async fn generate_multiple_sizes<P: AsRef<Path> + Send + 'static>(
        input_path: P,
        output_dir: P,
        base_name: &str,
    ) -> Result<Photo, Box<dyn std::error::Error + Send + Sync>> {
        let output_path = output_dir.as_ref();
        tokio::fs::create_dir_all(output_path).await?;

        // 使用模式匹配处理 basename，截去图片扩展名
        let clean_base_name = match base_name.to_lowercase().as_str() {
            name if name.ends_with(".jpg") => &base_name[..base_name.len() - 4],
            _ => base_name,
        };

        let input_path_buf = input_path.as_ref().to_path_buf();
        
        // 并行处理：同时进行图片压缩和EXIF信息提取
        let compression_task = Self::generate_sizes_parallel(input_path_buf.clone(), output_path.to_path_buf(), clean_base_name);
        let info_task = Self::get_image_info(input_path_buf.clone());
        
        let (compression_result, img_info) = tokio::try_join!(compression_task, info_task)?;
        
        if compression_result {
            Ok(Photo::new(clean_base_name, img_info))
        } else {
            Err("Failed to generate compressed images".into())
        }
    }

    async fn generate_sizes_parallel(
        input_path: std::path::PathBuf,
        output_path: std::path::PathBuf,
        clean_base_name: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let configs = vec![
            ("detail", CompressionConfig::detail()),
            ("thumbnail", CompressionConfig::thumbnail()),
            ("medium", CompressionConfig::medium()),
        ];

        // 并行生成所有尺寸
        let tasks: Vec<_> = configs.into_iter().map(|(size_name, config)| {
            let input_path = input_path.clone();
            let output_file = output_path.join(format!("{}_{}.jpg", clean_base_name, size_name));
            let size_name = size_name.to_string();
            
            tokio::spawn(async move {
                match Self::compress_image(input_path, output_file, config).await {
                    Ok(_) => {
                        info!("Generated {} size", size_name);
                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to generate {} size: {}", size_name, e);
                        Err(e)
                    }
                }
            })
        }).collect();

        // 等待所有任务完成
        let mut success_count = 0;
        for task in tasks {
            match task.await {
                Ok(Ok(_)) => success_count += 1,
                Ok(Err(e)) => error!("Compression task failed: {}", e),
                Err(e) => error!("Task join failed: {}", e),
            }
        }

        Ok(success_count == 3) // 所有3个尺寸都成功
    }

    fn calculate_new_dimensions(
        original_width: u32,
        original_height: u32,
        max_width: u32,
        max_height: u32,
    ) -> (u32, u32) {
        if original_width <= max_width && original_height <= max_height {
            return (original_width, original_height);
        }

        let width_ratio = max_width as f32 / original_width as f32;
        let height_ratio = max_height as f32 / original_height as f32;
        let ratio = width_ratio.min(height_ratio);

        let new_width = (original_width as f32 * ratio) as u32;
        let new_height = (original_height as f32 * ratio) as u32;

        (new_width, new_height)
    }

    fn save_with_quality<P: AsRef<Path>>(
        img: &DynamicImage,
        output_path: P,
        config: &CompressionConfig,
    ) -> ImageResult<()> {
        let output_path = output_path.as_ref();
        
        match config.format {
            ImageFormat::Jpeg => {
                let file = File::create(output_path)?;
                let mut writer = BufWriter::new(file);
                
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                    &mut writer,
                    config.quality,
                );
                
                encoder.encode_image(img)?;
            }
            _ => {
                img.save(output_path)?;
            }
        }

        Ok(())
    }

    pub async fn get_image_info<P: AsRef<Path> + Send + 'static>(
        path: P,
    ) -> Result<ImageInfo, Box<dyn std::error::Error + Send + Sync>> {
        let path_buf = path.as_ref().to_path_buf();
        
        // Run I/O intensive operations in a blocking task
        tokio::task::spawn_blocking(move || -> Result<ImageInfo, Box<dyn std::error::Error + Send + Sync>> {
            let file_size = std::fs::metadata(&path_buf)?.len();

            // 提取 EXIF 数据
            let exif_data = Self::extract_exif_data(&path_buf);
            
            // 快速获取图片尺寸，不完整解码
            let (width, height, format) = Self::get_image_dimensions_fast(&path_buf)?;

            Ok(ImageInfo {
                width,
                height,
                format,
                file_size,
                created_at: exif_data.created_at,
                camera_make: exif_data.camera_make,
                camera_model: exif_data.camera_model,
                lens_model: exif_data.lens_model,
                focal_length: exif_data.focal_length,
                aperture: exif_data.aperture,
                exposure_time: exif_data.exposure_time,
                iso: exif_data.iso,
                flash: exif_data.flash,
                white_balance: exif_data.white_balance,
            })
        }).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
    }

    /// 提取 EXIF 数据
    fn extract_exif_data<P: AsRef<Path>>(path: P) -> ExifData {
        let mut exif_data = ExifData::default();

        // 尝试读取 EXIF 数据
        if let Ok(file) = std::fs::File::open(path) {
            let mut buf_reader = std::io::BufReader::new(file);
            let exif_reader = exif::Reader::new();
            if let Ok(exif) = exif_reader.read_from_container(&mut buf_reader) {
                exif_data = Self::parse_exif(&exif);
            }
        }

        exif_data
    }
    
    /// 快速获取图片尺寸，不完整解码图片
    fn get_image_dimensions_fast<P: AsRef<Path>>(path: P) -> Result<(u32, u32, String), Box<dyn std::error::Error + Send + Sync>> {
        // 首先尝试从文件扩展名快速判断格式
        let format = match path.as_ref().extension().and_then(|ext| ext.to_str()) {
            Some("jpg") | Some("jpeg") => "JPEG".to_string(),
            Some("png") => "PNG".to_string(),
            Some("webp") => "WEBP".to_string(),
            Some("gif") => "GIF".to_string(),
            _ => "JPEG".to_string(), // 默认假设是JPEG
        };
        
        // 使用image::io::Reader来快速读取尺寸
        let file = std::fs::File::open(&path)?;
        let reader = std::io::BufReader::new(file);
        
        if let Ok(reader) = image::io::Reader::new(reader).with_guessed_format() {
            if let Ok((width, height)) = reader.into_dimensions() {
                return Ok((width, height, format));
            }
        }
        
        // 回退：尝试使用旧方法
        match image::image_dimensions(path.as_ref()) {
            Ok((width, height)) => Ok((width, height, format)),
            Err(_) => {
                // 最后回退到完整解码 (慢)
                println!("Warning: Falling back to full image decode for dimensions");
                let img = image::open(path)?;
                Ok((img.width(), img.height(), format!("{:?}", img.color())))
            }
        }
    }

    /// 解析 EXIF 数据
    fn parse_exif(exif: &Exif) -> ExifData {
        let mut data = ExifData::default();

        // 拍摄时间
        if let Some(field) = exif.get_field(Tag::DateTime, In::PRIMARY) {
            data.created_at = Self::parse_datetime(&field.display_value().to_string());
        } else if let Some(field) = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
            data.created_at = Self::parse_datetime(&field.display_value().to_string());
        }

        // 相机制造商
        if let Some(field) = exif.get_field(Tag::Make, In::PRIMARY) {
            data.camera_make = Some(field.display_value().to_string());
        }

        // 相机型号
        if let Some(field) = exif.get_field(Tag::Model, In::PRIMARY) {
            data.camera_model = Some(field.display_value().to_string());
        }

        // 镜头型号
        if let Some(field) = exif.get_field(Tag::LensModel, In::PRIMARY) {
            data.lens_model = Some(field.display_value().to_string());
        }

        // 焦距
        if let Some(field) = exif.get_field(Tag::FocalLength, In::PRIMARY) {
            if let Value::Rational(ref rationals) = field.value {
                if !rationals.is_empty() {
                    let rational = &rationals[0];
                    data.focal_length = Some(rational.num as f64 / rational.denom as f64);
                }
            }
        }

        // 光圈
        if let Some(field) = exif.get_field(Tag::FNumber, In::PRIMARY) {
            if let Value::Rational(ref rationals) = field.value {
                if !rationals.is_empty() {
                    let rational = &rationals[0];
                    data.aperture = Some(rational.num as f64 / rational.denom as f64);
                }
            }
        }

        // 曝光时间
        if let Some(field) = exif.get_field(Tag::ExposureTime, In::PRIMARY) {
            data.exposure_time = Some(field.display_value().to_string());
        }

        // ISO
        if let Some(field) = exif.get_field(Tag::PhotographicSensitivity, In::PRIMARY) {
            if let Value::Short(ref values) = field.value {
                if !values.is_empty() {
                    data.iso = Some(values[0] as u32);
                }
            }
        }

        // 闪光灯
        if let Some(field) = exif.get_field(Tag::Flash, In::PRIMARY) {
            data.flash = Some(field.display_value().to_string());
        }

        // 白平衡
        if let Some(field) = exif.get_field(Tag::WhiteBalance, In::PRIMARY) {
            data.white_balance = Some(field.display_value().to_string());
        }

        data
    }

    /// 解析日期时间
    fn parse_datetime(datetime_str: &str) -> Option<DateTime<Utc>> {
        // EXIF 日期时间格式: "YYYY:MM:DD HH:MM:SS"
        let trimmed = datetime_str.trim();

        // 直接解析EXIF格式
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(trimmed, "%Y:%m:%d %H:%M:%S") {
            return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
        }

        // 如果失败，尝试其他常见格式
        let formats = [
            "%Y-%m-%d %H:%M:%S",
            "%Y/%m/%d %H:%M:%S",
            "%Y:%m:%d %H:%M:%S",
        ];

        for format in &formats {
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(trimmed, format) {
                return Some(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
            }
        }

        None
    }

}

#[derive(Debug, Default)]
struct ExifData {
    created_at: Option<DateTime<Utc>>,
    camera_make: Option<String>,
    camera_model: Option<String>,
    lens_model: Option<String>,
    focal_length: Option<f64>,
    aperture: Option<f64>,
    exposure_time: Option<String>,
    iso: Option<u32>,
    flash: Option<String>,
    white_balance: Option<String>,
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use image::{ImageBuffer, Rgb};

    fn create_test_image(width: u32, height: u32, path: &str) -> PathBuf {
        // 创建一个简单的测试图片
        let img = ImageBuffer::from_fn(width, height, |x, y| {
            if (x + y) % 2 == 0 {
                Rgb([255u8, 0u8, 0u8]) // 红色
            } else {
                Rgb([0u8, 255u8, 0u8]) // 绿色
            }
        });
        
        let path_buf = PathBuf::from(path);
        if let Some(parent) = path_buf.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        
        img.save(&path_buf).unwrap();
        path_buf
    }

    fn cleanup_test_files(paths: &[&str]) {
        for path in paths {
            let _ = fs::remove_file(path);
        }
        // 清理测试目录
        let _ = fs::remove_dir_all("test_images");
    }

    #[test]
    fn test_compression_config_presets() {
        let thumbnail = CompressionConfig::thumbnail();
        assert_eq!(thumbnail.max_width, 300);
        assert_eq!(thumbnail.max_height, 300);
        assert_eq!(thumbnail.quality, 100);

        let medium = CompressionConfig::medium();
        assert_eq!(medium.max_width, 800);
        assert_eq!(medium.max_height, 600);
        assert_eq!(medium.quality, 100);
    }

    #[test]
    fn test_calculate_new_dimensions_no_resize_needed() {
        let (w, h) = ImageCompressor::calculate_new_dimensions(800, 600, 1920, 1080);
        assert_eq!((w, h), (800, 600));
    }

    #[test]
    fn test_calculate_new_dimensions_width_limited() {
        let (w, h) = ImageCompressor::calculate_new_dimensions(2000, 1000, 1000, 1200);
        assert_eq!((w, h), (1000, 500));
    }

    #[test]
    fn test_calculate_new_dimensions_height_limited() {
        let (w, h) = ImageCompressor::calculate_new_dimensions(1000, 2000, 1200, 1000);
        assert_eq!((w, h), (500, 1000));
    }

    #[test]
    fn test_calculate_new_dimensions_both_limited() {
        let (w, h) = ImageCompressor::calculate_new_dimensions(1920, 1080, 960, 540);
        assert_eq!((w, h), (960, 540));
    }

    #[tokio::test]
    async fn test_compress_image() {
        let input_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/test.jpg");
        let output_path = "test_images/test_output.jpg";
        
        // 检查测试文件是否存在
        if !input_path.exists() {
            println!("跳过测试：测试文件 {} 不存在", input_path.display());
            return;
        }
        
        // 创建输出目录
        if let Some(parent) = PathBuf::from(output_path).parent() {
            std::fs::create_dir_all(parent).expect("Failed to create output directory");
        }
        
        let config = CompressionConfig::thumbnail();
        let result = ImageCompressor::compress_image(input_path, PathBuf::from(output_path), config).await;
        
        assert!(result.is_ok(), "压缩应该成功: {:?}", result.err());
        assert!(PathBuf::from(output_path).exists(), "输出文件应该存在");
        
        // 验证输出图片尺寸
        if let Ok(info) = ImageCompressor::get_image_info(output_path).await {
            assert!(info.width <= 300, "宽度应该不超过300, 实际: {}", info.width);
            assert!(info.height <= 300, "高度应该不超过300, 实际: {}", info.height);
        }
        
        cleanup_test_files(&[output_path]);
    }

    #[tokio::test]
    async fn test_generate_multiple_sizes() {
        let input_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/test.jpg");
        let output_dir = "test_images/output";
        let base_name = "test";
        
        // 检查测试文件是否存在
        if !input_path.exists() {
            println!("跳过测试：测试文件 {} 不存在", input_path.display());
            return;
        }
        
        let photo = ImageCompressor::generate_multiple_sizes(input_path, PathBuf::from(output_dir), base_name).await;
        
        assert!(photo.is_ok(), "生成多尺寸应该成功: {:?}", photo.as_ref().err());
        
        let _photo = photo.unwrap();
        
        let detail_path = format!("{}/test_detail.jpg", output_dir);
        let medium_path = format!("{}/test_medium.jpg", output_dir);
        let thumbnail_path = format!("{}/test_thumbnail.jpg", output_dir);
        
        assert!(PathBuf::from(&detail_path).exists(), "生成的文件应该存在: {}", detail_path);
        assert!(PathBuf::from(&medium_path).exists(), "生成的文件应该存在: {}", medium_path);
        assert!(PathBuf::from(&thumbnail_path).exists(), "生成的文件应该存在: {}", thumbnail_path);
        
        
        // 验证缩略图尺寸
        let thumbnail_path = format!("{}/test_thumbnail.jpg", output_dir);
        if PathBuf::from(&thumbnail_path).exists() {
            if let Ok(info) = ImageCompressor::get_image_info(thumbnail_path.clone()).await {
                assert!(info.width <= 300, "缩略图宽度应该不超过300");
                assert!(info.height <= 300, "缩略图高度应该不超过300");
            }
        }
        
        // 验证中等尺寸
        let medium_path = format!("{}/test_medium.jpg", output_dir);
        if PathBuf::from(&medium_path).exists() {
            if let Ok(info) = ImageCompressor::get_image_info(medium_path.clone()).await {
                assert!(info.width <= 800, "中等尺寸宽度应该不超过800");
                assert!(info.height <= 600, "中等尺寸高度应该不超过600");
            }
        }
        
        let _ = fs::remove_dir_all(output_dir);
    }

    #[tokio::test]
    async fn test_get_image_info() {
        let test_path = "test_data/test.jpg";
        
        // 检查测试文件是否存在
        if !PathBuf::from(test_path).exists() {
            println!("跳过测试：测试文件 {} 不存在", test_path);
            return;
        }
        
        let result = ImageCompressor::get_image_info(test_path).await;
        assert!(result.is_ok(), "获取图片信息应该成功: {:?}", result.as_ref().err());
        
        let info = result.unwrap();
        println!("=== 详细图片信息 ===");
        println!("尺寸: {}x{} pixels", info.width, info.height);
        println!("文件大小: {} bytes", info.file_size);
        println!("格式: {}", info.format);
        
        if let Some(created_at) = &info.created_at {
            println!("拍摄时间: {}", created_at);
        }
        
        if let Some(camera_make) = &info.camera_make {
            println!("相机制造商: {}", camera_make);
        }
        
        if let Some(camera_model) = &info.camera_model {
            println!("相机型号: {}", camera_model);
        }
        
        if let Some(lens_model) = &info.lens_model {
            println!("镜头型号: {}", lens_model);
        }
        
        if let Some(focal_length) = info.focal_length {
            println!("焦距: {}mm", focal_length);
        }
        
        if let Some(aperture) = info.aperture {
            println!("光圈: f/{:.1}", aperture);
        }
        
        if let Some(exposure_time) = &info.exposure_time {
            println!("曝光时间: {}", exposure_time);
        }
        
        if let Some(iso) = info.iso {
            println!("ISO: {}", iso);
        }
        
        if let Some(flash) = &info.flash {
            println!("闪光灯: {}", flash);
        }
        
        if let Some(white_balance) = &info.white_balance {
            println!("白平衡: {}", white_balance);
        }
        
        println!("===================");
        
        assert!(info.width > 0);
        assert!(info.height > 0);
        assert!(info.file_size > 0);
        assert!(!info.format.is_empty());
    }

    #[tokio::test]
    async fn test_compress_image_no_resize_needed() {
        let input_path = "test_images/small_input.jpg";
        let output_path = "test_images/small_output.jpg";
        
        // 创建一个已经很小的图片
        create_test_image(200, 150, input_path);
        
        let config = CompressionConfig::thumbnail(); // 300x300 限制
        let result = ImageCompressor::compress_image(input_path, output_path, config).await;
        
        assert!(result.is_ok(), "压缩应该成功");
        
        // 验证尺寸没有变化（因为原图已经小于限制）
        if let Ok(info) = ImageCompressor::get_image_info(output_path).await {
            assert_eq!(info.width, 200);
            assert_eq!(info.height, 150);
        }
        
        cleanup_test_files(&[input_path, output_path]);
    }

    #[test]
    fn test_aspect_ratio_preservation() {
        let test_cases = vec![
            (1920, 1080, 960, 960), // 16:9 -> 应该变成 960x540
            (1080, 1920, 960, 960), // 9:16 -> 应该变成 540x960
            (1000, 1000, 500, 800), // 1:1 -> 应该变成 500x500
        ];

        for (i, (orig_w, orig_h, max_w, max_h)) in test_cases.iter().enumerate() {
            let (new_w, new_h) = ImageCompressor::calculate_new_dimensions(
                *orig_w, *orig_h, *max_w, *max_h
            );
            
            // 验证宽高比保持不变（允许小幅误差）
            let original_ratio = *orig_w as f32 / *orig_h as f32;
            let new_ratio = new_w as f32 / new_h as f32;
            let ratio_diff = (original_ratio - new_ratio).abs();
            
            assert!(
                ratio_diff < 0.01,
                "测试案例 {}: 宽高比应该保持不变。原始: {:.3}, 新: {:.3}",
                i, original_ratio, new_ratio
            );
            
            // 验证不超过限制
            assert!(new_w <= *max_w, "测试案例 {}: 新宽度不应超过限制", i);
            assert!(new_h <= *max_h, "测试案例 {}: 新高度不应超过限制", i);
        }
    }
}