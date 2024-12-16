use serde::Serialize;

#[derive(Clone, Serialize)]
pub(crate) enum MimeType {
    ImagePng,
    ImageSvgXml,
    ImageXIcon,
    ImageVndMicrosoftIcon,
    ImageGif,
    ImageJpeg,
    ImageWebp,
    Unknown(String),
}

impl MimeType {
    pub fn from_str(mime_type: &str) -> Self {
        match mime_type {
            "image/png" => MimeType::ImagePng,
            "image/svg+xml" => MimeType::ImageSvgXml,
            "image/x-icon" => MimeType::ImageXIcon,
            "image/vnd.microsoft.icon" => MimeType::ImageVndMicrosoftIcon,
            "image/gif" => MimeType::ImageGif,
            "image/jpeg" => MimeType::ImageJpeg,
            "image/webp" => MimeType::ImageWebp,
            _ => MimeType::Unknown(mime_type.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            MimeType::ImagePng => "image/png",
            MimeType::ImageSvgXml => "image/svg+xml",
            MimeType::ImageXIcon => "image/x-icon",
            MimeType::ImageVndMicrosoftIcon => "image/vnd.microsoft.icon",
            MimeType::ImageGif => "image/gif",
            MimeType::ImageJpeg => "image/jpeg",
            MimeType::ImageWebp => "image/webp",
            MimeType::Unknown(mime_type) => mime_type,
        }
    }
}
