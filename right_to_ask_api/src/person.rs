
//! Information about a person. Includes APIs for modifying the database.

use serde::{Serialize,Deserialize};


use crate::regions::{State, Electorate};
use std::fmt;
use mysql::{TxOpts};
use crate::database::{get_rta_database_connection, get_bulletin_board};
use mysql::prelude::Queryable;
use merkle_tree_bulletin_board::hash::HashValue;

pub type PublicKey=String;
/// Signature encodings
/// To sign a list of fields:
/// * Each field is converted to a byte array.
/// * fields are then concatenated in the order of the structure.
/// * Strings are encoded as the utf-8 encoding with a trailing 0.
/// * Optional values are encoded as (byte)0 for None and (byte)1 followed by the binary version of the field.
/// * Lists are encoded as a series of elements, with (byte)1 before each element and (byte)0 after the final element.
/// * Structures are encoded as the concatenation of their elements' encodings.
/// * Enumerations are encoded as the string representation, followed by the parameters, if any.
pub type Signature = String;

/// APIs

/// Information for the NewRegistration function
#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub struct NewRegistration {
    uid : String,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    display_name : Option<String>,
    public_key : PublicKey,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    state : Option<State>,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    electorates : Option<Vec<Electorate>>
}

#[derive(Debug,Clone,Copy,Serialize,Deserialize,Eq,PartialEq)]
pub enum RegistrationError {
    UIDAlreadyTaken,
    UIDTooShort,
    UIDTooLong,
    DisplayNameTooShort,
    DisplayNameTooLong,
    InternalError,
    CouldNotWriteToBulletinBoard,
}
impl fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl NewRegistration {
    async fn store_in_database(&self) -> mysql::Result<()> {
        let mut conn = get_rta_database_connection().await?;
        let mut tx = conn.start_transaction(TxOpts::default())?;
        tx.exec_drop("insert into USERS (UID,DisplayName,PublicKey,AusState) values (?,?,?,?)",(&self.uid,&self.display_name,&self.public_key,self.state.map(|s|s.to_string())))?;
        // TODO do something about electorates.
        tx.commit()?;
        Ok(())
    }

    async fn log_in_bulletin_board(&self) -> anyhow::Result<HashValue> {
        let mut board = get_bulletin_board().await;
        let data = serde_json::ser::to_string(self).unwrap();
        board.submit_leaf(&data)
    }
    pub async fn register(&self) -> Result<HashValue,RegistrationError> {
        if self.uid.len()<1 { return Err(RegistrationError::UIDTooShort); }
        if self.uid.len()>30 { return Err(RegistrationError::UIDTooLong); }
        if let Some(dn) = self.display_name.as_ref() {
            if dn.len()<1 { return Err(RegistrationError::DisplayNameTooShort); }
            if dn.len()>60 { return Err(RegistrationError::DisplayNameTooLong); }
        }
        match self.store_in_database().await {
            Ok(_) => {}
            Err(mysql::Error::MySqlError(e)) if e.code==1062 => {return Err(RegistrationError::UIDAlreadyTaken)}
            Err(e) => {
                println!("Error with SQL : {}",e);
                return Err(RegistrationError::InternalError);
            }
        }
        let hash = self.log_in_bulletin_board().await.map_err(|_|RegistrationError::CouldNotWriteToBulletinBoard)?;
        println!("Registered uid={} display_name={:?} state={:?} electorates={:?} public_key={}",self.uid,self.display_name,self.state,self.electorates,self.public_key);
        Ok(hash)
    }
}

pub async fn get_list_of_all_users() -> mysql::Result<Vec<String>> {
    let mut conn = get_rta_database_connection().await?;
    let elements : Vec<String> = conn.exec_map("SELECT UID from USERS",(),|(v,)|v)?;
    Ok(elements)
}
/// Get the number of users of the system.
pub async fn get_count_of_all_users() -> mysql::Result<usize> {
    let mut conn = get_rta_database_connection().await?;
    let elements : usize = conn.exec_first("SELECT COUNT(UID) from USERS",())?.unwrap();
    Ok(elements)
}


/// Information to request that an email be sent asking for verification.
#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub struct RequestEmailValidation {
    uid : String, // uid making the query
    email : String, // email address to be validated
    why : EmailValidationReason,
    signature : Signature, // signature of UTF-8 encoding of uid|0|email|0|why(as string)
}

#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub enum EmailValidationReason {
    AsMP,
    AsOrg,
    AccountRecovery,
    RevokeMP(String), // revoke a given UID.
    RevokeOrg(String), // revoke a given UID
}



/// Information to request that an email be sent asking for verification.
#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub struct EmailProof {
    uid : String, // uid making the query
    pin : String, // email address to be validated
    signature : Signature, // signature of UTF-8 encoding of uid|pin
}


/// Information for the EditRegistration function
#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub struct EditRegistration {
    uid : String,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    display_name : Option<String>,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    public_key : Option<PublicKey>,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    state : Option<State>,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    electorates : Option<Vec<Electorate>>,
    signature : Signature, // signature of UTF-8 encoding of uid,display_name,public_key,state,electorates.
}
