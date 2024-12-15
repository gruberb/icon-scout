use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

// Save favicon to disk using the hash of its data as the filename
pub fn save_favicon_to_disk(
    favicon_data: &[u8],
    extension: &str,
) -> Result<PathBuf, std::io::Error> {
    let hash = Sha256::digest(favicon_data);
    let filename = format!("{:x}{}", hash, extension);

    let folder_path = Path::new("favicons");
    if !folder_path.exists() {
        std::fs::create_dir_all(folder_path)?;
    }

    let filepath = folder_path.join(filename);
    let mut file = File::create(filepath.clone())?;
    file.write_all(favicon_data)?;
    Ok(filepath)
}
