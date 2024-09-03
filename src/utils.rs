use sanitize_filename::sanitize;

pub fn sanitize_website_filename(url: &str) -> String {
    // Remove "https://" or "http://"
    let sanitized_url = url.replace("https://", "").replace("http://", "");

    // Sanitize the remaining part of the URL
    sanitize(&sanitized_url)
}
