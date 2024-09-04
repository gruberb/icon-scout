use axum::{
    debug_handler,
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures::future::join_all;
use serde::Deserialize;
use website::process_website;

mod favicon;
mod utils;
mod website;

#[derive(Deserialize)]
struct WebsiteList(Vec<String>);

#[debug_handler]
async fn get_favicons(Json(website_list): Json<WebsiteList>) -> impl IntoResponse {
    let tasks: Vec<_> = website_list
        .0
        .iter()
        .map(|website| process_website(website.to_string()))
        .collect();

    // Run all tasks concurrently using join_all
    let results = join_all(tasks).await;

    let mut file_paths = Vec::new();
    for result in results {
        if let Ok(path) = result {
            file_paths.push(path);
        }
    }

    let output_zip_path = "favicons.zip";
    if let Err(_) = utils::compress_files_to_zip(file_paths, output_zip_path) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create ZIP file",
        )
            .into_response();
    }

    // Return the ZIP file as a response
    let file = match tokio::fs::read(output_zip_path).await {
        Ok(contents) => contents,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read ZIP file").into_response()
        }
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/zip")
        .body(file.into())
        .unwrap()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/favicons", get(get_favicons));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
