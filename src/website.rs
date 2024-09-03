use reqwest::Client;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use url::Url;
use tracing::info;

use crate::favicon::{parse_favicon_url, check_for_favicon};
use crate::utils::sanitize_website_filename;

#[derive(serde::Deserialize)]
pub struct WebsiteList(pub Vec<String>);

async fn save_favicon_to_disk(website: &str, favicon_data: &[u8], extension: &str) -> std::io::Result<()> {
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
    let base_url = Url::parse(website)?;

    // Fetch the HTML content of the website
    let client = Client::new();
    let response = client.get(website).send().await?;
    let html = response.text().await?;

    // Parse the favicon URL from the HTML
    if let Some(favicon_url) = parse_favicon_url(&html, base_url) {
        if let Some(favicon_data) = check_for_favicon(favicon_url.clone()).await {
            let extension = if favicon_url.ends_with(".svg") { ".svg" } else { ".png" };
            save_favicon_to_disk(website, &favicon_data, extension).await?;
            info!("Favicon saved for {} as {}", website, extension);
        } else {
            info!("No valid favicon found for {}", website);
        }
    } else {
        info!("No favicon URL found for {}", website);
    }

    Ok(())
}
