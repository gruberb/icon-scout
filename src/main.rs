use axum::{extract::Json, http::StatusCode, response::IntoResponse, routing::post, Router};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use website::process_website;

mod favicon;
mod mime_type;
mod utils;
mod website;

#[derive(Deserialize)]
struct WebsiteList(Vec<String>);

#[derive(Serialize)]
struct Response {
    url: String,
    data_uri: String,
}

async fn get_favicons(Json(website_list): Json<WebsiteList>) -> impl IntoResponse {
    let tasks: Vec<_> = website_list
        .0
        .iter()
        .map(|website| process_website(website.to_string()))
        .collect();

    // Run all tasks concurrently using join_all
    let results = join_all(tasks).await;

    let mut favicon_data_uris = Vec::new();
    for (website, result) in website_list.0.iter().zip(results.into_iter()) {
        if let Ok(favicon) = result {
            match mime_type::generate_data_uri(&favicon) {
                Some(data_uri) => {
                    favicon_data_uris.push(Response {
                        url: website.clone(),
                        data_uri,
                    });
                }
                None => continue,
            }
        }
    }

    (StatusCode::OK, Json(favicon_data_uris))
}
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/favicons", post(get_favicons));

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
