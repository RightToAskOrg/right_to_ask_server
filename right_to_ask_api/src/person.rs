
//! Information about a person. Includes APIs for modifying the database.

use serde::{Serialize,Deserialize};


use crate::regions::{State, Electorate};
use std::fmt;
use std::fmt::Debug;
use std::sync::Mutex;
use std::time::Duration;
use anyhow::anyhow;
use mysql::{TxOpts, Value, FromValueError};
use crate::database::{get_rta_database_connection, LogInBulletinBoard};
use mysql::prelude::{Queryable, ConvIr, FromValue};
use merkle_tree_bulletin_board::hash::HashValue;
use once_cell::sync::Lazy;
use rand::Rng;
use sha2::{Digest, Sha256};
use crate::mp::MPSpec;
use crate::signing::ClientSigned;
use crate::time_limited_hashmap::TimeLimitedHashMap;

/// A unique ID identifying a person.
pub type UserUID = String;

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
    uid : UserUID,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    display_name : Option<String>,
    public_key : PublicKey,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    state : Option<State>,
    #[serde(default,skip_serializing_if = "Vec::is_empty")]
    electorates : Vec<Electorate>
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
fn bulletin_board_error(error:anyhow::Error) -> RegistrationError {
    eprintln!("Bulletin Board error {:?}",error);
    RegistrationError::CouldNotWriteToBulletinBoard
}
fn internal_error<T:Debug>(error:T) -> RegistrationError {
    eprintln!("Internal error {:?}",error);
    RegistrationError::InternalError
}
fn email_internal_error<T:Debug>(error:T) -> EmailValidationError {
    eprintln!("Internal error {:?}",error);
    EmailValidationError::InternalError
}
#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub struct UserInfo {
    uid : UserUID,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    display_name : Option<String>,
    public_key : PublicKey,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    state : Option<State>,
    #[serde(default,skip_serializing_if = "Vec::is_empty")]
    electorates : Vec<Electorate>,
    badges : Vec<Badge>,
}

#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
/// Like UserInfo, but less info. For searches.
pub struct MiniUserInfo {
    uid : UserUID,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    display_name : Option<String>,
    #[serde(default,skip_serializing_if = "Vec::is_empty")]
    badges : Vec<Badge>,
}

#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
/// What a badge represents.
pub enum BadgeType {
    EmailDomain,
    MP,
    MPStaff,
}

/// Some verification that someone has access to email.
/// What is the domain for email, or MP name for MP
#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub struct Badge {
    badge : BadgeType,
    /// What the badge is about (the text on a badge)
    /// For an MP, this is MP::badge_name, for an organization it is the domain.
    name: String,
}

impl Badge {
    /// Add a badge to the database
    async fn store_in_database(&self,uid:&str) -> mysql::Result<()> {
        let mut conn = get_rta_database_connection().await?;
        let mut tx = conn.start_transaction(TxOpts::default())?;
        tx.exec_drop("insert into BADGES (UID,badge,what) values (?,?,?)",(uid,&self.badge,&self.name))?;
        tx.commit()?;
        Ok(())
    }
    /// removes a badge from the database.
    async fn remove_from_database(&self,uid:&str) -> mysql::Result<()> {
        let mut conn = get_rta_database_connection().await?;
        let mut tx = conn.start_transaction(TxOpts::default())?;
        tx.exec_drop("delete from BADGES where UID=? and badge=? and what=?",(uid,&self.badge,&self.name))?;
        tx.commit()?;
        Ok(())
    }
    /// See if a badge is already in the database.
    async fn is_in_database(&self,uid:&str) -> mysql::Result<bool> {
        let mut conn = get_rta_database_connection().await?;
        let count : Option<usize> = conn.exec_first("select COUNT(UID) from BADGES where UID=? and badge=? and what=?",(uid,&self.badge,&self.name))?;
        Ok(count.is_some() && count.unwrap()>0)
    }
}

// Provide Display & to_string() for BadgeType enum
impl fmt::Display for BadgeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<BadgeType> for Value {
    fn from(s: BadgeType) -> Self {
        Value::Bytes(s.to_string().into_bytes())
    }
}

impl ConvIr<BadgeType> for BadgeType {
    fn new(v: Value) -> Result<Self, FromValueError> {
        match v { // May have to deal with int and uint if it is an enumeration on the server.
            Value::Bytes(bytes) => match bytes.as_slice() {
                b"EmailDomain" => Ok(BadgeType::EmailDomain),
                b"MP" => Ok(BadgeType::MP),
                b"MPStaff" => Ok(BadgeType::MPStaff),
                _ => Err(FromValueError(Value::Bytes(bytes))),
            },
            v => Err(FromValueError(v)),
        }
    }

    fn commit(self) -> Self { self }
    fn rollback(self) -> Value { self.into() }
}

impl FromValue for BadgeType {
    type Intermediate = Self;
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
        for e in &self.electorates {
            tx.exec_drop("insert into ELECTORATES (UID,Chamber,Electorate) values (?,?,?)",(&self.uid,&e.chamber.to_string(),&e.region))?;
        }
        tx.commit()?;
        Ok(())
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
        let hash = LogInBulletinBoard::NewUser(self.clone()).log_in_bulletin_board().await.map_err(bulletin_board_error)?;
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

pub async fn get_user_by_id(uid:&UserUID) -> mysql::Result<Option<UserInfo>> {
    let mut conn = get_rta_database_connection().await?;
    let electorates = conn.exec_map("SELECT Chamber,Electorate from ELECTORATES where UID=?",(uid,),|(chamber,location)|Electorate{ chamber, region: location })?;
    let badges = conn.exec_map("SELECT badge,what from BADGES where UID=?",(uid,),|(badge,name)|Badge{ badge, name })?;
    if let Some((display_name,state,public_key)) = conn.exec_first("SELECT DisplayName,AusState,PublicKey from USERS where UID=?",(uid,))? {
        Ok(Some(UserInfo{
            uid : uid.to_string(),
            display_name,
            public_key,
            state,
            electorates,
            badges
        }))
    } else {Ok(None)}
}

/// Make a list of users who have a search string as a subset of their UID or DisplayName (case insensitive).
/// want_badges says whether badges are wanted as well (significantly more expensive).
pub async fn search_for_users(search:&str,want_badges:bool) -> mysql::Result<Vec<MiniUserInfo>> {
    let mut conn = get_rta_database_connection().await?;
    let query = "%".to_string()+&search.replace('!',"!!").replace('_',"!_").replace('%',"!%").replace('[',"![").to_uppercase()+"%";
    let mut res : Vec<MiniUserInfo> = conn.exec_map("SELECT UID,DisplayName from USERS where (UPPER(UID) like ? escape '!') or (UPPER(DisplayName) like ? escape '!')",(&query,&query),|(uid,display_name)|MiniUserInfo{uid,display_name,badges:vec![] })?;
    if want_badges {
        for user in &mut res {
            let badges = conn.exec_map("SELECT badge,what from BADGES where UID=?",(&user.uid,),|(badge,name)|Badge{ badge, name })?;
            user.badges=badges;
        }
    }
    Ok(res)
}

pub async fn is_user_mp_or_staffer(uid:&UserUID) -> mysql::Result<bool> {
    let mut conn = get_rta_database_connection().await?;
    let badges = conn.exec_map("SELECT badge,what from BADGES where UID=?",(uid,),|(badge,name)|Badge{ badge, name })?;
    Ok(badges.iter().any(|b|b.badge==BadgeType::MP || b.badge==BadgeType::MPStaff))
}

/// see if a given uid is a valid user.
pub fn user_exists(uid:&UserUID,conn:&mut impl Queryable) -> mysql::Result<bool> {
    let count : usize = conn.exec_first("SELECT COUNT(UID) from USERS where UID=?",(uid,))?.unwrap();
    Ok(count>0)
}

pub async fn get_user_public_key_by_id(uid:&UserUID) -> mysql::Result<Option<String>> {
    let mut conn = get_rta_database_connection().await?;
    conn.exec_first("SELECT PublicKey from USERS where UID=?",(uid,))
}

#[derive(Debug,Clone,Copy,Serialize,Deserialize,Eq,PartialEq)]
pub enum EmailValidationError {
    NoCodeOrExpired,
    WrongUser,
    WrongCode,
    InternalError,
    CouldNotWriteToBulletinBoard,
    MPEmailNotKnown,
    BadgeNameDoesNotMatchEmailAddress,
    AlreadyHaveBadge,
    DoesNotHaveBadgeToRevoke,
    OnDoNotEmailList, // if trying to send to someone who is on the list
    AlreadyOnDoNotEmailList, // if trying to put on and already there.
    NotOnDoNotEmailList, // if trying to take off and not already there.
    SentTooFrequentlyToday,
    SentTooFrequentlyThisMonth,
}

impl fmt::Display for EmailValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn internal_error_email<T:Debug>(error:T) -> EmailValidationError {
    eprintln!("Internal error {:?}",error);
    EmailValidationError::InternalError
}
fn bulletin_board_error_email(error:anyhow::Error) -> EmailValidationError {
    eprintln!("Bulletin Board error {:?}",error);
    EmailValidationError::CouldNotWriteToBulletinBoard
}

/// Information to request that an email be sent asking for verification.
#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub struct RequestEmailValidation {
    why : EmailValidationReason,
    /// the "name" of the badge. For an MP, the [MP::badge_name], for an organization the domain name, for an account recovery...TBD. Possibly the new key?
    name : String,
}

#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub struct EmailAddress {
    email : String,
}

impl EmailAddress {

    /// check to see if the email is in the DoNotEmail list. If so, don't send.
    async fn check_is_not_in_do_not_email_list(&self) -> Result<(), EmailValidationError>  {
        let mut conn = get_rta_database_connection().await.map_err(email_internal_error)?;
        if let Some(count) = conn.exec_first::<u64,_,_>("SELECT COUNT(*) from DoNotEmail where email=?",(&self.canonicalise_for_equality_check(),)).map_err(internal_error_email)? {
            if count==0 { Ok(()) } else { Err(EmailValidationError::OnDoNotEmailList) }
        } else { Err(internal_error_email(anyhow!("No return from select count in is_in_do_not_email_list"))) }
    }

    /// if want_on, insert email address into the DoNotEmail list. If !want_on, remove email address from the DoNotEmail list.
    pub async fn change_do_not_email_list(&self,want_on:bool) -> Result<(), EmailValidationError>  {
        let mut conn = get_rta_database_connection().await.map_err(email_internal_error)?;
        let mut transaction = conn.start_transaction(TxOpts::default()).map_err(email_internal_error)?;
        if let Some(count) = transaction.exec_first::<u64,_,_>("SELECT COUNT(*) from DoNotEmail where email=?",(&self.canonicalise_for_equality_check(),)).map_err(internal_error_email)? {
            if want_on {
                if count!=0  { return Err(EmailValidationError::AlreadyOnDoNotEmailList) }
            } else {
                if count==0  { return Err(EmailValidationError::NotOnDoNotEmailList) }
            }
        } else { return Err(internal_error_email(anyhow!("No return from select count in change_do_not_email_list"))) }
        if want_on {
            transaction.exec_drop("insert into DoNotEmail (email) values (?)",(&self.canonicalise_for_equality_check(),)).map_err(internal_error_email)?;
        } else {
            transaction.exec_drop("delete from DoNotEmail where email=?",(&self.canonicalise_for_equality_check(),)).map_err(internal_error_email)?;
        }
        transaction.commit().map_err(internal_error_email)?;
        Ok(())
    }

    /// Get a simple list of all email addresses in the DoNotEmail table.
    pub async fn get_do_not_email_list() -> Result<Vec<EmailAddress>,EmailValidationError> {
        let mut conn = get_rta_database_connection().await.map_err(email_internal_error)?;
        conn.query_map("SELECT email from DoNotEmail",|email|EmailAddress{email}).map_err(internal_error_email)
    }
    /// Maximum number of emails that can be sent to a given email address in a single day
    const MAX_SENT_PER_DAY: u32 = 5;
    /// Maximum number of emails that can be sent to a given email address in a single month
    const MAX_SENT_PER_MONTH: u32 = 10;

    /// Fred@Fred.COM and fred@fred.com are the same email address. Convert to a simple form.
    /// TODO deal with fred+32@fred.com
    fn canonicalise_for_equality_check(&self) -> String {
        self.email.to_lowercase()
    }

    /// record the fact that an email is about to be sent to this email address, and return an error if it is already sent to frequently.
    ///
    async fn add_to_times_sent(&self) -> Result<(),EmailValidationError> {
        let mut conn = get_rta_database_connection().await.map_err(email_internal_error)?;
        let mut transaction = conn.start_transaction(TxOpts::default()).map_err(email_internal_error)?;
        // first check we aren't overdoing things
        let existing : Vec<(u32,u32)> = transaction.exec_map("SELECT timescale,sent from EmailRateLimitHistory where email=?",(&self.canonicalise_for_equality_check(),),|(timescale,sent)|(timescale,sent)).map_err(internal_error_email)?;
        for (timescale,sent) in &existing {
            match *timescale {
                0 => if *sent>=Self::MAX_SENT_PER_DAY {return Err(EmailValidationError::SentTooFrequentlyToday)}
                1 => if *sent>=Self::MAX_SENT_PER_MONTH {return Err(EmailValidationError::SentTooFrequentlyThisMonth)}
                _ => return Err(EmailValidationError::InternalError)
            }
        }
        // indicate that we are doing them.
        for timescale in [0,1] {
            if let Some((_,sent)) = existing.iter().find(|(t,_)|*t==timescale) {
                transaction.exec_drop("update EmailRateLimitHistory set sent=? where email=? and timescale=?",(*sent+1,&self.canonicalise_for_equality_check(),timescale)).map_err(internal_error_email)?;
            } else {
                transaction.exec_drop("insert into EmailRateLimitHistory (email,timescale,sent) values (?,?,1)",(&self.canonicalise_for_equality_check(),timescale)).map_err(internal_error_email)?;
            }
        }
        transaction.commit().map_err(internal_error_email)?;
        Ok(())
    }

    /// Get rid of all entries in the EmailRateLimitHistory with a particular timescale (0=day, 1=month).
    pub async fn reset_times_sent(timescale:u32) -> Result<(),EmailValidationError> {
        let mut conn = get_rta_database_connection().await.map_err(email_internal_error)?;
        conn.exec_drop("delete from EmailRateLimitHistory where timescale=?",(timescale,)).map_err(internal_error_email)
    }

    /// Get rid of all entries in the EmailRateLimitHistory with a particular timescale (0=day, 1=month).
    pub async fn get_times_sent(timescale:u32) -> Result<Vec<TimesSent>,EmailValidationError> {
        let mut conn = get_rta_database_connection().await.map_err(email_internal_error)?;
        conn.exec_map("select email,sent from EmailRateLimitHistory where timescale=?",(timescale,),|(email,sent)|TimesSent{email,sent}).map_err(internal_error_email)
    }

    pub async fn take_off_times_sent_list(&self) -> Result<(),EmailValidationError> {
        let mut conn = get_rta_database_connection().await.map_err(email_internal_error)?;
        conn.exec_drop("delete from EmailRateLimitHistory where email=?",(&self.canonicalise_for_equality_check(),)).map_err(internal_error_email)
    }
}

#[derive(Debug,Clone,Serialize)]
pub struct TimesSent {
    email : String,
    /// The number of times it has been sent on a given timescale
    sent : u32,
}

pub static EMAIL_VALIDATION_CODE_STORAGE : Lazy<Mutex<TimeLimitedHashMap<HashValue,(u32,ClientSigned<RequestEmailValidation,EmailAddress>)>>> = Lazy::new(||Mutex::new(TimeLimitedHashMap::new(Duration::from_secs(3600))));

impl RequestEmailValidation {
    /// Deal with a RequestEmailValidation
    /// * Post the request to the bulletin board? Should this be done??
    /// * Make a response code and email it to the requested email address.
    /// * Store said code for use with EmailProof.
    ///
    /// Returns a hash value that can be used for EmailProof.
    pub async fn process(sig : &ClientSigned<RequestEmailValidation,EmailAddress>) -> Result<HashValue, EmailValidationError> {
        sig.signed_message.unsigned.check_is_not_in_do_not_email_list().await?;
        let badge = RequestEmailValidation::get_badge(sig)?;
        match sig.parsed.why.get_type() {
            EmailValidationType::GainBadge => {
                if badge.is_in_database(&sig.signed_message.user).await.map_err(internal_error_email)? { return Err(EmailValidationError::AlreadyHaveBadge); }
            },
            EmailValidationType::RevokeBadge(uid) => {
                if !badge.is_in_database(&uid).await.map_err(internal_error_email)? { return Err(EmailValidationError::DoesNotHaveBadgeToRevoke); }
            },
            EmailValidationType::AccountRecovery => {}
        }
        let code : u32 = rand::thread_rng().gen_range(100000..1000000);
        sig.signed_message.unsigned.add_to_times_sent().await?;
        println!("Consider this an email to {} with code {}",sig.signed_message.unsigned.email,code); // TODO actually send email.
        let hash = {
            let data = serde_json::ser::to_string(&sig.signed_message).unwrap();
            let mut hasher = Sha256::default();
            hasher.update(data.as_bytes());
            hasher.update(sig.signed_message.unsigned.email.as_bytes());
            HashValue(<[u8; 32]>::from(hasher.finalize()))
        };
        EMAIL_VALIDATION_CODE_STORAGE.lock().unwrap().insert(hash,(code,sig.clone()));
        Ok(hash)
    }

    pub fn get_badge(sig : &ClientSigned<RequestEmailValidation,EmailAddress>) -> Result<Badge,EmailValidationError> {
        match &sig.parsed.why {
            EmailValidationReason::AsMP(principal) => {
                let mps = MPSpec::get().map_err(internal_error_email)?;
                let mp = mps.find_by_email(&sig.signed_message.unsigned.email).ok_or(EmailValidationError::MPEmailNotKnown)?;
                if mp.badge_name()!=sig.parsed.name { return Err(EmailValidationError::BadgeNameDoesNotMatchEmailAddress)}
                Ok(Badge{
                    badge: if *principal {BadgeType::MP} else {BadgeType::MPStaff},
                    name: sig.parsed.name.clone(),
                })
            }
            EmailValidationReason::AsOrg => {
                let domain = sig.signed_message.unsigned.email.trim_start_matches(|c|c!='@');
                if domain!=sig.parsed.name.as_str() { return Err(EmailValidationError::BadgeNameDoesNotMatchEmailAddress)}
                Ok(Badge{
                    badge: BadgeType::EmailDomain,
                    name: sig.parsed.name.clone(),
                })
            }
            EmailValidationReason::AccountRecovery => {
                Err(EmailValidationError::InternalError) // TODO we haven't worked out how account recovery works yet.
            }
            EmailValidationReason::RevokeMP(_uid,principal) => {
                let mps = MPSpec::get().map_err(internal_error_email)?;
                let mp = mps.find_by_email(&sig.signed_message.unsigned.email).ok_or(EmailValidationError::MPEmailNotKnown)?;
                if mp.badge_name()!=sig.parsed.name { return Err(EmailValidationError::BadgeNameDoesNotMatchEmailAddress)}
                Ok(Badge{
                    badge: if *principal {BadgeType::MP} else {BadgeType::MPStaff},
                    name: sig.parsed.name.clone(),
                })
            }
            EmailValidationReason::RevokeOrg(_uid) => {
                let domain = sig.signed_message.unsigned.email.trim_start_matches(|c|c!='@');
                if domain!=sig.parsed.name.as_str() { return Err(EmailValidationError::BadgeNameDoesNotMatchEmailAddress)}
                Ok(Badge{
                    badge: BadgeType::EmailDomain,
                    name: sig.parsed.name.clone(),
                })
            }
        }
    }


}

#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub enum EmailValidationReason {
    AsMP(bool), // if argument is true, the principal. Otherwise a staffer with access to email.
    AsOrg,
    AccountRecovery,
    RevokeMP(UserUID,bool), // revoke a given UID. bool same meaning as AsMP.
    RevokeOrg(UserUID), // revoke a given UID
}

enum EmailValidationType {
    GainBadge,
    RevokeBadge(UserUID),
    AccountRecovery
}

impl EmailValidationReason {
    fn get_type(&self) -> EmailValidationType {
        match self {
            EmailValidationReason::AsMP(_) => EmailValidationType::GainBadge,
            EmailValidationReason::AsOrg => EmailValidationType::GainBadge,
            EmailValidationReason::AccountRecovery => EmailValidationType::AccountRecovery,
            EmailValidationReason::RevokeMP(s, _) => EmailValidationType::RevokeBadge(s.clone()),
            EmailValidationReason::RevokeOrg(s) => EmailValidationType::RevokeBadge(s.clone()),
        }
    }
}

/// Information to request that an email be sent asking for verification.
#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub struct EmailProof {
    hash : HashValue, // value returned from RequestEmailValidation::process()
    code : u32, // email address to be validated
}

impl EmailProof {
    /// Action the email proof. Assign the appropriate badge (or unassign as appropriate).
    /// TODO it would be good to tell people they have been revoked, and by whom.
    pub async fn process(sig : &ClientSigned<EmailProof>) -> Result<Option<HashValue>, EmailValidationError> {
        if let Some((code,initial_request)) = EMAIL_VALIDATION_CODE_STORAGE.lock().unwrap().get(&sig.parsed.hash) {
            if initial_request.signed_message.user!=sig.signed_message.user { return Err(EmailValidationError::WrongUser)}
            if *code!=sig.parsed.code { return Err(EmailValidationError::WrongCode)}
            let badge = RequestEmailValidation::get_badge(initial_request)?;
            // successfully verified!
            match initial_request.parsed.why.get_type() {
                EmailValidationType::GainBadge => {
                    if badge.is_in_database(&initial_request.signed_message.user).await.map_err(internal_error_email)? { return Err(EmailValidationError::AlreadyHaveBadge); }
                    badge.store_in_database(&initial_request.signed_message.user).await.map_err(internal_error_email)?
                },
                EmailValidationType::RevokeBadge(uid) => {
                    if !badge.is_in_database(&uid).await.map_err(internal_error_email)? { return Err(EmailValidationError::DoesNotHaveBadgeToRevoke); }
                    badge.remove_from_database(&uid).await.map_err(internal_error_email)?
                },
                EmailValidationType::AccountRecovery => {} // TODO we haven't worked out how account recovery works yet.
            }
            let bb_hash = LogInBulletinBoard::EmailVerification(initial_request.signed_message.just_signed_part()).log_in_bulletin_board().await.map_err(bulletin_board_error_email)?;
            Ok(Some(bb_hash))
        } else { Err(EmailValidationError::NoCodeOrExpired)}
    }
}

/// Information for the EditRegistration function
#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub struct EditUserDetails {
    #[serde(default,skip_serializing_if = "Option::is_none")]
    display_name : Option<String>,
    #[serde(default,skip_serializing_if = "Option::is_none",with = "::serde_with::rust::double_option")]
    state : Option<Option<State>>,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    electorates : Option<Vec<Electorate>>,
}

impl EditUserDetails {
    /// Change the user details, returning the bulletin board entry.
    pub async fn edit_user(edits:&ClientSigned<EditUserDetails>) -> Result<HashValue,RegistrationError> {
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        let mut transaction = conn.start_transaction(TxOpts::default()).map_err(internal_error)?;
        if let Some(display_name) = &edits.parsed.display_name {
            if display_name.len()<1 { return Err(RegistrationError::DisplayNameTooShort); }
            if display_name.len()>60 { return Err(RegistrationError::DisplayNameTooLong); }
            // Set display name
            transaction.exec_drop("update USERS set DisplayName=? where UID=?", (display_name,&edits.signed_message.user)).map_err(internal_error)?;
        }
        if let Some(state) = &edits.parsed.state {
            transaction.exec_drop("update USERS set AusState=? where UID=?", (state.map(|s|s.to_string()),&edits.signed_message.user)).map_err(internal_error)?;
        }
        if let Some(electorates) = &edits.parsed.electorates {
            transaction.exec_drop("delete from ELECTORATES where UID=?", (&edits.signed_message.user,)).map_err(internal_error)?;
            for e in electorates {
                transaction.exec_drop("insert into ELECTORATES (UID,Chamber,Electorate) values (?,?,?)",(&edits.signed_message.user,&e.chamber.to_string(),&e.region)).map_err(internal_error)?;
            }
        }
        transaction.commit().map_err(internal_error)?;
        let version = LogInBulletinBoard::EditUser(edits.signed_message.clone()).log_in_bulletin_board().await.map_err(bulletin_board_error)?;
        Ok(version)
    }
}
