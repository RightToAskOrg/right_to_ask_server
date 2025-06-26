//! Utilities for parse_upcoming_hearings.rs and parse_mp_lists.rs

use std::fs::File;
use std::io::Write;
use anyhow::anyhow;
use itertools::Itertools;
use regex::Regex;
use reqwest::Client;
use tempfile::NamedTempFile;
use reqwest::header::{HeaderMap, ACCEPT, USER_AGENT, CONTENT_TYPE};
use serde_json::Value;

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


/// Download a single wikipedia file (with proper polite headers)
/// and return as a json value
pub(crate) async fn download_wikipedia_data(insecure_url:&str, client: &Client) -> anyhow::Result<Value> {
    let url = insecure_url.replace("http://", "https://");
    println!("Downloading wiki data from {}", &url);
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, DD_USER_AGENT.parse().unwrap());
    headers.insert(ACCEPT, "application/json".parse().unwrap());
    headers.insert(CONTENT_TYPE, "application/sparql-query".parse().unwrap());
    let response = client.get(url)
        .headers(headers)
        .send()
        .await?;
    let content = response.json().await?;
    Ok(content)
}

/// Download a single wikipedia file (with proper polite headers)
/// So far suspiciously identical to download_wiki_data_to_file
/// except for the URL
pub(crate) async fn download_wikipedia_file(insecure_url:&str, client: &Client) -> anyhow::Result<NamedTempFile> {
    let url = insecure_url.replace("http://", "https://");
    println!("Downloading wiki data to file from {}", &url);
    std::fs::create_dir_all(TEMP_DIR)?;
    let mut file = NamedTempFile::new_in(TEMP_DIR)?;
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, DD_USER_AGENT.parse().unwrap());
    headers.insert(ACCEPT, "application/json".parse().unwrap());
    headers.insert(CONTENT_TYPE, "application/sparql-query".parse().unwrap());
    let response = client.post(url)
        .headers(headers)
        .send()
        .await?;
    let content = response.bytes().await?;
    file.write_all(&content)?;
    file.flush()?;
    Ok(file)
}

/// Download a json file using a wikidata query.
pub(crate) async fn download_wiki_data_to_file(query:&str, client: &Client) -> anyhow::Result<NamedTempFile> {
    println!("Downloading wiki data to json file from query");
    std::fs::create_dir_all(TEMP_DIR)?;
    let mut file = NamedTempFile::new_in(TEMP_DIR)?;
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, DD_USER_AGENT.parse().unwrap());
    headers.insert(ACCEPT, "application/json".parse().unwrap());
    headers.insert(CONTENT_TYPE, "application/sparql-query".parse().unwrap());
    let response = client.post(WIKI_DATA_BASE_URL)
        .headers(headers)
        .body(query.to_string())
        .send()
        .await?;
    let content = response.bytes().await?;
    file.write_all(&content)?;
    file.flush()?;
    Ok(file)
}

/// Read the json data stored in file; return a tuple of Name, district, ID, and image url
/// TODO: a struct might be better for this.
pub async  fn parse_wiki_data(file: File) -> anyhow::Result<Vec<(String, String, String, Option<String>)>> {
    let mut mps_data : Vec<(String, String, String, Option<String>)> = Vec::new();
    let raw : Value = serde_json::from_reader(file)?;
    println!("Got data from file: {}", raw.to_string());
    let raw = raw.get("results").unwrap().get("bindings").and_then(|v|v.as_array()).ok_or_else(||anyhow!("Can't parse wiki data json."))?;
    for mp in raw {
       let id_url = mp.get("mp").unwrap().get("value").expect("Can't find mp ID in json").as_str().unwrap();
        // FIXME change to https
       let base_url_regexp = Regex::new(r"http://www.wikidata.org/entity/(?<QID>\w+)").unwrap();
       let id = &base_url_regexp.captures(id_url).unwrap()["QID"]; 
       println!("Got ID {}", id);
       let district = mp.get("districtLabel").unwrap().get("value").expect("Can't find mp's district in json").as_str().unwrap();
       let name = mp.get("mpLabel").unwrap().get("value").expect("Can't find mp's name in json").as_str().unwrap();
       let img = mp.get("image");
       let img: Option<String> = match img {
            Some(img) => Some(img.get("value").expect("Can't find mp's name in json").as_str().unwrap().to_string()),
            None => None 
       };
       println!("Found MP id = {id}, name = {name}, district = {district}", id=id, name=name);
       mps_data.push((name.to_string(), district.to_string(), id.to_string(), img));
    }
    Ok(mps_data)
}

pub fn relative_url(base_url:&str,url:&str) -> anyhow::Result<String> {
    let base = reqwest::Url::parse(base_url)?;
    let res = base.join(url)?;
    Ok(res.to_string())
}