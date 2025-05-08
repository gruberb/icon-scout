use crate::favicon::{check_for_favicon, fetch_and_parse_favicon, FaviconLocation};
use crate::utils::{decode_image_metadata, save_favicon};
use reqwest::redirect::Policy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};
use url::Url;

#[derive(Debug, Serialize)]
pub enum ProcessWebsiteError {
    SaveError(String),
    NetworkError(String),
    ParseError(String),
    HttpError { status: u16, url: String },
    ForbiddenAccess(String),
    TooManyRequests(String),
    MalformedUrl { url: String, error: String },
    ManifestParsingError { url: String, error: String },
    InvalidImageData { url: String },
    Timeout(String),
    EmptyResponse(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FaviconAttemptResult {
    pub url: String,
    pub success: bool,
    pub error: Option<String>,
    pub timestamp: String,
}

#[derive(Serialize)]
pub enum ProcessWebsiteResult {
    Success {
        path: String,
        mime_type: String,
        attempted_urls: Vec<FaviconAttemptResult>,
        width: Option<u32>,
        height: Option<u32>,
        byte_size: usize,
    },
    Failure {
        attempted_urls: Vec<FaviconAttemptResult>,
        reasons: Vec<String>,
    },
}

async fn fetch_html(url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::builder()
        .redirect(Policy::limited(15))
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
        .build()
        .map_err(|e| ProcessWebsiteError::NetworkError(e.to_string()))?; // Use NetworkError

    let response = client
        .get(manifest_url)
        .send()
        .await
        .map_err(|e| ProcessWebsiteError::NetworkError(e.to_string()))?; // Use NetworkError

    if !response.status().is_success() {
        let status = response.status().as_u16();
        return match status {
            403 => Err(ProcessWebsiteError::ForbiddenAccess(
                manifest_url.to_string(),
            )), // Add this variant
            429 => Err(ProcessWebsiteError::TooManyRequests(
                manifest_url.to_string(),
            )), // Add this variant
            _ => Err(ProcessWebsiteError::HttpError {
                // Add this variant
                status,
                url: manifest_url.to_string(),
            }),
        };
    }

    let manifest: Value = response.json().await.map_err(|e| {
        ProcessWebsiteError::ParseError(format!("Failed to parse manifest JSON: {}", e))
    })?;

    let mut favicons = Vec::new();

    if let Some(icons) = manifest.get("icons").and_then(|icons| icons.as_array()) {
        for icon in icons {
            if let Some(src) = icon.get("src").and_then(|src| src.as_str()) {
                let mime_type = icon.get("type").and_then(|t| t.as_str()).unwrap_or("");

                match Url::parse(manifest_url) {
                    Ok(base_url) => {
                        match base_url.join(src) {
                            Ok(full_url) => {
                                favicons.push(FaviconLocation {
                                    url: full_url.to_string(),
                                    mime_type: crate::mime_type::MimeType::from_str(mime_type),
                                });
                            }
                            Err(e) => {
                                return Err(ProcessWebsiteError::MalformedUrl {
                                    // Use MalformedUrl variant
                                    url: src.to_string(),
                                    error: e.to_string(),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        return Err(ProcessWebsiteError::MalformedUrl {
                            // Use MalformedUrl variant
                            url: manifest_url.to_string(),
                            error: e.to_string(),
                        });
                    }
                }
            }
        }
    } else {
        return Err(ProcessWebsiteError::ManifestParsingError {
            // Use ManifestParsingError variant
            url: manifest_url.to_string(),
            error: "No icons found in manifest".to_string(),
        });
    }

    if favicons.is_empty() {
        return Err(ProcessWebsiteError::ManifestParsingError {
            // Use ManifestParsingError variant
            url: manifest_url.to_string(),
            error: "Manifest contained no valid icons".to_string(),
        });
    }

    Ok(favicons)
}

pub async fn process_website(website: String) -> Result<ProcessWebsiteResult, ProcessWebsiteError> {
    info!("Processing website: {}", website);
    let mut attempted_urls = Vec::new();
    let mut error_reasons = Vec::new();

    // Step 1: Parse favicons from the HTML
    match fetch_and_parse_favicon(website.clone()).await {
        Ok((favicon, favicon_attempts)) => {
            // Convert attempts to our serializable format
            for attempt in favicon_attempts {
                attempted_urls.push(FaviconAttemptResult {
                    url: attempt.url,
                    success: attempt.result.is_ok(),
                    error: match attempt.result {
                        Ok(_) => None,
                        Err(e) => Some(e.to_string()),
                    },
                    timestamp: format!("{:?}", attempt.timestamp),
                });
            }

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
            error_reasons.push(format!("HTML favicon error: {}", e));

            // If we have HTML but couldn't find a favicon, try to get the HTML content
            if let Ok(html) = fetch_html(&website).await {
                // Step 2: Check the manifest file
                if let Some(manifest_url) = parse_manifest_url(
                    &html,
                    &Url::parse(&website)
                        .map_err(|e| ProcessWebsiteError::ParseError(e.to_string()))?,
                ) {
                    info!("Found manifest file: {}", manifest_url);

                    match fetch_and_parse_manifest(&manifest_url).await {
                        Ok(manifest_favicons) => {
                            for favicon in manifest_favicons {
                                match check_for_favicon(favicon.url.clone()).await {
                                    Ok(data) => {
                                        attempted_urls.push(FaviconAttemptResult {
                                            url: favicon.url.clone(),
                                            success: true,
                                            error: None,
                                            timestamp: format!(
                                                "{:?}",
                                                std::time::SystemTime::now()
                                            ),
                                        });

                                        let mime_type = favicon.mime_type.as_str().to_string();
                                        return Ok(ProcessWebsiteResult::Success {
                                            path: save_favicon(&data, mime_type.clone())
                                                .await
                                                .map_err(|e| {
                                                    ProcessWebsiteError::SaveError(e.to_string())
                                                })?,
                                            mime_type,
                                            attempted_urls,
                                            width: None,
                                            height: None,
                                            byte_size: data.len(),
                                        });
                                    }
                                    Err(error) => {
                                        attempted_urls.push(FaviconAttemptResult {
                                            url: favicon.url.clone(),
                                            success: false,
                                            error: Some(error.to_string()),
                                            timestamp: format!(
                                                "{:?}",
                                                std::time::SystemTime::now()
                                            ),
                                        });
                                        error_reasons
                                            .push(format!("Manifest favicon error: {:#?}", error));
                                    }
                                }
                            }
                        }
                        Err(manifest_error) => {
                            error_reasons
                                .push(format!("Manifest parsing error: {:#?}", manifest_error));
                        }
                    }
                } else {
                    error_reasons.push("No manifest found in HTML".to_string());
                }
            } else {
                error_reasons.push("Failed to fetch HTML content".to_string());
            }
        }
    }

    // Step 3: Fallback to `/favicon.ico`
    let common_favicon_url = format!("{}/favicon.ico", website.trim_end_matches('/'));
    match check_for_favicon(common_favicon_url.clone()).await {
        Ok(data) => {
            info!("Favicon found at common location for {}", website);
            let (width, height) = decode_image_metadata(&data);
            let byte_size = data.len();

            attempted_urls.push(FaviconAttemptResult {
                url: common_favicon_url,
                success: true,
                error: None,
                timestamp: format!("{:?}", std::time::SystemTime::now()),
            });

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
        Err(error) => {
            attempted_urls.push(FaviconAttemptResult {
                url: common_favicon_url,
                success: false,
                error: Some(error.to_string()),
                timestamp: format!("{:?}", std::time::SystemTime::now()),
            });
            error_reasons.push(format!("Common favicon.ico error: {}", error));
        }
    }

    // No favicon found, return all failed attempts with reasons
    Ok(ProcessWebsiteResult::Failure {
        attempted_urls,
        reasons: error_reasons,
    })
}
