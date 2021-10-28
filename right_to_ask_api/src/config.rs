


use once_cell::sync::Lazy;
use std::fs;
use serde::{Serialize,Deserialize};

const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Deserialize)]
pub struct Config {
    pub(crate) signing : Base64EncodedKeyPair,
    pub(crate) database : DatabaseURLs,
}


#[derive(Serialize,Deserialize)]
/// A base 64 encoded key pair.
pub(crate) struct Base64EncodedKeyPair {
    pub public : String, // public key
    pub private : String, // private key
}

#[derive(Deserialize)]
pub(crate) struct DatabaseURLs {
    pub rta : String, // RightToAsk database url
    pub bulletinboard : String, // BulletinBoard url
}
pub static CONFIG : Lazy<Config> = Lazy::new(|| {
    let file = fs::read_to_string(CONFIG_FILE_NAME).expect("Could not read config.toml");
    let config : Config = toml::de::from_str(&file).expect("Could not parse config.toml");
    config
});