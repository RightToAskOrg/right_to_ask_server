//! Utilities for parse_upcoming_hearings.rs and parse_mp_lists.rs

use std::io::Write;
use reqwest::Client;
use tempfile::NamedTempFile;
use reqwest::header::{HeaderMap, ACCEPT, USER_AGENT, CONTENT_TYPE};

/// Temporary file directory. Should be in same filesystem as MP_SOURCE.
const TEMP_DIR : &'static str = "data/temp";
const DD_USER_AGENT : &'static str = "right-to-ask/api; https://www.democracydevelopers.org.au/; info@democracydevelopers.org.au";
pub const WIKI_DATA_BASE_URL : &'static str = "https://query.wikidata.org/sparql?query=";

/// Download from a URL to a temporary file.
pub(crate) async fn download_to_file(url:&str) -> anyhow::Result<NamedTempFile> {
    println!("Downloading {}",url);
    std::fs::create_dir_all(TEMP_DIR)?;
    let mut file = NamedTempFile::new_in(TEMP_DIR)?;
    let response = reqwest::get(url).await?;
    let content= response.bytes().await?;
    file.write_all(&content)?;
    file.flush()?;
    Ok(file)
}

/// Download a json file using a wikidata query.
pub(crate) async fn download_wiki_data_to_file(query:&str, client: Client) -> anyhow::Result<NamedTempFile> {
    println!("Downloading wiki data");
    std::fs::create_dir_all(TEMP_DIR)?;
    let mut file = NamedTempFile::new_in(TEMP_DIR)?;
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, DD_USER_AGENT.parse().unwrap());
    headers.insert(ACCEPT, "application/json".parse().unwrap());
    headers.insert(CONTENT_TYPE, "application/sparql-query".parse().unwrap());
    let response = client.post(WIKI_DATA_BASE_URL)
        .headers(headers)
        .body(query.clone().to_string())
        .send()
        .await?;
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