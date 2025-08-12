//! Stuff to do with signing


use std::convert::TryFrom;
use std::fmt::{Display, Formatter};
use once_cell::sync::Lazy;
use ed25519_dalek::{SigningKey,Signature, VerifyingKey,Verifier,pkcs8::DecodePrivateKey};
use ed25519_dalek::ed25519::signature::{Signer};
use serde::{Serialize,Deserialize};
use crate::config::CONFIG;
use serde::de::DeserializeOwned;
use crate::person::get_user_public_key_by_id;

pub fn base64_decode(s:&str)-> Result<Vec<u8>, base64::DecodeError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s)
}
pub fn base64_encode<T: AsRef<[u8]>>(input: T) -> String { use base64::Engine; base64::engine::general_purpose::STANDARD.encode(input) }

static SERVER_KEY : Lazy<SigningKey>  = Lazy::new(||{
    let private = base64_decode(&CONFIG.signing.private).expect("Could not decode config private key base64 encoding");
    let signing_key = SigningKey::from_pkcs8_der(&private).expect("Could not decode private key as PKCS8");
    let public_key = base64_decode(&CONFIG.signing.public).expect("Could not decode config public key base64 encoding");
    use pkcs8::DecodePublicKey;
    let public_key = VerifyingKey::from_public_key_der(&public_key).expect("Could not decode config public key der encoding");
    let computed_public_key = signing_key.verifying_key();
    if computed_public_key.as_bytes()!=public_key.as_bytes() { panic!("Computed public key from server private key does not match supplied public key.")}
    signing_key
});

// static SERVER_PRIVATE_EXPANDED_KEY : Lazy<ExpandedSecretKey> = Lazy::new(||{ (&SERVER_KEY.secret).into() });

pub fn get_server_public_key_base64encoded() -> String {
    CONFIG.signing.public.clone()
    // base64::encode(SERVER_PUBLIC_KEY.as_bytes())
}

pub fn get_server_public_key_raw_hex() -> String {
    hex::encode(SERVER_KEY.verifying_key().as_bytes())
    // base64::encode(SERVER_PUBLIC_KEY.as_bytes())
}
pub fn get_server_public_key_raw_base64() -> String {
    base64_encode(SERVER_KEY.verifying_key().as_bytes())
    // base64::encode(SERVER_PUBLIC_KEY.as_bytes())
}

// standard way to sign things.
pub fn sign_message(message : &[u8]) -> String {
    let signing_key : &SigningKey = &SERVER_KEY;
    let signature = signing_key.sign(message);
    // let signature = SERVER_PRIVATE_EXPANDED_KEY.sign(message,&SERVER_KEY.public);
    base64_encode(signature.to_bytes())
}

#[derive(Serialize,Deserialize,Debug,Clone)]
#[serde(try_from = "ClientSignedUnparsed<U>")]
#[serde(bound(deserialize = "T: DeserializeOwned"))]
/// This is a signed message from the client to the server.
///
/// The message is a possibly complex structure of type T. It has been encoded as JSON in the
/// [ClientSignedUnparsed::message] field, which has then been signed. Because JSON encoding
/// is not necessarily unique, it is needed to specifically keep the encoding around. This structure
/// transparently serializes/deserializes as if it were a [ClientSignedUnparsed] message,
/// but also decoding to a parsed value.
pub struct ClientSigned<T,U=()> where U: DeserializeOwned {
    #[serde(flatten)]
    pub signed_message : ClientSignedUnparsed<U>,
    #[serde(skip_serializing,bound="")]
    pub parsed : T,
}

#[derive(Serialize,Deserialize,Debug,Clone)]
/// This is a message from a client to the server, signed by the client.
/// The message is generally some JSON encoded data.
///
/// [ClientSigned] is a more type safe version, handling parsing automatically.
///
/// There might be extra, unsigned fields (such as an email address) which are
/// included in U, if specified.
pub struct ClientSignedUnparsed<U=()> {
    /// The message is a JSON encoding of the actual command being sent from the client. The actual command is of type T.
    pub message : String,
    /// the signature of the message
    pub signature : String,
    /// unique ID of the user
    pub user : String,
    #[serde(flatten)]
    pub unsigned : U,
}

impl <T,U> TryFrom<ClientSignedUnparsed<U>> for ClientSigned<T,U> where T: DeserializeOwned, U : DeserializeOwned {
    type Error = serde_json::Error;

    fn try_from(signed_message: ClientSignedUnparsed<U>) -> Result<Self, Self::Error> {
        let parsed : T = serde_json::from_str(&signed_message.message)?;
        Ok(ClientSigned{ signed_message , parsed })
    }
}

#[derive(Debug)]
pub enum SignatureCheckError {
    InternalError,
    NoSuchUser,
    InvalidPublicKeyFormat,
    InvalidSignatureFormat,
    BadSignature,
    UserBlocked,
    UserUnregistered,
}
impl Display for SignatureCheckError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?}",self)
    }
}

impl <U> ClientSignedUnparsed<U> {

    /// Check the signature, return Ok(()) if good, otherwise an error.
    pub async fn check_signature(&self,need_to_have_validated_email:bool) -> Result<(), SignatureCheckError> {
        if let Some(signing_info) = get_user_public_key_by_id(&self.user).await.map_err(|_| SignatureCheckError::InternalError)? {
            if signing_info.blocked { return Err(SignatureCheckError::UserBlocked); }
            if CONFIG.require_validated_email && need_to_have_validated_email && !signing_info.email_validated { return Err(SignatureCheckError::UserUnregistered); }
            let public_key = base64_decode(&signing_info.public_key).map_err(|_| SignatureCheckError::InvalidPublicKeyFormat)?;
            if public_key.len()!=32 { return Err(SignatureCheckError::InvalidPublicKeyFormat)}
            let mut public_key_fixed_size = [0u8;32];
            public_key_fixed_size.copy_from_slice(&public_key);
            let public_key = VerifyingKey::from_bytes(&public_key_fixed_size).map_err(|_| SignatureCheckError::InvalidPublicKeyFormat)?;
            let signature = base64_decode(&self.signature).map_err(|_| SignatureCheckError::InvalidSignatureFormat)?;
            if signature.len()!=64 { return Err(SignatureCheckError::InvalidSignatureFormat)}
            let mut signature_fixed_size = [0u8;64];
            signature_fixed_size.copy_from_slice(&signature);
            let signature = Signature::from_bytes(&signature_fixed_size);
            public_key.verify(self.message.as_bytes(),&signature).map_err(|_| SignatureCheckError::BadSignature)
        } else { Err(SignatureCheckError::NoSuchUser) }
    }

    /// Clone this, discarding any unsigned part. If (as usually) U=() then this is same as clone().
    pub fn just_signed_part(&self) -> ClientSignedUnparsed<()> {
        ClientSignedUnparsed{
            message: self.message.clone(),
            signature: self.signature.clone(),
            user: self.user.clone(),
            unsigned: ()
        }
    }

    #[cfg(test)]
    pub fn sign(message:String,user:&str,private_key:&str,unsigned:U) ->  ClientSignedUnparsed<U> {
        let private_key = base64_decode(&private_key).expect("Could not decode test private key base64 encoding");
        let signer = SigningKey::from_pkcs8_der(&private_key).expect("Could not decode test private key as PKCS8");
        let signature = signer.sign(message.as_bytes());
        let signature = base64_encode(signature.to_bytes());
        ClientSignedUnparsed{ message,signature,user: user.to_string(),unsigned }
    }
}

/// A test public key for use in unit tests. There is no reason not to make this public. It is only used for unit tests. Don't use it for anything else.
/// Can be made by
/// ```bash
/// openssl genpkey -algorithm Ed25519 -out priv.pem
/// cat priv.pem
/// openssl ec -in priv.pem -text -noout | tail -3 | xxd -r -p | base64
/// ```
/// The output of 'cat priv.pem' is the private key, the output of the last line is the public key below.
#[cfg(test)]
pub const DEFAULT_TESTING_PUBLIC_KEY : &str = "1chhwoStgwImuvUkLcZ5RhHjloRTp82ofyyGB8/6GYo="; // Note that this key is just for testing and does not actually keep anything secure and is not used outside of unit testing.
#[cfg(test)]
/// A test public key for use in unit tests. There is no reason not to make this public. It is only used for unit tests. Don't use it for anything else.
/// See DEFAULT_TESTING_PUBLIC_KEY for how it is made using openssl
const DEFAULT_TESTING_SECRET_KEY : &str = "MC4CAQAwBQYDK2VwBCIEICMI7uUJF/iueFO6T5xin638TU7y/6I6avrAM47VzBpr"; // Note that this key is just for testing and does not actually keep anything secure and is not used outside of unit testing.
#[cfg(test)]
pub async fn make_test_signed<T:Serialize+DeserializeOwned,U:DeserializeOwned>(user:&str,to_be_signed:&T,unsigned:U) -> ClientSigned<T,U> {
    let message = serde_json::to_string(to_be_signed).expect("Could not serialize to_be_signed");
    let unparsed = ClientSignedUnparsed::sign(message,user,DEFAULT_TESTING_SECRET_KEY,unsigned);
    unparsed.check_signature(false).await.unwrap();
    unparsed.try_into().expect("Could not parse the signed client")
}

#[derive(Serialize,Deserialize,Debug,Clone)] // deserialization probably won't be needed.
pub struct ServerSigned {
    message : String,
    signature : String
}

impl ServerSigned {
    pub fn new(x:&impl Serialize) -> serde_json::Result<Self> {
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

