//! Utilities for parse_upcoming_hearings.rs and parse_mp_lists.rs

use std::io::Write;
use tempfile::NamedTempFile;

/// Temporary file directory. Should be in same filesystem as MP_SOURCE.
const TEMP_DIR : &'static str = "data/temp";

/// Download from a URL to a temporary file.
pub(crate) async fn download_to_file(url:&str) -> anyhow::Result<NamedTempFile> {
    println!("Downloading {}",url);
    std::fs::create_dir_all(TEMP_DIR)?;
    let mut file = NamedTempFile::new_in(TEMP_DIR)?;
    let response = reqwest::get(url).await?;
    let content = response.bytes().await?;
    file.write_all(&content)?;
    file.flush()?;
    Ok(file)
}

pub fn relative_url(base_url:&str,url:&str) -> anyhow::Result<String> {
    let base = reqwest::Url::parse(base_url)?;
    let res = base.join(url)?;
    Ok(res.to_string())
}