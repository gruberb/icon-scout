use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use tracing::{error, info};

use crate::favicon::fetch_and_parse_favicon;
use crate::favicon::Favicon;
use crate::utils::sanitize_website_filename;

#[derive(Serialize)]
pub enum ProcessWebsiteError {
    FaviconNotFound,
}

#[allow(dead_code)]
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

pub async fn process_website(website: String) -> Result<Favicon, ProcessWebsiteError> {
    // Fetch and parse the favicon, handling redirects and common locations
    match fetch_and_parse_favicon(website.clone()).await {
        Ok(favicon) => {
            info!("Favicon found for {website}");
            Ok(favicon)
        }
        Err(e) => {
            error!("No valid favicon found for {website}: {e:?}");
            Err(ProcessWebsiteError::FaviconNotFound)
        }
    }
}
