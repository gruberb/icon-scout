use serde::Serialize;
use std::path::PathBuf;
use tracing::{error, info};

use crate::favicon::fetch_and_parse_favicon;
use crate::favicon::Favicon;
use crate::mime_type::MimeType;
use crate::utils::save_favicon_to_disk;
use image::io::Reader as ImageReader;

#[derive(Serialize)]
pub enum ProcessWebsiteError {
    FaviconNotFound,
}

#[derive(Serialize)]
pub enum ProcessWebsiteResult {
    Success {
        path: PathBuf,
        mime_type: String,
        attempted_urls: Vec<String>,
        width: Option<u32>,
        height: Option<u32>,
        byte_size: usize,
    },
    Failure {
        attempted_urls: Vec<String>,
    },
}

pub async fn process_website(website: String) -> Result<ProcessWebsiteResult, ProcessWebsiteError> {
    let mut attempted_urls = Vec::new();

    // Try the common favicon location first
    let common_favicon_url = if website.starts_with("http") {
        format!("{}/favicon.ico", website.trim_end_matches('/'))
    } else {
        format!("https://{}/favicon.ico", website.trim_end_matches('/'))
    };
    attempted_urls.push(common_favicon_url.clone());

    if let Some(data) = crate::favicon::check_for_favicon(common_favicon_url.clone()).await {
        info!("Favicon found at common location for {}", website);
        let (width, height) = decode_image_metadata(&data);
        let byte_size = data.len();
        let path = save_favicon_to_disk(&data, ".ico")
            .map_err(|_| ProcessWebsiteError::FaviconNotFound)?;
        return Ok(ProcessWebsiteResult::Success {
            path,
            mime_type: MimeType::ImageXIcon.as_str().to_string(),
            attempted_urls,
            width,
            height,
            byte_size,
        });
    }

    // If not found at common location, attempt to fetch and parse from HTML
    match fetch_and_parse_favicon(website.clone()).await {
        Ok((Favicon { data, mime_type }, mut favicon_attempts)) => {
            // Record all attempts from fetch_and_parse_favicon
            attempted_urls.append(&mut favicon_attempts);

            // Extract metadata
            let (width, height) = decode_image_metadata(&data);
            let byte_size = data.len();
            let extension = match mime_type {
                MimeType::ImagePng => ".png",
                MimeType::ImageSvgXml => ".svg",
                MimeType::ImageXIcon | MimeType::ImageVndMicrosoftIcon => ".ico",
                MimeType::ImageGif => ".gif",
                MimeType::ImageJpeg => ".jpg",
                MimeType::ImageWebp => ".webp",
                MimeType::Unknown(_) => ".bin",
            };

            let path = save_favicon_to_disk(&data, extension)
                .map_err(|_| ProcessWebsiteError::FaviconNotFound)?;
            Ok(ProcessWebsiteResult::Success {
                path,
                mime_type: mime_type.as_str().to_string(),
                attempted_urls,
                width,
                height,
                byte_size,
            })
        }
        Err(e) => {
            error!("No valid favicon found for {}: {:?}", website, e);
            // Failure: return attempted URLs
            Ok(ProcessWebsiteResult::Failure { attempted_urls })
        }
    }
}

// Attempt to decode image metadata: width and height
fn decode_image_metadata(data: &[u8]) -> (Option<u32>, Option<u32>) {
    if let Ok(reader) = ImageReader::new(std::io::Cursor::new(data)).with_guessed_format() {
        if let Ok(img) = reader.decode() {
            return (Some(img.width()), Some(img.height()));
        }
    }
    (None, None)
}
