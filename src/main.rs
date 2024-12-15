use axum::http::Method;
use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
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

#[derive(Serialize)]
struct FaviconResult {
    url: String,
    status: String,
    path: Option<String>,
    attempted_urls: Option<Vec<String>>,
    width: Option<u32>,
    height: Option<u32>,
    byte_size: Option<usize>,
    mime_type: Option<String>,
}

async fn get_favicons(Json(website_list): Json<WebsiteList>) -> impl IntoResponse {
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
                width,
                height,
                byte_size: Some(byte_size),
                mime_type: Some(mime_type),
            },
            Ok(ProcessWebsiteResult::Failure { attempted_urls }) => FaviconResult {
                url: website.clone(),
                status: "Failed".to_string(),
                path: None,
                attempted_urls: Some(attempted_urls),
                width: None,
                height: None,
                byte_size: None,
                mime_type: None,
            },
            Err(_) => FaviconResult {
                url: website.clone(),
                status: "Error".to_string(),
                path: None,
                attempted_urls: None,
                width: None,
                height: None,
                byte_size: None,
                mime_type: None,
            },
        })
        .collect();

    (StatusCode::OK, axum::Json(favicon_results))
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
        .layer(cors);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Starting server at {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
