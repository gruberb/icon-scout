use crate::mime_type::{self, MimeType};
use crate::website::ProcessWebsiteError;

impl From<ProcessWebsiteError> for ParseFaviconError {
    fn from(error: ProcessWebsiteError) -> Self {
        match error {
            ProcessWebsiteError::MalformedUrl { url, error } => ParseFaviconError::MalformedUrl { url, error },
            _ => ParseFaviconError::Other(format!("{:?}", error)),
        }
    }
}
use reqwest::redirect::Policy;
use reqwest::Client;
use scraper::{Html, Selector};
use tracing::info;
use url::Url;

#[derive(Clone)]
pub(crate) struct FaviconLocation {
    pub url: String,
    pub mime_type: MimeType,
}

#[derive(Clone)]
pub(crate) struct Favicon {
    pub(crate) data: Vec<u8>,
    pub(crate) mime_type: MimeType,
}

#[derive(Debug, Clone)]
pub(crate) enum ParseFaviconError {
    NotFound,
    HttpError { status: u16, url: String },
    ForbiddenAccess { url: String },
    NetworkError { url: String, error: String },
    RedirectLimitExceeded { url: String },
    MalformedUrl { url: String, error: String },
    InvalidImageData { url: String },
    ManifestParsingError { url: String, error: String },
    TooManyRequests { url: String },
    Timeout { url: String },
    EmptyResponse { url: String },
    Other(String),
}

impl std::fmt::Display for ParseFaviconError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParseFaviconError::NotFound => write!(f, "Favicon not found. The website doesn't appear to have a favicon defined in its HTML or at common locations."),
            ParseFaviconError::HttpError { status, url } => {
                let status_explanation = match status {
                    404 => "page or resource not found",
                    500 => "internal server error",
                    502 => "bad gateway",
                    503 => "service unavailable",
                    504 => "gateway timeout",
                    _ => "server returned an error",
                };
                write!(f, "HTTP error {} ({}) fetching favicon from {}. The server might be experiencing issues or the resource might not exist.", status, status_explanation, url)
            }
            ParseFaviconError::ForbiddenAccess { url } => {
                write!(f, "Access forbidden (403) to favicon at {}. The server actively denied access to this resource, possibly due to security measures or authentication requirements.", url)
            }
            ParseFaviconError::NetworkError { url, error } => {
                write!(f, "Network error fetching favicon from {}: {}. This could be due to connectivity issues, DNS problems, or server unavailability.", url, error)
            }
            ParseFaviconError::RedirectLimitExceeded { url } => {
                write!(f, "Too many redirects for favicon at {}. The server might be implementing an infinite redirect loop or the resource location is misconfigured.", url)
            }
            ParseFaviconError::MalformedUrl { url, error } => {
                write!(f, "Malformed URL {} - error: {}. The URL format is invalid and cannot be processed.", url, error)
            }
            ParseFaviconError::InvalidImageData { url } => {
                write!(f, "Invalid image data at {}. The file exists but contains corrupt or unrecognized image data.", url)
            }
            ParseFaviconError::ManifestParsingError { url, error } => {
                write!(f, "Failed to parse manifest at {}: {}. The web app manifest file exists but has an invalid format or missing required elements.", url, error)
            }
            ParseFaviconError::TooManyRequests { url } => {
                write!(f, "Rate limited (429) for favicon at {}. The server has temporarily blocked access due to too many requests. Try again later or reduce request frequency.", url)
            }
            ParseFaviconError::Timeout { url } => {
                write!(f, "Request timed out for favicon at {}. The server took too long to respond, possibly due to high load or network congestion.", url)
            }
            ParseFaviconError::EmptyResponse { url } => {
                write!(f, "Empty response for favicon at {}. The server returned a successful status code but no content, which might indicate a misconfiguration.", url)
            }
            ParseFaviconError::Other(err) => write!(f, "Error parsing favicon: {}. An unexpected issue occurred during the favicon retrieval process.", err),
        }
    }
}

impl std::error::Error for ParseFaviconError {}

#[derive(Debug, Clone)]
pub struct FaviconFetchAttempt {
    pub url: String,
    pub result: Result<(), ParseFaviconError>,
    pub timestamp: std::time::SystemTime,
}

impl FaviconFetchAttempt {
    pub fn new(url: String, result: Result<(), ParseFaviconError>) -> Self {
        Self {
            url,
            result,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

pub fn parse_favicon_url(html: &str, base_url: Url) -> Option<FaviconLocation> {
    let document = Html::parse_document(html);
    let mut favicon_urls = Vec::new();

    fn parse_size(size: Option<&str>) -> u32 {
        size.and_then(|s| s.split('x').next())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    }

    // Check for <link rel="icon" type="image/svg+xml">
    let svg_selector = Selector::parse(r#"link[rel="icon"][type="image/svg+xml"]"#).unwrap();
    for svg_element in document.select(&svg_selector) {
        if let Some(href) = svg_element.value().attr("href") {
            if let Ok(url) = base_url.join(href) {
                favicon_urls.push((u32::MAX, url.to_string(), "image/svg+xml".to_string()));
            }
        }
    }

    // Check for <link rel="icon"> or <link rel="shortcut icon">
    let icon_selector =
        Selector::parse(r#"link[rel~="icon"], link[rel~="shortcut icon"]"#).unwrap();
    for icon_element in document.select(&icon_selector) {
        if let Some(href) = icon_element.value().attr("href") {
            let size = parse_size(icon_element.value().attr("sizes"));
            if let Ok(url) = base_url.join(href) {
                let mime_type = icon_element
                    .value()
                    .attr("type")
                    .unwrap_or("image/x-icon")
                    .to_string();
                favicon_urls.push((size, url.to_string(), mime_type));
            }
        }
    }

    // Check for <link rel="apple-touch-icon">
    let apple_icon_selector = Selector::parse(r#"link[rel~="apple-touch-icon"]"#).unwrap();
    for icon_element in document.select(&apple_icon_selector) {
        if let Some(href) = icon_element.value().attr("href") {
            let size = parse_size(icon_element.value().attr("sizes"));
            if let Ok(url) = base_url.join(href) {
                favicon_urls.push((size, url.to_string(), "image/png".to_string()));
            }
        }
    }

    // Sort by size in descending order (SVGs first due to MAX)
    favicon_urls.sort_by(|a, b| b.0.cmp(&a.0));
    favicon_urls
        .into_iter()
        .map(|(_, url, mime_type)| FaviconLocation {
            url,
            mime_type: mime_type::MimeType::from_str(&mime_type),
        })
        .next()
}

pub(crate) async fn check_for_favicon(icon_url: String) -> Result<Vec<u8>, ParseFaviconError> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| ParseFaviconError::Other(format!("Failed to initialize HTTP client: {}", e.to_string())))?;

    info!("Checking favicon at: {icon_url}");

    let response = match client
        .get(&icon_url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Version/14.1.2 Safari/537.36",
        )
        .send()
        .await {
            Ok(resp) => resp,
            Err(e) => {
                if e.is_timeout() {
                    return Err(ParseFaviconError::Timeout { 
                        url: icon_url 
                    });
                } else if e.is_connect() {
                    return Err(ParseFaviconError::NetworkError {
                        url: icon_url,
                        error: format!("Connection error: {}", e.to_string())
                    });
                } else if e.is_redirect() {
                    return Err(ParseFaviconError::RedirectLimitExceeded { 
                        url: icon_url 
                    });
                } else {
                    return Err(ParseFaviconError::NetworkError {
                        url: icon_url,
                        error: format!("Network request failed: {}", e.to_string())
                    });
                }
            }
        };

    let status = response.status();
    if !status.is_success() {
        return match status.as_u16() {
            403 => Err(ParseFaviconError::ForbiddenAccess { 
                url: icon_url 
            }),
            429 => Err(ParseFaviconError::TooManyRequests { 
                url: icon_url 
            }),
            404 => Err(ParseFaviconError::HttpError {
                status: status.into(),
                url: format!("{} (resource not found)", icon_url),
            }),
            // Handle server errors specifically
            500 => Err(ParseFaviconError::HttpError {
                status: status.into(),
                url: format!("{} (server error)", icon_url),
            }),
            502 => Err(ParseFaviconError::HttpError {
                status: status.into(),
                url: format!("{} (bad gateway)", icon_url),
            }),
            503 => Err(ParseFaviconError::HttpError {
                status: status.into(),
                url: format!("{} (service unavailable)", icon_url),
            }),
            504 => Err(ParseFaviconError::HttpError {
                status: status.into(),
                url: format!("{} (gateway timeout)", icon_url),
            }),
            status => Err(ParseFaviconError::HttpError {
                status,
                url: icon_url,
            }),
        };
    }

    match response.bytes().await {
        Ok(bytes) => {
            if bytes.is_empty() {
                return Err(ParseFaviconError::EmptyResponse { 
                    url: icon_url 
                });
            }

            // TODO: Add image validation here to check for valid image formats
            // This would prevent returning non-image data as favicons
            
            Ok(bytes.to_vec())
        }
        Err(e) => Err(ParseFaviconError::NetworkError {
            url: icon_url,
            error: format!("Failed to read response body: {}", e.to_string()),
        }),
    }
}

pub(crate) async fn fetch_and_parse_favicon(
    website: String,
) -> Result<(Favicon, Vec<FaviconFetchAttempt>), ParseFaviconError> {
    // Configure client with appropriate timeouts and redirect policy
    let client = Client::builder()
        .redirect(Policy::limited(15))  // Limit redirects to prevent infinite loops
        .timeout(std::time::Duration::from_secs(15))  // Set a reasonable timeout
        .build()
        .map_err(|err| ParseFaviconError::Other(format!("Failed to initialize HTTP client: {}", err.to_string())))?;

    // Ensure URL has protocol prefix
    let website_url = if website.starts_with("http") {
        website.clone()
    } else {
        format!("https://{}", website)  // Default to HTTPS
    };

    // Parse and validate the URL
    let parsed_url = Url::parse(&website_url).map_err(|e| ParseFaviconError::MalformedUrl {
        url: website.clone(),
        error: format!("URL parsing failed: {}", e.to_string()),
    })?;

    // Track all attempts to fetch favicons (for debugging and user feedback)
    let mut attempts = Vec::new();

    // Step 1: Fetch the website HTML
    let response = match client
        .get(parsed_url.clone())
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Version/14.1.2 Safari/537.36",
        )
        .send()
        .await {
            Ok(resp) => resp,
            Err(e) => {
                // Classify the error into specific types for better user feedback
                let error = if e.is_timeout() {
                    ParseFaviconError::Timeout { 
                        url: parsed_url.to_string() 
                    }
                } else if e.is_connect() {
                    ParseFaviconError::NetworkError {
                        url: parsed_url.to_string(),
                        error: format!("Connection failed: {}", e.to_string())
                    }
                } else if e.is_redirect() {
                    ParseFaviconError::RedirectLimitExceeded { 
                        url: parsed_url.to_string() 
                    }
                } else {
                    ParseFaviconError::NetworkError {
                        url: parsed_url.to_string(),
                        error: format!("Request failed: {}", e.to_string())
                    }
                };
                // Record this failed attempt
                attempts.push(FaviconFetchAttempt::new(parsed_url.to_string(), Err(error.clone())));
                return Err(error);
            }
        };

    // Check if we got a successful response
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let error = match status {
            403 => ParseFaviconError::ForbiddenAccess {
                url: parsed_url.to_string(),
            },
            429 => ParseFaviconError::TooManyRequests {
                url: parsed_url.to_string(),
            },
            404 => ParseFaviconError::HttpError {
                status: status.into(),
                url: parsed_url.to_string(),
            },
            // Handle other common HTTP error codes specifically
            500 | 501 | 502 | 503 | 504 => ParseFaviconError::HttpError {
                status: status.into(),
                url: parsed_url.to_string(),
            },
            status => ParseFaviconError::HttpError {
                status: status.into(),
                url: parsed_url.to_string(),
            },
        };
        // Record this HTTP error attempt
        attempts.push(FaviconFetchAttempt::new(
            parsed_url.to_string(),
            Err(error.clone()),
        ));
        return Err(error);
    }

    let final_url = response.url().clone();
    let html = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            let error = ParseFaviconError::NetworkError {
                url: parsed_url.to_string(),
                error: e.to_string(),
            };
            attempts.push(FaviconFetchAttempt::new(
                parsed_url.to_string(),
                Err(error.clone()),
            ));
            return Err(error);
        }
    };

    // Step 2: Parse the favicon URL from the HTML
    if let Some(favicon_location) = parse_favicon_url(&html, final_url) {
        // Record this attempt
        match check_for_favicon(favicon_location.url.clone()).await {
            Ok(data) => {
                attempts.push(FaviconFetchAttempt::new(
                    favicon_location.url.clone(),
                    Ok(()),
                ));
                return Ok((
                    Favicon {
                        data,
                        mime_type: favicon_location.mime_type,
                    },
                    attempts,
                ));
            }
            Err(error) => {
                attempts.push(FaviconFetchAttempt::new(
                    favicon_location.url.clone(),
                    Err(error),
                ));
            }
        }
    } else {
        attempts.push(FaviconFetchAttempt::new(
            parsed_url.to_string(),
            Err(ParseFaviconError::NotFound),
        ));
    }

    Err(ParseFaviconError::NotFound)
}
