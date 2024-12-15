use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use google_cloud_storage::http::objects::Object;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub async fn save_favicon(
    website: &str,
    favicon_data: &[u8],
    mime_type: impl AsRef<str> + 'static,
) -> Result<String, Box<dyn std::error::Error>> {
    let storage_mode = env::var("STORAGE_MODE").unwrap_or_else(|_| "local".to_string());

    match storage_mode.as_str() {
        "gcs" => {
            let bucket_name = "icon-scout-favicons"; // Replace with your GCS bucket name
            save_favicon_to_gcs(bucket_name, favicon_data, mime_type.as_ref()).await
        }
        _ => save_favicon_to_disk(website, favicon_data, mime_type.as_ref()),
    }
}

fn save_favicon_to_disk(
    website: &str,
    favicon_data: &[u8],
    mime_type: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let folder_path = Path::new("favicons");
    if !folder_path.exists() {
        std::fs::create_dir_all(folder_path)?;
    }

    let extension = get_extension(mime_type);
    let filename = format!("{}{}", sanitize_website_filename(website), extension);
    let filepath = folder_path.join(filename);

    let mut file = File::create(filepath.clone())?;
    file.write_all(favicon_data)?;

    Ok(filepath.to_string_lossy().to_string())
}

pub async fn save_favicon_to_gcs(
    bucket_name: &str,
    favicon_data: &[u8],
    mime_type: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Load the Google service account key file from the environment variable
    let _ = env::var("GOOGLE_APPLICATION_CREDENTIALS").expect(
        "GOOGLE_APPLICATION_CREDENTIALS must be set to the path of your service account key",
    );

    // Compute the hash of the favicon content
    let hash = Sha256::digest(favicon_data);
    let filename = format!("{:x}", hash);

    // Initialize the GCS client with credentials
    let config = ClientConfig::default()
        .with_credentials(google_cloud_auth::credentials::CredentialsFile::new().await?)
        .await?;
    let client = Client::new(config);

    // Create the upload request
    let upload_request = UploadObjectRequest {
        bucket: bucket_name.to_string(),
        ..Default::default()
    };

    let media = Media {
        name: Cow::Owned(filename.to_string()),
        content_type: Cow::Owned(mime_type.to_string()),
        content_length: Some(favicon_data.len() as u64),
    };

    // Set the upload type to `Simple` with the favicon data
    let upload_type = UploadType::Simple(media);

    // Perform the upload
    let response: Object = client
        .upload_object(&upload_request, favicon_data.to_vec(), &upload_type)
        .await?;

    // Construct and return the public URL
    Ok(format!(
        "https://storage.googleapis.com/{}/{}",
        bucket_name, response.name
    ))
}

pub fn decode_image_metadata(data: &[u8]) -> (Option<u32>, Option<u32>) {
    if let Ok(reader) = image::io::Reader::new(std::io::Cursor::new(data)).with_guessed_format() {
        if let Ok(img) = reader.decode() {
            return (Some(img.width()), Some(img.height()));
        }
    }
    (None, None)
}

pub fn sanitize_website_filename(url: &str) -> String {
    url.replace("https://", "")
        .replace("http://", "")
        .replace("/", "_")
}

fn get_extension(mime_type: &str) -> &str {
    match mime_type {
        "image/png" => ".png",
        "image/svg+xml" => ".svg",
        "image/x-icon" | "image/vnd.microsoft.icon" => ".ico",
        "image/gif" => ".gif",
        "image/jpeg" => ".jpg",
        "image/webp" => ".webp",
        _ => ".bin",
    }
}
