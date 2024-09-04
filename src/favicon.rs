use reqwest::redirect::Policy;
use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;
use tracing::info;
use url::Url;

pub fn parse_favicon_url(html: &str, base_url: Url) -> Option<String> {
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
                favicon_urls.push((u32::MAX, url.to_string()));
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
                favicon_urls.push((size, url.to_string()));
            }
        }
    }

    // Check for <link rel="apple-touch-icon">
    let apple_icon_selector = Selector::parse(r#"link[rel~="apple-touch-icon"]"#).unwrap();
    for icon_element in document.select(&apple_icon_selector) {
        if let Some(href) = icon_element.value().attr("href") {
            let size = parse_size(icon_element.value().attr("sizes"));
            if let Ok(url) = base_url.join(href) {
                favicon_urls.push((size, url.to_string()));
            }
        }
    }

    // Sort by size in descending order (SVGs will naturally come first due to u32::MAX) and return the first URL
    favicon_urls.sort_by(|a, b| b.0.cmp(&a.0));
    favicon_urls.into_iter().map(|(_, url)| url).next()
}

pub async fn check_for_favicon(icon_url: String) -> Option<Vec<u8>> {
    let client = Client::new();

    info!("Checking: {icon_url}");
    let response = client
        .get(&icon_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send()
        .await
        .ok()?;
    if response.status().is_success() {
        let bytes = response.bytes().await.ok()?;
        return Some(bytes.to_vec());
    }

    None
}

pub async fn fetch_and_parse_favicon(
    website: String,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Create a client with redirect policy to follow all redirects
    let client = Client::builder()
        .redirect(Policy::limited(10))
        .timeout(Duration::from_secs(5))
        .build()?;

    let mut url = website.to_string();

    if !website.starts_with("www.") {
        url = format!("https://www.{}", website.trim_start_matches("https://"));
    }

    // Fetch the HTML content of the website
    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .send()
        .await?;
    info!("Done fetching");

    let final_url = response.url().clone(); // Final URL after all redirects
    let html = response.text().await?;

    // Attempt to check common locations if parsing fails
    let common_favicon_url = final_url.join("/favicon.ico")?;
    info!("Couldn't find it, let's check {common_favicon_url}");
    if let Some(favicon_data) = check_for_favicon(common_favicon_url.to_string()).await {
        return Ok(favicon_data);
    }
    // Parse the favicon URL from the HTML
    if let Some(favicon_url) = parse_favicon_url(&html, final_url.clone()) {
        if let Some(favicon_data) = check_for_favicon(favicon_url.clone()).await {
            return Ok(favicon_data);
        }
    }

    Ok(Vec::default())
}
