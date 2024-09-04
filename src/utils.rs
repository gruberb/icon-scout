use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use sanitize_filename::sanitize;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

pub fn sanitize_website_filename(url: &str) -> String {
    // Remove "https://" or "http://"
    let sanitized_url = url.replace("https://", "").replace("http://", "");

    // Sanitize the remaining part of the URL
    sanitize(&sanitized_url)
}

pub fn compress_files_to_zip(file_paths: Vec<String>, output_path: &str) -> std::io::Result<()> {
    let path = Path::new(output_path);
    let file = File::create(&path)?;
    let mut zip = ZipWriter::new(file);

    for file_path in file_paths {
        let path = Path::new(&file_path);
        let name = path.file_name().unwrap().to_str().unwrap();
        let mut f = File::open(path)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;

        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);

        zip.start_file(name, options)?;
        zip.write_all(&buffer)?;
    }

    zip.finish()?;
    Ok(())
}
