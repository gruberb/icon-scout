use axum::{
    debug_handler, extract::Json, http::StatusCode, response::IntoResponse, routing::get, Router,
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
    let _results = join_all(tasks).await;

    // Handle any errors (optional)

    "Processed".into_response()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/favicons", get(get_favicons));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
