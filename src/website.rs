use crate::favicon::{fetch_and_parse_favicon, FaviconLocation};
use crate::utils::{decode_image_metadata, save_favicon};
use reqwest::redirect::Policy;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use tracing::{error, info};
use url::Url;

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

async fn fetch_html(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .redirect(Policy::limited(15)) // Add redirect handling
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let url = if url.starts_with("http") {
        url.to_string()
    } else {
        format!("https://{}", url)
    };

    let response = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Version/14.1.2 Safari/537.36",
        )
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch HTML from {}: HTTP {}",
            url,
            response.status()
        )
        .into());
    }

    let html = response
        .text()
        .await
        .map_err(|e| format!("Failed to get response text: {}", e))?;
    Ok(html)
}

pub fn parse_manifest_url(html: &str, base_url: &Url) -> Option<String> {
    let document = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse(r#"link[rel="manifest"]"#).unwrap();

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            return base_url.join(href).ok().map(|url| url.to_string());
        }
    }

    None
}

pub async fn fetch_and_parse_manifest(
    manifest_url: &str,
) -> Result<Vec<FaviconLocation>, ProcessWebsiteError> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| ProcessWebsiteError::SaveError(e.to_string()))?;

    let response = client
        .get(manifest_url)
        .send()
        .await
        .map_err(|e| ProcessWebsiteError::SaveError(e.to_string()))?;

    if !response.status().is_success() {
        return Err(ProcessWebsiteError::SaveError(format!(
            "Failed to fetch manifest from {}: HTTP {}",
            manifest_url,
            response.status()
        )));
    }

    let manifest: Value = response
        .json()
        .await
        .map_err(|e| ProcessWebsiteError::SaveError(e.to_string()))?;
    let mut favicons = Vec::new();

    if let Some(icons) = manifest.get("icons").and_then(|icons| icons.as_array()) {
        for icon in icons {
            if let Some(src) = icon.get("src").and_then(|src| src.as_str()) {
                let mime_type = icon.get("type").and_then(|t| t.as_str()).unwrap_or("");
                favicons.push(FaviconLocation {
                    url: Url::parse(manifest_url)
                        .map_err(|e| ProcessWebsiteError::SaveError(e.to_string()))?
                        .join(src)
                        .map_err(|e| ProcessWebsiteError::SaveError(e.to_string()))?
                        .to_string(),
                    mime_type: crate::mime_type::MimeType::from_str(mime_type),
                });
            }
        }
    }

    Ok(favicons)
}

pub async fn process_website(website: String) -> Result<ProcessWebsiteResult, ProcessWebsiteError> {
    info!("Processing website: {}", website);
    let mut attempted_urls = Vec::new();

    // Step 1: Fetch the HTML
    let html = match fetch_html(&website).await {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to fetch HTML for {}: {}", website, e);
            return Err(ProcessWebsiteError::SaveError(e.to_string()));
        }
    };

    // Step 2: Parse favicons from the HTML
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

    // Step 3: Check the manifest file
    if let Some(manifest_url) = parse_manifest_url(
        &html,
        &Url::parse(&website).map_err(|e| ProcessWebsiteError::SaveError(e.to_string()))?,
    ) {
        info!("Found manifest file: {}", manifest_url);
        if let Ok(manifest_favicons) = fetch_and_parse_manifest(&manifest_url).await {
            for favicon in manifest_favicons {
                attempted_urls.push(favicon.url.clone());
                if let Some(data) = crate::favicon::check_for_favicon(favicon.url.clone()).await {
                    let mime_type = favicon.mime_type.as_str().to_string(); // Convert to owned String
                    return Ok(ProcessWebsiteResult::Success {
                        path: save_favicon(&data, mime_type.clone()) // Pass as reference to owned String
                            .await
                            .map_err(|e| ProcessWebsiteError::SaveError(e.to_string()))?,
                        mime_type,
                        attempted_urls,
                        width: None,
                        height: None,
                        byte_size: data.len(),
                    });
                }
            }
        }
    }

    // Step 4: Fallback to `/favicon.ico`
    let common_favicon_url = format!("{}/favicon.ico", website.trim_end_matches('/'));
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
