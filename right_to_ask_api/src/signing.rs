//! Stuff to do with signing


use std::convert::TryFrom;
use once_cell::sync::Lazy;
use ed25519_dalek::{Keypair, PUBLIC_KEY_LENGTH, SecretKey, PublicKey, ExpandedSecretKey};
use serde::{Serialize,Deserialize};
use crate::config::CONFIG;
use pkcs8::{PrivateKeyInfo, SubjectPublicKeyInfo};
use pkcs8::der::Decodable;
use serde::de::DeserializeOwned;

static SERVER_KEY : Lazy<Keypair>  = Lazy::new(||{
    let private = base64::decode(&CONFIG.signing.private).expect("Could not decode config private key base64 encoding");
    let pkd : PrivateKeyInfo = PrivateKeyInfo::from_der(&private).expect("Could not decode private key as PKCS8");
    // println!("{:?}",pkd);
    let private = pkd.private_key; // TODO should check oid is { 1.3.101.112 }
    if private.len()!=34 { panic!("Server private key should be 34 bytes, is {} bytes",private.len()) }
    if !private.starts_with(&[4,32]) { panic!("Server private key should start with 4, 32, actually is {:?}",private) }
    let secret = SecretKey::from_bytes(&private[2..]).expect("Could not create server secret key");

    // println!("Server private key {}",hex::encode(secret.as_bytes()));

    let computed_public : PublicKey = (&secret).into();

    let public = base64::decode(&CONFIG.signing.public).expect("Could not decode config public key base64 encoding");
    let pkd : SubjectPublicKeyInfo = SubjectPublicKeyInfo::from_der(&public).expect("Could not decode public key as SubjectPublicKeyInfo (SPKI)");
    // println!("{:?}",pkd);
    let public = pkd.subject_public_key; // TODO should check oid is { 1.3.101.112 }
    if public.len()!=PUBLIC_KEY_LENGTH { panic!("Server public key should be {} bytes, is {} bytes",PUBLIC_KEY_LENGTH,public.len()) }
    if computed_public.as_ref() != public { panic!("Computed public key {:?} does not match config public key {:?}",computed_public.as_ref(),public)}
    let public = PublicKey::from_bytes(public).expect("Could not create server public key");
    Keypair{ secret, public }
});

static SERVER_PRIVATE_EXPANDED_KEY : Lazy<ExpandedSecretKey> = Lazy::new(||{ (&SERVER_KEY.secret).into() });

pub fn get_server_public_key_base64encoded() -> String {
    CONFIG.signing.public.clone()
    // base64::encode(SERVER_PUBLIC_KEY.as_bytes())
}

pub fn get_server_public_key_raw_hex() -> String {
    hex::encode(SERVER_KEY.public.as_bytes())
    // base64::encode(SERVER_PUBLIC_KEY.as_bytes())
}
pub fn get_server_public_key_raw_base64() -> String {
    base64::encode(SERVER_KEY.public.as_bytes())
    // base64::encode(SERVER_PUBLIC_KEY.as_bytes())
}

// standard way to sign things.
pub fn sign_message(message : &[u8]) -> String {
    let signature = SERVER_PRIVATE_EXPANDED_KEY.sign(message,&SERVER_KEY.public);
    base64::encode(signature.to_bytes())
}

#[derive(Serialize,Deserialize,Debug,Clone)]
#[serde(try_from = "ClientSignedUnparsed")]
#[serde(bound(deserialize = "T: DeserializeOwned"))]
/// This is a signed message from the client to the server.
///
/// The message is a possibly complex structure of type T. It has been encoded as JSON in the
/// [ClientSignedUnparsed::message] field, which has then been signed. Because JSON encoding
/// is not necessarily unique, it is needed to specifically keep the encoding around. This structure
/// transparently serializes/deserializes as if it were a [ClientSignedUnparsed] message,
/// but also decoding to a parsed value.
pub struct ClientSigned<T> {
    #[serde(flatten)]
    pub signed_message : ClientSignedUnparsed,
    #[serde(skip_serializing,bound="")]
    pub parsed : T,
}

#[derive(Serialize,Deserialize,Debug,Clone)]
/// This is a message from a client to the server, signed by the client.
/// The message is generally some JSON encoded data.
///
/// [ClientSigned] is a more type safe version, handling parsing automatically.
pub struct ClientSignedUnparsed {
    /// The message is a JSON encoding of the actual command being sent from the client. The actual command is of type T.
    pub message : String,
    /// the signature of the message
    pub signature : String,
    /// unique ID of the user
    pub user : String,
}

impl <T> TryFrom<ClientSignedUnparsed> for ClientSigned<T> where T: DeserializeOwned {
    type Error = anyhow::Error;

    fn try_from(signed_message: ClientSignedUnparsed) -> Result<Self, Self::Error> {
        let parsed : T = serde_json::from_str(&signed_message.message)?;
        Ok(ClientSigned{ signed_message , parsed })
    }
}

impl <T:DeserializeOwned> ClientSigned<T> {

    pub fn check_signature(&self) {

    }
}


#[derive(Serialize,Deserialize)] // deserialization probably won't be needed.
pub struct ServerSigned {
    message : String,
    signature : String
}

impl ServerSigned {
    pub fn new(x:&impl Serialize) -> anyhow::Result<Self> {
        let message = serde_json::to_string(x)?;
        let signature = sign_message(message.as_bytes());
        Ok(ServerSigned{ message, signature })
    }
    pub fn new_string(message : String) -> Self {
        let signature = sign_message(message.as_bytes());
        ServerSigned{ message, signature }
    }

    pub fn sign<T:Serialize,E:ToString>(r:Result<T,E>) -> Result<ServerSigned,String> {
        match r {
            Ok(r) => Self::new(&r).map_err(|e|e.to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn sign_string<T:ToString,E:ToString>(r:Result<T,E>) -> Result<ServerSigned,String> {
        match r {
            Ok(r) => Ok(Self::new_string(r.to_string())),
            Err(e) => Err(e.to_string()),
        }
    }
}

