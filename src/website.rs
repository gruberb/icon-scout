use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use tracing::info;

use crate::favicon::fetch_and_parse_favicon;
use crate::utils::sanitize_website_filename;

#[derive(Serialize)]
pub enum FaviconError {
    NotFound,
    CannotSave,
}

fn save_favicon_to_disk(
    website: &str,
    favicon_data: &[u8],
    extension: &str,
) -> Result<PathBuf, std::io::Error> {
    let folder_path = Path::new("favicons");
    if !folder_path.exists() {
        std::fs::create_dir_all(folder_path)?;
    }

    let filename = format!("{}{}", sanitize_website_filename(website), extension);
    let filepath = folder_path.join(filename);
    let mut file = File::create(filepath.clone())?;
    file.write_all(favicon_data)?;
    Ok(filepath)
}

pub async fn process_website(website: String) -> Result<String, FaviconError> {
    // Fetch and parse the favicon, handling redirects and common locations
    match fetch_and_parse_favicon(website.clone()).await {
        Ok(favicon_data) => {
            let extension = if website.ends_with(".svg") {
                ".svg"
            } else {
                ".png"
            };
            match save_favicon_to_disk(&website, &favicon_data, extension) {
                Ok(path) => Ok(path.to_str().unwrap().to_string()),
                Err(_) => Err(FaviconError::CannotSave),
            }
        }
        Err(_) => {
            info!("No valid favicon found for {}", website);
            Err(FaviconError::NotFound)
        }
    }
}
