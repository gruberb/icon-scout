use std::fs::File;
use std::io::Write;
use std::path::Path;
use tracing::info;

use crate::favicon::fetch_and_parse_favicon;
use crate::utils::sanitize_website_filename;

#[derive(serde::Deserialize)]
pub struct WebsiteList(pub Vec<String>);

async fn save_favicon_to_disk(
    website: &str,
    favicon_data: &[u8],
    extension: &str,
) -> std::io::Result<()> {
    let folder_path = Path::new("favicons");
    if !folder_path.exists() {
        tokio::fs::create_dir_all(&folder_path).await?;
    }

    let filename = format!("{}{}", sanitize_website_filename(website), extension);
    let filepath = folder_path.join(filename);
    let mut file = File::create(filepath)?;
    file.write_all(favicon_data)?;
    Ok(())
}

pub async fn process_website(website: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Fetch and parse the favicon, handling redirects and common locations
    if let Some(favicon_data) = fetch_and_parse_favicon(website).await? {
        let extension = if website.ends_with(".svg") {
            ".svg"
        } else {
            ".png"
        };
        save_favicon_to_disk(website, &favicon_data, extension).await?;
        info!("Favicon saved for {} as {}", website, extension);
    } else {
        info!("No valid favicon found for {}", website);
    }

    Ok(())
}
