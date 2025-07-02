//! Utilities for parse_upcoming_hearings.rs and parse_mp_lists.rs

use std::fs::File;
use std::io::Write;
use anyhow::anyhow;
use mysql_common::frunk::labelled::chars::f;
use regex::Regex;
use reqwest::Client;
use tempfile::NamedTempFile;
use reqwest::header::{HeaderMap, ACCEPT, USER_AGENT, CONTENT_TYPE};
use serde_json::Value;

/// Temporary file directory. Should be in same filesystem as MP_SOURCE.
pub(crate) const TEMP_DIR : &'static str = "data/temp";
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
/// except for the URL and the use of get instead of post.
pub(crate) async fn download_wikipedia_file(insecure_url:&str, client: &Client) -> anyhow::Result<NamedTempFile> {
    let url = insecure_url.replace("http://", "https://");
    println!("Downloading wiki data to file from {}", &url);
    std::fs::create_dir_all(TEMP_DIR)?;
    let mut file = NamedTempFile::new_in(TEMP_DIR)?;
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, DD_USER_AGENT.parse().unwrap());
    headers.insert(ACCEPT, "application/json".parse().unwrap());
    headers.insert(CONTENT_TYPE, "application/sparql-query".parse().unwrap());
    let response = client.get(url)
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

/// Read the json data stored in file; return a tuple of Name, district, ID
pub async  fn parse_wiki_data(file: File) -> anyhow::Result<Vec<(String, String, String)>> {
    let mut mps_data : Vec<(String, String, String)> = Vec::new();
    let raw : Value = serde_json::from_reader(file)?;
    println!("Got data from file: {}", raw.to_string());
    let raw = raw.get("results").unwrap().get("bindings").and_then(|v|v.as_array()).ok_or_else(||anyhow!("Can't parse wiki data json."))?;
    for mp in raw {
       let id_url = mp.get("mp").unwrap().get("value").expect("Can't find mp ID in json").as_str().unwrap();
       let base_url_regexp = Regex::new(r"http://www.wikidata.org/entity/(?<QID>\w+)").unwrap();
       let id = &base_url_regexp.captures(id_url).unwrap()["QID"]; 
       println!("Got ID {}", id);
       let district = mp.get("districtLabel").unwrap().get("value").expect("Can't find mp's district in json").as_str().unwrap();
       let name = mp.get("mpLabel").unwrap().get("value").expect("Can't find mp's name in json").as_str().unwrap();
       println!("Found MP id = {id}, name = {name}, district = {district}", id=id, name=name);
       mps_data.push((name.to_string(), district.to_string(), id.to_string()));
    }
    Ok(mps_data)
}

pub fn relative_url(base_url:&str,url:&str) -> anyhow::Result<String> {
    let base = reqwest::Url::parse(base_url)?;
    let res = base.join(url)?;
    Ok(res.to_string())
}

/// extracts as a string a nested json value, by getting each field in sequence.
pub fn get_nested_json<'a>(json: &'a serde_json::Value,fields:&[&str]) -> Option<&'a str> {
    if fields.len() == 0 { json.as_str() }
    else if let Some(nested) = json.get(fields[0]) { get_nested_json(nested,&fields[1..]) }
    else { None }
}

/// extracts as a string a nested json value, by getting each field in sequence.
/// if strip_quotes is true, it applies strip_quotes before returning the string.
pub fn extract_nested_json(json:&Value, fields: &Vec<String>, strip_quotes: bool) -> Option<String> {

    todo!();
    /*
    for field in fields {
        json.get(field).map(|v|{})
    }
     */

}

/// Strip a single pair of outer quotes, either '...' or "...", from a string, if present.
pub fn strip_quotes(s: &str) -> String {
    let double_quote_regexp = Regex::new(r#"^"(?s)(.*)"$"#).unwrap();
    let single_quote_regexp = Regex::new(r#"^'(?s)(.*)'$"#).unwrap();
    if let Some(inner_d) = &double_quote_regexp.captures(s) {
       inner_d[1].to_string()
    } else if let Some(inner_s) = &single_quote_regexp.captures(s) {
       inner_s[1].to_string()
    } else {
        s.to_string()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_quotes1() {
        assert_eq!("Hi", strip_quotes("\"Hi\""));         // Strip matched double quotes
        assert_eq!("Hi", strip_quotes("\'Hi\'"));         // Strip matched single quotes
        assert_eq!("Hi", strip_quotes("Hi"));             // Keep things with no quotes
        assert_eq!("\'Hi", strip_quotes("\'Hi"));         // Don't strip unmatched quotes
        assert_eq!("Hi\"", strip_quotes("Hi\""));         // "
        assert_eq!("H\"i\"", strip_quotes("H\"i\""));     // Don't strip quotes that aren't at the ends
        assert_eq!("\'H\'\"i\"", strip_quotes("\'H\'\"i\""));  //  "
        assert_eq!("\"Hi\'", strip_quotes("\"Hi\'"));     // Don't strip quotes that don't match
        assert_eq!("\"Hi\"", strip_quotes("\"\"Hi\"\"")); // Only strip the outer layer
        assert_eq!("\'Hi\'", strip_quotes("\"\'Hi\'\"")); // "
        assert_eq!("Hi\n", strip_quotes("\"Hi\n\""));     // Include newlines
        assert_eq!("Hi\n", strip_quotes("\'Hi\n\'"));     // "
    }
}