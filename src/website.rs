use axum::debug_handler;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tracing::info;

use crate::favicon::fetch_and_parse_favicon;
use crate::utils::sanitize_website_filename;

#[derive(serde::Deserialize)]
pub struct WebsiteList(pub Vec<String>);

fn save_favicon_to_disk(
    website: &str,
    favicon_data: &[u8],
    extension: &str,
) -> std::io::Result<()> {
    let folder_path = Path::new("favicons");
    if !folder_path.exists() {
        std::fs::create_dir_all(&folder_path)?;
    }

    let filename = format!("{}{}", sanitize_website_filename(website), extension);
    let filepath = folder_path.join(filename);
    let mut file = File::create(filepath)?;
    file.write_all(favicon_data)?;
    Ok(())
}

#[debug_handler]
pub async fn process_website(website: String) -> impl IntoResponse {
    // Fetch and parse the favicon, handling redirects and common locations
    match fetch_and_parse_favicon(website.clone()).await {
        Ok(favicon_data) => {
            let extension = if website.ends_with(".svg") {
                ".svg"
            } else {
                ".png"
            };
            if let Err(_) = save_favicon_to_disk(&website, &favicon_data, extension) {
                return (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong").into_response();
            }
        }
        Err(_) => {
            info!("No valid favicon found for {}", website);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong").into_response();
        }
    }

    (StatusCode::CREATED, "Favicon saved to disk").into_response()
}
