use crate::favicon::fetch_and_parse_favicon;
use crate::utils::{decode_image_metadata, save_favicon};
use serde::Serialize;
use tracing::{error, info};

#[derive(Serialize)]
pub enum ProcessWebsiteError {
    SaveError(String),
}

#[derive(Serialize)]
pub enum ProcessWebsiteResult {
    Success {
        path: String,
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
    info!("Processing website: {}", website);
    let mut attempted_urls = Vec::new();

    // Step 1: Try to fetch and parse favicons from the HTML
    match fetch_and_parse_favicon(website.clone()).await {
        Ok((favicon, mut favicon_attempts)) => {
            attempted_urls.append(&mut favicon_attempts);
            let mime_type = favicon.mime_type.as_str().to_string();
            let (width, height) = decode_image_metadata(&favicon.data);
            let byte_size = favicon.data.len();

            let path = save_favicon(&favicon.data, mime_type.clone())
                .await
                .map_err(|e| ProcessWebsiteError::SaveError(e.to_string()))?;

            return Ok(ProcessWebsiteResult::Success {
                path,
                mime_type,
                attempted_urls,
                width,
                height,
                byte_size,
            });
        }
        Err(e) => {
            error!("Failed to fetch favicon from HTML for {}: {:?}", website, e);
        }
    }

    // Step 2: Fallback to the common `/favicon.ico` location
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

        let path = save_favicon(&data, "image/x-icon")
            .await
            .map_err(|e| ProcessWebsiteError::SaveError(e.to_string()))?;

        return Ok(ProcessWebsiteResult::Success {
            path,
            mime_type: "image/x-icon".to_string(),
            attempted_urls,
            width,
            height,
            byte_size,
        });
    }

    // No favicon found
    Ok(ProcessWebsiteResult::Failure { attempted_urls })
}
