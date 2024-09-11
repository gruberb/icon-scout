use crate::favicon::Favicon;
use base64::{engine::general_purpose, Engine as _};
use serde::Serialize;

#[derive(Serialize)]
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

pub(crate) fn generate_data_uri(favicon: &Favicon) -> Option<String> {
    let data_uri = match favicon.mime_type {
        MimeType::ImagePng => format!(
            "data:{};base64,{}",
            MimeType::ImagePng.as_str(),
            general_purpose::STANDARD.encode(&favicon.data)
        ),
        MimeType::ImageSvgXml => {
            let svg_string = String::from_utf8_lossy(&favicon.data);
            format!(
                "data:{};utf8,{}",
                MimeType::ImageSvgXml.as_str(),
                svg_string
            )
        }
        MimeType::ImageXIcon | MimeType::ImageVndMicrosoftIcon => {
            format!(
                "data:{};base64,{}",
                MimeType::ImageXIcon.as_str(),
                general_purpose::STANDARD.encode(&favicon.data)
            )
        }
        MimeType::ImageGif => format!(
            "data:{};base64,{}",
            MimeType::ImageGif.as_str(),
            general_purpose::STANDARD.encode(&favicon.data)
        ),
        MimeType::ImageJpeg => format!(
            "data:{};base64,{}",
            MimeType::ImageJpeg.as_str(),
            general_purpose::STANDARD.encode(&favicon.data)
        ),
        MimeType::ImageWebp => format!(
            "data:{};base64,{}",
            MimeType::ImageWebp.as_str(),
            general_purpose::STANDARD.encode(&favicon.data)
        ),
        MimeType::Unknown(_) => return None,
    };
    Some(data_uri)
}
