use once_cell::sync::Lazy;
use std::fs;
use std::str::FromStr;
use lettre::address::AddressError;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use serde::{Serialize,Deserialize};

const CONFIG_FILE_NAME: &str = if cfg!(test) {"test_config.toml"} else {"config.toml"};

#[derive(Deserialize)]
pub struct Config {
    pub(crate) signing : Base64EncodedKeyPair,
    pub(crate) database : DatabaseURLs,
    pub(crate) search_cache_size : std::num::NonZeroUsize,
    #[serde(default)]
    pub(crate) require_validated_email: bool, // this will be removed in the future when it is required.
    #[serde(default)]
    pub(crate) email : Option<EmailConfig>,
}

/// a wrapper around Mailbox allowing serde parsing.
#[derive(serde_with::DeserializeFromStr)]
pub struct ParsedEmailAddress(Mailbox);
impl FromStr for ParsedEmailAddress {
    type Err = AddressError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ParsedEmailAddress(FromStr::from_str(s)?))
    }
}

impl ParsedEmailAddress {
    pub fn mailbox(&self) -> Mailbox { self.0.clone() }
}

#[derive(Deserialize)]
pub struct EmailConfig {
    /// The address used as the "from" sender
    pub(crate) verification_from_email : ParsedEmailAddress,
    pub(crate) verification_reply_to_email : ParsedEmailAddress,
    pub(crate) relay : String,
    #[serde(default)]
    /// if using SMTP, give credentials here. Note that STARTTLS on the submission port is used.
    pub(crate) smtp_credentials : Option<Credentials>,
    #[serde(default)]
    /// If present, send emails to this address rather than the supposed address. Use for testing.
    pub(crate) testing_email_override : Option<ParsedEmailAddress>,
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
    let file = fs::read_to_string(CONFIG_FILE_NAME).expect(&format!("Could not read {}",CONFIG_FILE_NAME));
    let config : Config = toml::de::from_str(&file).expect(&format!("Could not parse {}",CONFIG_FILE_NAME));
    config
});

#[cfg(test)]
pub fn change_directory_to_one_containing_config_file() {
    if std::path::Path::new(CONFIG_FILE_NAME).exists() { return; }
    let up_a_folder = format!("../{}",CONFIG_FILE_NAME);
    if std::path::Path::new(&up_a_folder).exists() {
        std::env::set_current_dir("..").unwrap();
    } else {
        panic!("Could not find config file {} either in the current working directory or its parent",CONFIG_FILE_NAME);
    }
}