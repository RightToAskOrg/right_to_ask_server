//! Stuff to do with signing


use once_cell::sync::Lazy;
use ed25519_dalek::{Keypair, PUBLIC_KEY_LENGTH, Sha512, Digest, SecretKey, PublicKey};
use serde::{Serialize,Deserialize};
use crate::config::CONFIG;
use pkcs8::{PrivateKeyInfo, SubjectPublicKeyInfo};
use pkcs8::der::Decodable;

static SERVER_KEY : Lazy<Keypair>  = Lazy::new(||{
    let private = base64::decode(&CONFIG.signing.private).expect("Could not decode config private key base64 encoding");
    let pkd : PrivateKeyInfo = PrivateKeyInfo::from_der(&private).expect("Could not decode private key as PKCS8");
    println!("{:?}",pkd);
    let private = pkd.private_key; // should check oid is { 1.3.101.112 }
    if private.len()!=34 { panic!("Server private key should be 34 bytes, is {} bytes",private.len()) }
    if !private.starts_with(&[4,32]) { panic!("Server private key should start with 4, 32, actually is {:?}",private) }
    let secret = SecretKey::from_bytes(&private[2..]).expect("Could not create server secret key");

    let computed_public : PublicKey = (&secret).into();

    let public = base64::decode(&CONFIG.signing.public).expect("Could not decode config public key base64 encoding");
    let pkd : SubjectPublicKeyInfo = SubjectPublicKeyInfo::from_der(&public).expect("Could not decode public key as SubjectPublicKeyInfo (SPKI)");
    println!("{:?}",pkd);
    let public = pkd.subject_public_key;
    if public.len()!=PUBLIC_KEY_LENGTH { panic!("Server public key should be {} bytes, is {} bytes",PUBLIC_KEY_LENGTH,public.len()) }
    if computed_public.as_ref() != public { panic!("Computed public key {:?} does not match config public key {:?}",computed_public.as_ref(),public)}
    let public = PublicKey::from_bytes(public).expect("Could not create server public key");
    Keypair{ secret, public }
});

pub fn get_server_public_key_base64encoded() -> String {
    CONFIG.signing.public.clone()
    // base64::encode(SERVER_PUBLIC_KEY.as_bytes())
}
pub fn sign_message(message : &[u8]) -> String {
    let mut prehashed: Sha512 = Sha512::new();
    prehashed.update(message);
    let signature = SERVER_KEY.sign_prehashed(prehashed,Some(b"RightToAskServer")).expect("Problem with signature");
    base64::encode(signature.to_bytes())
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

