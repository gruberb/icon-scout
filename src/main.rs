use axum::http::Method;
use axum::{
    extract::Json,
    http::StatusCode,
    routing::{get, post},
    Router,
};

use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use website::{process_website, ProcessWebsiteResult};

mod favicon;
mod mime_type;
mod utils;
mod website;

#[derive(Deserialize)]
struct WebsiteList(Vec<String>);

#[derive(Deserialize, Serialize)]
struct FaviconResult {
    url: String,
    status: String,
    path: Option<String>,
    attempted_urls: Option<Vec<website::FaviconAttemptResult>>,
    error_reasons: Option<Vec<String>>,
    width: Option<u32>,
    height: Option<u32>,
    byte_size: Option<usize>,
    mime_type: Option<String>,
}

#[derive(Serialize)]
struct ManifestDomain {
    domain: String,
    url: String,
    title: String,
    icon: String,
}

#[derive(Serialize)]
struct Manifest {
    domains: Vec<ManifestDomain>,
}

#[axum::debug_handler]
async fn generate_manifest(
    Json(results): Json<Vec<FaviconResult>>,
) -> Result<Json<Manifest>, StatusCode> {
    let domains: Vec<ManifestDomain> = results
        .into_iter()
        .filter(|result| result.status == "Success" && result.path.is_some())
        .map(|result| {
            let domain = result
                .url
                .replace("https://", "")
                .replace("http://", "")
                .split('.')
                .next()
                .unwrap_or("")
                .to_string();

            ManifestDomain {
                domain: domain.clone(),
                url: result.url,
                title: domain
                    .chars()
                    .next()
                    .unwrap_or('_')
                    .to_uppercase()
                    .chain(domain.chars().skip(1))
                    .collect(),
                icon: result.path.unwrap_or_default(),
            }
        })
        .collect();

    Ok(Json(Manifest { domains }))
}

#[axum::debug_handler]
async fn get_favicons(
    Json(website_list): Json<WebsiteList>,
) -> Result<Json<Vec<FaviconResult>>, StatusCode> {
    // Change the return type to impl IntoResponse
    let tasks: Vec<_> = website_list
        .0
        .iter()
        .map(|website| process_website(website.to_string()))
        .collect();

    let results = join_all(tasks).await;

    let favicon_results: Vec<FaviconResult> = website_list
        .0
        .iter()
        .zip(results.into_iter())
        .map(|(website, result)| match result {
            Ok(ProcessWebsiteResult::Success {
                path,
                mime_type,
                attempted_urls,
                width,
                height,
                byte_size,
            }) => FaviconResult {
                url: website.clone(),
                status: "Success".to_string(),
                path: Some(path),
                attempted_urls: Some(attempted_urls),
                error_reasons: None,
                width,
                height,
                byte_size: Some(byte_size),
                mime_type: Some(mime_type),
            },
            Ok(ProcessWebsiteResult::Failure {
                attempted_urls,
                reasons,
            }) => FaviconResult {
                url: website.clone(),
                status: "Failed".to_string(),
                path: None,
                attempted_urls: Some(attempted_urls),
                error_reasons: Some(reasons),
                width: None,
                height: None,
                byte_size: None,
                mime_type: None,
            },
            Err(e) => {
                let error_message = match e {
                    website::ProcessWebsiteError::SaveError(msg) => {
                        format!("System error: Failed to save favicon to storage: {}. Please check system permissions and available disk space.", msg)
                    }
                    website::ProcessWebsiteError::NetworkError(msg) => {
                        format!("Network error: {}. Please check your internet connection and verify the website is accessible. The server may be down or unreachable.", msg)
                    }
                    website::ProcessWebsiteError::ParseError(msg) => {
                        format!("Parse error: {}. The website's HTML could not be properly parsed. The site may have an unusual structure or invalid markup.", msg)
                    }
                    website::ProcessWebsiteError::HttpError { status, url } => {
                        let status_explanation = match status {
                            404 => "page or resource not found",
                            500 => "internal server error",
                            502 => "bad gateway",
                            503 => "service unavailable",
                            504 => "gateway timeout",
                            _ => "server returned an error",
                        };
                        format!("HTTP error {} ({}): Could not retrieve favicon from {}. The server might be experiencing issues or the resource might not exist.", status, status_explanation, url)
                    }
                    website::ProcessWebsiteError::ForbiddenAccess(url) => {
                        format!("Access forbidden (403): The server at {} actively refused access to the favicon. This website may implement security measures that prevent favicon scraping or may require authentication.", url)
                    }
                    website::ProcessWebsiteError::TooManyRequests(url) => {
                        format!("Rate limited (429): The server at {} has rate-limiting protection in place and has temporarily blocked our requests. Please try again later or reduce the frequency of requests to this domain.", url)
                    }
                    website::ProcessWebsiteError::MalformedUrl { url, error } => {
                        format!("Invalid URL format: The URL '{}' could not be processed because: {}. Please check for typos and ensure the URL is correctly formatted, including the protocol (http:// or https://).", url, error)
                    }
                    website::ProcessWebsiteError::ManifestParsingError { url, error } => {
                        format!("Web app manifest error: Could not parse the manifest file at {}. Error details: {}. The site may have an invalid or non-standard web app manifest.", url, error)
                    }
                    website::ProcessWebsiteError::InvalidImageData { url } => {
                        format!("Invalid image data: The file at {} appears to be corrupted or is not a valid image format. The server may be returning a non-image response like an error page or placeholder.", url)
                    }
                    website::ProcessWebsiteError::Timeout(url) => {
                        format!("Connection timeout: The request to {} took too long to complete and was aborted. This could be due to a slow server response, network congestion, or server-side processing delays.", url)
                    }
                    website::ProcessWebsiteError::EmptyResponse(url) => {
                        format!("Empty response: The server at {} returned an empty response. The favicon file may exist but contain no data, or the server might be misconfigured.", url)
                    }
                };
                FaviconResult {
                    url: website.clone(),
                    status: "Error".to_string(),
                    path: None,
                    attempted_urls: None,
                    error_reasons: Some(vec![error_message]),
                    width: None,
                    height: None,
                    byte_size: None,
                    mime_type: None,
                }
            }
        })
        .collect();

    // Return just the Json type here
    Ok(Json(favicon_results))
}

async fn health_check() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::POST, Method::GET])
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(health_check))
        .route("/api/favicons", post(get_favicons))
        .route("/api/manifest", post(generate_manifest))
        .layer(cors);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Starting server at {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
