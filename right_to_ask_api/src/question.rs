//! Information about a question. Includes APIs for modifying the database.

// Functions here
// - submit New Question
// - edit existing question
// - look up current info on a specific question.


use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::fmt::{Debug, Display, Formatter};
use futures::lock::{Mutex, MutexGuard};
use serde::{Serialize, Deserialize};
use merkle_tree_bulletin_board::hash::HashValue;
use merkle_tree_bulletin_board::hash_history::{Timestamp, timestamp_now};
use mysql::prelude::Queryable;
use mysql::{Transaction, TxOpts};
use once_cell::sync::Lazy;
use rand::Rng;
use reqwest::Url;
use sha2::{Digest, Sha256};
use url::Host;
use word_comparison::comparison_list::ScoredIDs;
use crate::censorship::CensorshipStatus;
use crate::committee::{CommitteeId, CommitteeIndexInDatabaseTable};
use crate::common_file::COMMITTEES;
use crate::config::CONFIG;
use crate::database::{add_question_to_comparison_database, find_similar_text_question, get_rta_database_connection, LogInBulletinBoard};
use crate::minister::{MinisterId, MinisterIndexInDatabaseTable};
use crate::mp::{get_org_id_from_database, MPId, MPIndexInDatabaseTable, MPSpec, OrgIndexInDatabaseTable};
use crate::person::{get_user_id, user_exists, UserID, UserUID};
use crate::signing::ClientSigned;

/// A question ID is a hash of the question text, the question writer, and the upload timestamp.
/// It is NOT directly a node on the bulletin board; it is just using the bulletin board HashValue as that is a convenient way of representing a HashValue with serialization/deserialization/printing/debugging already handled.
pub type QuestionID = HashValue;
/// a definition of the last time the question was updated, which is a node on the bulletin board.
pub type LastQuestionUpdate = HashValue;


#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
/// Errors that could be returned from the APIs to add/edit questions.
/// Lots more to go...
pub enum QuestionError {
    AuthorIsNotRegistered,
    InternalError,
    CouldNotWriteToBulletinBoard,
    QuestionTooShort,
    QuestionTooLong,
    YouJustAskedThatQuestion, // within the last 24 hours
    AnswerTooLong,
    AnswerContainsUndesiredFields, // the answer structure contains timestamp and answered_by fields that are filled in by the server.
    UserDoesNotHaveCorrectMPBadge,
    BackgroundTooLong,
    SameQuestionSubmittedRecently,
    OnlyAuthorCanChangeBackground,
    OnlyAuthorCanChangePermissions,
    OnlyAuthorCanAcceptAnswer,
    CantAcceptAnswerIfNonePresent,
    CanOnlyExtendBackground,
    FollowUpIsNotAValidQuestion,
    FollowUpIsAlreadySet,
    /// The provided question_id does not exist.
    QuestionDoesNotExist,
    /// The provided last_update hash is not the current last update
    LastUpdateIsNotCurrent,
    TooLongListOfPeopleAskingQuestion,
    TooLongListOfPeopleAnsweringQuestion,
    OrganisationNameTooLong,
    /// The provided MP is not one we recognise.
    InvalidMP,
    /// The provided Committee is not one we recognise.
    InvalidCommittee,
    /// The provided Minister is not one we recognise.
    InvalidMinister,
    /// The user to ask/answer the question does not exist.
    InvalidUserSpecified,
    /// The question exists, but was censored.
    Censored,
    /// The data in the bulletin board is not consistent and cannot be loaded.
    /// Note that old format data in the bulletin board can cause this.
    BulletinBoardHistoryIsCorrupt,
    /// Trying to report or censor an answer that is either not an answer to the question or is already censored.
    NotAnUncensoredAnswer,
    HansardLinkIsNotURL,
    /// The Hansard link URL does not satisfy the sanitization filters.
    HansardLinkIsNotAllowed,
    AlreadyVoted,
    /// The question author doesn't exist. Mainly happens if submitting a new question at the same time as changing UID.
    NoSuchUser,
    /// The user is reporting a question (or answer) for something already reported by that same user.
    AlreadyReported,
}

impl Display for QuestionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?}",self)
    }
}
/// The maximum number of characters in a question.
const MAX_QUESTION_LENGTH : usize = 280;
const MIN_QUESTION_LENGTH : usize = 10;
const MAX_BACKGROUND_LENGTH : usize = 280;
const MAX_ANSWER_LENGTH : usize = 1000;
const MAX_MPS_WHO_SHOULD_ASK_THE_QUESTION : usize = 10;
const MAX_MPS_WHO_SHOULD_ANSWER_THE_QUESTION : usize = 10;


/*************************************************************************
                         COMMON DATA STRUCTURES
 *************************************************************************/




/// The fields that never change for a question and are used to define the hash for this field.
///
///
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct QuestionDefiningFields {
    /// The UID of the person asking the question
    author : UserUID,
    /// The actual text of the question.
    question_text : String,
    /// When the question was originally created.
    timestamp : Timestamp,
}

impl QuestionDefiningFields {
    /// The hash value is computed by concatenating
    ///  * The utf8 encoding of the author
    ///  * the byte 0
    ///  * The utf8 encoding of the question text
    ///  * the byte 0
    ///  * The bigendian 64 bit integer timestamp
    /// and then taking the SHA256 hash of the result.
    pub fn compute_hash(&self) -> HashValue {
        let mut hasher = Sha256::default();
        hasher.update(self.author.as_bytes());
        hasher.update(&[0]);
        hasher.update(self.question_text.as_bytes());
        hasher.update(&[0]);
        hasher.update(&self.timestamp.to_be_bytes());
        HashValue(<[u8; 32]>::from(hasher.finalize()))
    }
}

#[derive(Serialize,Deserialize,Copy,Clone,Debug,Eq, PartialEq)]
pub enum Permissions {
    WriterOnly,
    Others,
    NoChange,
}

impl Default for Permissions {
    fn default() -> Self { Self::NoChange }
}

impl Permissions {
    fn is_no_change(&self) -> bool {
        match self {
            Permissions::NoChange => true,
            _ => false,
        }
    }
}
pub type OrgID = String;

#[derive(Serialize,Deserialize,Debug,Clone,Eq,PartialEq,Hash)]
pub enum PersonID {
    User(UserUID),
    MP(MPId),
    Organisation(OrgID),
    Committee(CommitteeId),
    Minister(MinisterId),
}

impl PersonID {
    /// Get the people who should ask (role='Q') or answer (role='A') a question.
    fn get_for_question(conn:&mut impl Queryable,role:char,question:QuestionID) -> mysql::Result<Vec<PersonID>> {
        let elements : Vec<(Option<UserUID>,Option<MPIndexInDatabaseTable>,Option<OrgIndexInDatabaseTable>,Option<CommitteeIndexInDatabaseTable>,Option<MinisterIndexInDatabaseTable>)> = conn.exec_map("SELECT USERS.UID,MP,ORG,Committee,Minister from PersonForQuestion left join USERS ON PersonForQuestion.UserId=USERS.id where QuestionId=? and ROLE=?",(&question.0,role.to_string()),|(uid,mp,org,committee,minister)|(uid,mp,org,committee,minister))?;
        let mut res = vec![];
        for (uid,mp,org,committee,minister) in elements {
            let decoded = {
                if let Some(uid) = uid { PersonID::User(uid) }
                else if let Some(mp) = mp { // we may want to cache this for performance.
                    if let Some(mp_id) = MPId::read_from_database(conn,mp)? {
                        PersonID::MP(mp_id)
                    } else {
                        eprintln!("Missing mp {} for question {} role {}",mp,question,role);
                        continue;
                    }
                } else if let Some(committee) = committee { // we may want to cache this for performance.
                    if let Some(committee_id) = CommitteeId::read_from_database(conn,committee)? {
                        PersonID::Committee(committee_id)
                    } else {
                        eprintln!("Missing committee {} for question {} role {}",committee,question,role);
                        continue;
                    }
                } else if let Some(org) = org { // we may want to cache this for performance.
                    if let Some(org_id) = conn.exec_first::<String,_,_>("select OrgID from Organisations where id=?",(org,))? {
                        PersonID::Organisation(org_id)
                    } else {
                        eprintln!("Missing organisation {} for question {} role {}",org,question,role);
                        continue;
                    }
                } else if let Some(minister) = minister { // we may want to cache this for performance.
                    if let Some(minister_id) = MinisterId::read_from_database(conn,minister)? {
                        PersonID::Minister(minister_id)
                    } else {
                        eprintln!("Missing minister {} for question {} role {}",minister,question,role);
                        continue;
                    }
                } else {
                    eprintln!("Blank person for question {} role {}",question,role);
                    continue;
                }
            };
            res.push(decoded);
        }
        Ok(res)
    }
    /// Add the given people to a given question.
    fn add_for_question(conn:&mut impl Queryable,role:char,question:QuestionID,people:HashSet<&PersonID>) -> Result<(),QuestionError> {
        let mut references : Vec<(Option<UserID>,Option<MPIndexInDatabaseTable>,Option<OrgIndexInDatabaseTable>,Option<CommitteeIndexInDatabaseTable>,Option<MinisterIndexInDatabaseTable>)> = vec![];
        for &person in people.iter() {
            match person {
                PersonID::User(uid) => {
                    let user_id = get_user_id(uid,QuestionError::NoSuchUser,QuestionError::InternalError,conn)?;
                    references.push((Some(user_id),None,None,None,None));
                }
                PersonID::MP(mp_id) => {
                    let id = mp_id.get_id_from_database(conn).map_err(internal_error)?;
                    references.push((None,Some(id),None,None,None));
                }
                PersonID::Organisation(org_name) => {
                    let id = get_org_id_from_database(org_name,conn).map_err(internal_error)?;
                    references.push((None,None,Some(id),None,None));
                }
                PersonID::Committee(committee_id) => {
                    let id = committee_id.get_id_from_database(conn).map_err(internal_error)?;
                    references.push((None,None,None,Some(id),None));
                }
                PersonID::Minister(minister_id) => {
                    let id = minister_id.get_id_from_database(conn).map_err(internal_error)?;
                    references.push((None,None,None,None,Some(id)));
                }
            }
        }
        let role = role.to_string();
        conn.exec_batch("insert into PersonForQuestion (QuestionId,ROLE,UserId,MP,ORG,Committee,Minister) values (?,?,?,?,?,?,?)",references.into_iter().map(|(user_id,mp,org,committee,minister)|(question.0,&role,user_id,mp,org,committee,minister))).map_err(internal_error)?;
        Ok(())
    }

    fn check_sane(&self,conn:&mut impl Queryable) -> Result<(),QuestionError> {
        match self {
            PersonID::User(uid) => {
                if !user_exists(uid,conn).map_err(internal_error)? { return Err(QuestionError::InvalidUserSpecified) }
            }
            PersonID::MP(mp_id) => {
                let mps = MPSpec::get().map_err(internal_error)?;
                if !mps.contains(mp_id) { return Err(QuestionError::InvalidMP) }
            }
            PersonID::Organisation(org) => {
                if org.len()>50 { return Err(QuestionError::OrganisationNameTooLong); }
            }
            PersonID::Committee(committee_id) => {
                let mps = COMMITTEES.get_interpreted().map_err(internal_error)?;
                if !mps.iter().any(|ci|ci.jurisdiction==committee_id.jurisdiction && ci.name==committee_id.name) { return Err(QuestionError::InvalidCommittee) }
            }
            PersonID::Minister(minister_id) => {
                let mps = MPSpec::get().map_err(internal_error)?;
                if !mps.mps.iter().any(|mi|mi.is_in_role(minister_id)) { return Err(QuestionError::InvalidMinister) }
            }
        }
        Ok(())
    }
}


fn is_false(x:&bool) -> bool { !*x }
fn is_not_flagged(x:&CensorshipStatus) -> bool { *x == CensorshipStatus::NotFlagged }

#[derive(Serialize,Deserialize,Debug,Clone)]
/// This contains the fields for the question that can be changed.
///
/// It is used in two different ways with slightly different semantics
///  * To store the current state of a question. A blank field means that field has never been set.
///  * To contain an update. A blank field means that the field should not be changed.
///
/// There is generally no way to remove entries, except for the FLAG QUESTION command.
///
/// There is no need for timestamps to be stored by the server for all intermediate
/// modifications, because you can get a full history by following the linked list of
/// version numbers in the bulletin board, starting from the current version (a reference
/// to a Bulletin board node) and continuing back to the the initial new_question link.
pub struct QuestionNonDefiningFields {
    /// Validity: character length
    /// Permission: must be from the question-writer
    /// Merge rule: Allow append from question-writer.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub background : Option<String>,
    /// Validity: must be an MP or a user. (If a user is associated with an MP then tag for the MP.)
    /// Permission: defined by who_should_ask_the_question_permissions
    /// Merge rule: Eliminate duplicates (including with values already present). If the total number of values doesn't exceed the limit, accept. If the limit has already been exceeded, reject. If it hasn't, but would if this update was accepted, send a merge request back to the client (pick at most m out of the n you tried to submit...).  Note that this might cause cascading merges that need to be manually resolved, but that's less trouble than allowing locks.
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub mp_who_should_ask_the_question : Vec<PersonID>,
    /// Permission: must be from the question-writer
    /// Merge rule: overwrite, unless it's 'NoChange'
    #[serde(skip_serializing_if = "Permissions::is_no_change",default)]
    pub who_should_ask_the_question_permissions : Permissions,
    /// Validity : It's either an MP or a user.
    /// Permission: Defined by who_should_answer_the_question_permissions
    /// Merge rule : same as mp_who_should_ask_the_question
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub entity_who_should_answer_the_question : Vec<PersonID>,
    /// Permission: must be from the question-writer
    /// Merge rule: overwrite, unless it's 'NoChange'
    #[serde(skip_serializing_if = "Permissions::is_no_change",default)]
    pub who_should_answer_the_question_permissions : Permissions,
    /// Validity : character length; answerer must match the sig.
    /// Permission Must be from MP.
    /// Q: Can an entity_who_should_answer_the_question ... answer the question?
    /// VT: Counterintuitively, No. I am assuming that public authorities won't join the system, only MPs. And then it seems only fair to let other
    /// MPs answer, even if they are not the person tagged in the system.
    /// Merge rule : just add. No problems with multiple answers from different people. Or even multiple answers from the same person, e.g. MP day 1: "I will ask that for you." Day 3: "They said 42."
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub answers : Vec<QuestionAnswer>,
    /// Permission: must be from the question-writer
    /// Merge rule : may be changed from false to true.
    #[serde(skip_serializing_if = "is_false",default)]
    pub answer_accepted : bool,
    /// Validity : domain must be aph.gov.au, parliament.vic.gov.au, etc. (preloaded permit-list - note that url sanitation is nontrivial).
    /// Permission: anyone can add
    /// Merge rule : same as mp_who_should_ask_the_question
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub hansard_link : Vec<HansardLink>,
    /// Validity: must be a pre-existing Question-Id
    /// Permissions: Only the question-writer can write a followup.
    /// Merge rule:  Reject updates unless currently blank.  VT: Agree. Let's just have one at a time. People can make a linear chain. (Twitter actually allows a tree, and it's a complete pain. Lines are better.)
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub is_followup_to : Option<QuestionID>,
}

pub type AnswerId = usize;

#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct QuestionAnswer {
    /// must be a MP. Set by server to whoever signed the message - client should not set this when sending to server.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub answered_by : Option<UserUID>,
    pub mp : MPId,
    pub answer : String,
    /// set by server - client should not set this when sending to server.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub timestamp : Option<Timestamp>,
    /// Whether this answer has been censored/flagged/etc.
    /// set by server - client should not set this when sending to server.
    #[serde(skip_serializing_if = "is_not_flagged",default)]
    pub censorship_status : CensorshipStatus,
    /// The bulletin board identifier associated with this answer. Used for flagging/censorship.
    /// set by server - client should not set this when sending to server.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub version : Option<HashValue>,
}

impl QuestionAnswer {
    /// Get the answers to a question.
    fn get_for_question(conn:&mut impl Queryable,question:QuestionID) -> mysql::Result<Vec<QuestionAnswer>> {
        let entries : Vec<(UserUID,MPIndexInDatabaseTable,Timestamp,String,CensorshipStatus,mysql::Value)> = conn.exec("SELECT USERS.UID,mp,timestamp,answer,CensorshipStatus,version from Answer inner join USERS ON Answer.AuthorId=USERS.id where QuestionId=? order by timestamp",(&question.0,))?;
        let mut res : Vec<QuestionAnswer> = vec![];
        for (answered_by,mp,timestamp,answer,censorship_status,version) in entries {
            if let Some(mp_id) = MPId::read_from_database(conn,mp)? {
                res.push(QuestionAnswer{answered_by:Some(answered_by),mp:mp_id,answer,timestamp: Some(timestamp),censorship_status,version:opt_hash_from_value(version) })
            } else {
                eprintln!("Missing mp {} in question {} answer",mp,question);
            }
        }
        Ok(res)
    }
    /// Add a given answer to the database.
    fn add_for_question(&self,conn:&mut impl Queryable,question:QuestionID,timestamp:Timestamp,uid:&UserUID,version:HashValue) -> Result<(),QuestionError> {
        let mp = self.mp.get_id_from_database(conn).map_err(internal_error)?;
        let user_id = get_user_id(uid,QuestionError::NoSuchUser,QuestionError::InternalError,conn)?;
        conn.exec_drop("insert into Answer (QuestionId,AuthorId,mp,timestamp,answer,version) values (?,?,?,?,?,?)",(&question.0,user_id,mp,timestamp,&self.answer,&version.0)).map_err(internal_error)?;
        Ok(())
    }

    fn check_legal(&self,conn:&mut impl Queryable,uid:&UserUID) -> Result<(),QuestionError> {
        if self.answer.len()>MAX_ANSWER_LENGTH { return Err(QuestionError::AnswerTooLong); }
        if self.answered_by.is_some() || self.timestamp.is_some() || self.censorship_status!=CensorshipStatus::NotFlagged || self.version.is_some() { return Err(QuestionError::AnswerContainsUndesiredFields); }
        let mps = MPSpec::get().map_err(internal_error)?;
        if let Some(mp) = mps.find(&self.mp) {
            let badges : usize = conn.exec_first("SELECT COUNT(badge) from BADGES inner join USERS ON BADGES.user_id=USERS.id where USERS.UID=? and BADGES.what=? and (BADGES.badge='MP' || BADGES.badge='MPStaff')",(uid,mp.badge_name())).map_err(internal_error)?.ok_or_else(||QuestionError::InternalError)?;
            if badges==0 { return Err(QuestionError::UserDoesNotHaveCorrectMPBadge); }
        } else  { return Err(QuestionError::InvalidMP); }
        Ok(())
    }

}
///  domain must be aph.gov.au, parliament.vic.gov.au, etc. (preloaded permit-list - note that url sanitation is nontrivial).
#[derive(Serialize,Deserialize,Debug,Clone,Eq, PartialEq,Hash)]
pub struct HansardLink {
    pub url : String, // Should this be more structured?
}

/// A list of all hosts that can be linked to by the Hansard Link.
const ALLOWED_HOSTS: [&'static str; 9] = ["www.aph.gov.au",
    "parliament.act.gov.au",
    "parliament.nsw.gov.au",
    "parliament.nt.gov.au",
    "parliament.qld.gov.au",
    "parliament.sa.gov.au",
    "parliament.tas.gov.au",
    "parliament.vic.gov.au",
    "parliament.wa.gov.au"
];

impl HansardLink {
    /// Return OK if this seems like a safe URL.
    fn check_ok(&self) -> Result<(),QuestionError> {
        let url = Url::parse(&self.url).map_err(|_|QuestionError::HansardLinkIsNotURL)?;
        if let Some(Host::Domain(host)) = url.host() {
            if !ALLOWED_HOSTS.iter().any(|&h|h==host) { return Err(QuestionError::HansardLinkIsNotAllowed)}
            Ok(())
        } else { return Err(QuestionError::HansardLinkIsNotURL) }
    }

    /// Get the links for a question.
    fn get_for_question(conn:&mut impl Queryable,question:QuestionID) -> mysql::Result<Vec<HansardLink>> {
        conn.exec_map("SELECT url from HansardLink where QuestionId=?",(&question.0,),|url|HansardLink{url})
    }
    /// Add the given links to a given question.
    fn add_for_question(conn:&mut impl Queryable,question:QuestionID,links:&[&HansardLink]) -> mysql::Result<()> {
        conn.exec_batch("insert into HansardLink (QuestionId,url) values (?,?)",links.iter().map(|link|(question.0,&link.url)))
    }

}

/// Any modification to the question database will have to
///  * Check that the database version is the expected version.
///  * modify the version and last updated timestamp.
///  * Change the censorship status if in state 'Allowed' to state 'StructureChanged'
/// This does these common tasks.
pub(crate) async fn modify_question_database_version_and_time(transaction:&mut Transaction<'_>,question_id:QuestionID,new_version:LastQuestionUpdate,expecting_version:Option<LastQuestionUpdate>,timestamp:Timestamp) -> Result<(),QuestionError>{
    if let Some(current_version) = transaction.exec_first::<mysql::Value,_,_>("select Version from QUESTIONS where QuestionID=?",(question_id.0,)).map_err(internal_error)? {
        let expected : mysql::Value = expecting_version.map(|v|v.0).into();
        if expected!=current_version { return Err(QuestionError::LastUpdateIsNotCurrent); }
    } else { return Err(QuestionError::QuestionDoesNotExist); }
    transaction.exec_drop("update QUESTIONS set LastModifiedTimestamp=?,Version=?,CensorshipStatus = IF(CensorshipStatus='Allowed','StructureChanged',CensorshipStatus) where QuestionID=?", (timestamp,new_version.0,question_id.0)).map_err(internal_error)?;
    Ok(())
}

impl QuestionNonDefiningFields {
    /// Check that all the fields are legal to modify.
    // A database connection may be retrieved many times in a rather wasteful manner.
    pub async fn check_legal(&self,is_creator:bool,user:&UserUID,existing:Option<&QuestionInfo>) -> Result<(),QuestionError> {
        if let Some(background) = &self.background {
            if background.len()>MAX_BACKGROUND_LENGTH { return Err(QuestionError::BackgroundTooLong); }
            if !is_creator { return Err(QuestionError::OnlyAuthorCanChangeBackground); }
            if !existing.and_then(|info|info.non_defining.background.as_ref()).map(|e|background.starts_with(e)).unwrap_or(true) { return Err(QuestionError::CanOnlyExtendBackground); }
        }
        for a in &self.answers {
            let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
            a.check_legal(&mut conn,user)?;
        }
//        if (!self.answers.is_empty()) && !is_user_mp_or_staffer(user).await.map_err(internal_error)?  { return Err(QuestionError::OnlyMPCanAnswerQuestion); }
        if let Some(follow_up_to) = self.is_followup_to {
            // check it is a valid question
            let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
            if conn.exec_first::<mysql::Value,_,_>("select QuestionID from QUESTIONS where QuestionID=?",(follow_up_to.0,)).map_err(internal_error)?.is_none() { return Err(QuestionError::FollowUpIsNotAValidQuestion); }
            // check that it is not already set
            if let Some(existing) = existing {
                if existing.non_defining.is_followup_to.is_some() { return Err(QuestionError::FollowUpIsAlreadySet); }
            }
        }
        if !self.who_should_ask_the_question_permissions.is_no_change() {
            if !is_creator { return Err(QuestionError::OnlyAuthorCanChangePermissions); }
        }
        if !self.who_should_answer_the_question_permissions.is_no_change() {
            if !is_creator { return Err(QuestionError::OnlyAuthorCanChangePermissions); }
        }
        if !self.mp_who_should_ask_the_question.is_empty() {
            let existing = existing.iter().flat_map(|e|e.non_defining.mp_who_should_ask_the_question.iter()).collect::<HashSet<_>>();
            let extra : HashSet<_> = self.mp_who_should_ask_the_question.iter().filter(|m|!existing.contains(m)).collect();
            if existing.len()+extra.len() > MAX_MPS_WHO_SHOULD_ASK_THE_QUESTION { return Err(QuestionError::TooLongListOfPeopleAskingQuestion);}
            let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
            for e in extra { e.check_sane(&mut conn)? }
        }
        if !self.entity_who_should_answer_the_question.is_empty() {
            let existing = existing.iter().flat_map(|e|e.non_defining.entity_who_should_answer_the_question.iter()).collect::<HashSet<_>>();
            let extra : HashSet<_> = self.entity_who_should_answer_the_question.iter().filter(|m|!existing.contains(m)).collect();
            if existing.len()+extra.len() > MAX_MPS_WHO_SHOULD_ANSWER_THE_QUESTION { return Err(QuestionError::TooLongListOfPeopleAnsweringQuestion);}
            let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
            for e in extra { e.check_sane(&mut conn)? }
        }
        if self.answer_accepted {
            if let Some(existing) = existing {
                if !existing.non_defining.answer_accepted {
                    if !is_creator { return Err(QuestionError::OnlyAuthorCanAcceptAnswer); }
                    if !existing.non_defining.answers.is_empty() { return Err(QuestionError::CantAcceptAnswerIfNonePresent)}
                }
            } else { return Err(QuestionError::CantAcceptAnswerIfNonePresent)}
        }

        if !self.hansard_link.is_empty() {
            for link in &self.hansard_link {
                link.check_ok()?;
            }
        }
        Ok(())
    }


    /// Add a simple question to the database, without any extra information yet.
    async fn modify_database(&self,transaction:&mut Transaction<'_>,question_id:QuestionID,new_version:LastQuestionUpdate,expecting_version:Option<LastQuestionUpdate>,timestamp:Timestamp,uid:&UserUID) -> Result<(),QuestionError> {
        println!("modify_database with question non-defining fields {:?}",self);
        modify_question_database_version_and_time(transaction,question_id,new_version,expecting_version,timestamp).await?;
        if let Some(background) = &self.background {
            // println!("Setting background to {}",background);
            transaction.exec_drop("update QUESTIONS set Background=? where QuestionID=?", (background,question_id.0)).map_err(internal_error)?;
        }
        if !self.who_should_ask_the_question_permissions.is_no_change() {
            transaction.exec_drop("update QUESTIONS set CanOthersSetWhoShouldAsk=? where QuestionID=?", (self.who_should_ask_the_question_permissions==Permissions::Others,question_id.0)).map_err(internal_error)?;
        }
        if !self.who_should_answer_the_question_permissions.is_no_change() {
            transaction.exec_drop("update QUESTIONS set CanOthersSetWhoShouldAnswer=? where QuestionID=?", (self.who_should_answer_the_question_permissions==Permissions::Others,question_id.0)).map_err(internal_error)?;
        }
        if !self.mp_who_should_ask_the_question.is_empty() {
            let existing = PersonID::get_for_question(transaction,'Q',question_id).map_err(internal_error)?.into_iter().collect::<HashSet<_>>();
            let extra : HashSet<_> = self.mp_who_should_ask_the_question.iter().filter(|&m|!existing.contains(m)).collect();
            if existing.len()+extra.len() > MAX_MPS_WHO_SHOULD_ASK_THE_QUESTION { return Err(QuestionError::TooLongListOfPeopleAskingQuestion);}
            PersonID::add_for_question(transaction,'Q',question_id,extra)?;
        }
        if !self.entity_who_should_answer_the_question.is_empty() {
            let existing = PersonID::get_for_question(transaction,'A',question_id).map_err(internal_error)?.into_iter().collect::<HashSet<_>>();
            let extra : HashSet<_> = self.entity_who_should_answer_the_question.iter().filter(|&m|!existing.contains(m)).collect();
            if existing.len()+extra.len() > MAX_MPS_WHO_SHOULD_ASK_THE_QUESTION { return Err(QuestionError::TooLongListOfPeopleAskingQuestion);}
            PersonID::add_for_question(transaction,'A',question_id,extra)?;
        }
        if let Some(follow_up_to) = self.is_followup_to {
            transaction.exec_drop("update QUESTIONS set FollowUpTo=? where QuestionID=?", (follow_up_to.0,question_id.0)).map_err(internal_error)?;
        }
        for a in &self.answers {
            a.add_for_question(transaction,question_id,timestamp,uid,new_version)?;
        }
        if self.answer_accepted {
            // There could be optimization here checking if it was already set.
            transaction.exec_drop("update QUESTIONS set AnswerAccepted=true where QuestionID=?", (question_id.0,)).map_err(internal_error)?;
        }
        if !self.hansard_link.is_empty() {
            // remove duplicates
            let existing = HansardLink::get_for_question(transaction,question_id).map_err(internal_error)?.into_iter().collect::<HashSet<_>>();
            let extra : Vec<_> = self.hansard_link.iter().filter(|&m|!existing.contains(m)).collect();
            if !extra.is_empty() {
                HansardLink::add_for_question(transaction,question_id,&extra).map_err(internal_error)?;
            }
        }
        Ok(())
    }

    /// Find questions that have matching metadata. Returns a map of questionID to number of matches.
    /// NOTE THIS DOES NOT SCALE WELL. This is a temporary attempt until a long term approach is produced.
    pub async fn find_similar_metadata(&self) -> Result<HashMap<QuestionID,u32>,QuestionError> {
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        let mut res : HashMap<QuestionID,u32> = HashMap::default();
        for p in &self.mp_who_should_ask_the_question {
            for q in QuestionNonDefiningFields::find_questions_by_person_in_role(&mut conn, "Q", p).map_err(internal_error)? {
                *res.entry(q).or_insert(0)+=1;
            }
        }
        for p in &self.entity_who_should_answer_the_question {
            for q in QuestionNonDefiningFields::find_questions_by_person_in_role(&mut conn, "A", p).map_err(internal_error)? {
                *res.entry(q).or_insert(0)+=1;
            }
        }
        Ok(res)
    }

    /// get questions that have a given person in a given role (questioner or answerer)
    fn find_questions_by_person_in_role(conn:&mut impl Queryable,role:&str,person:&PersonID) -> mysql::Result<Vec<QuestionID>> {
        match person {
            PersonID::User(who) => conn.exec_map("select QuestionId from PersonForQuestion inner join USERS ON PersonForQuestion.UserId=USERS.id where ROLE=? and USERS.UID=?",(role,who),|(v,)|hash_from_value(v)),
            PersonID::MP(who) => {
                if let Some(id) = who.get_id_from_database_if_there(conn)? {
                    conn.exec_map("select QuestionId from PersonForQuestion where ROLE=? and MP=?",(role,id),|(v,)|hash_from_value(v))
                } else {
                    Ok(vec![])
                }
            },
            PersonID::Organisation(who) => {
                if let Some(id) = conn.exec_first::<usize,_,_>("select id from Organisations where OrgID=?",(who,))? {
                    conn.exec_map("select QuestionId from PersonForQuestion where ROLE=? and ORG=?",(role,id),|(v,)|hash_from_value(v))
                } else {
                    Ok(vec![])
                }
            },
            PersonID::Committee(who) => {
                if let Some(id) = who.get_id_from_database_if_there(conn)? {
                    //println!("Found committee with id {}",id);
                    conn.exec_map("select QuestionId from PersonForQuestion where ROLE=? and Committee=?",(role,id),|(v,)|hash_from_value(v))
                } else {
                    //println!("Did not find committee with id {:?}",who);
                    Ok(vec![])
                }
            },
            PersonID::Minister(who) => {
                if let Some(id) = who.get_id_from_database_if_there(conn)? {
                    conn.exec_map("select QuestionId from PersonForQuestion where ROLE=? and Minister=?",(role,id),|(v,)|hash_from_value(v))
                } else {
                    Ok(vec![])
                }
            },
        }
    }
}

/*************************************************************************
                       NEW QUESTION
 *************************************************************************/



/// A new question request starts a new question. It is a command sent to the server.
///
/// The question defining fields (question text, and sender) are augmented by the server with
/// a timestamp. A hash is then created, defining the unique QuestionID which will henceforth
/// be used to identify the question. This hash is *not* a bulletin-board hash, although the
/// same representation (hex string) is used. See QuestionDefiningFields for how the hash is defined.
///
/// The question database then has the question added, checking in the process that no identical
/// question by the same person was signed in the prior 24 hours.
///
/// The NewQuestion command data structure sent by the user, along with
/// the timestamp and the QuestionID, is sent to the bulletin board as a NewQuestionCommandPostedToBulletinBoard. This returns a
/// hash, which is called the LastQuestionUpdate. It is used as a track of
/// the current state of the question, a little like a git commit hash.
///
/// The server database is then updated, storing the lastQuestionUpdate and any non-defining fields.
///
/// The response is the QuestionID and LastQuestionUpdate. (stored in NewQuestionCommandResponse) This should be signed.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct NewQuestionCommand {
    /// The text of the question
    pub question_text : String,

    // additional fields that can be done at time of question, or may be done later.
    #[serde(flatten)]
    pub non_defining_fields : QuestionNonDefiningFields,
}

/// The message posted to the bulletin board
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct NewQuestionCommandPostedToBulletinBoard {
    pub command : ClientSigned<NewQuestionCommand>,
    pub timestamp : Timestamp,
    pub question_id : QuestionID,
}

/// Successful return from posting a new question.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct NewQuestionCommandResponse {
    pub question_id : QuestionID,
    pub version : LastQuestionUpdate,
}

pub(crate) fn internal_error<T:Debug>(error:T) -> QuestionError {
    eprintln!("Internal error {:?}",error);
    QuestionError::InternalError
}
pub(crate) fn bulletin_board_error(error:anyhow::Error) -> QuestionError {
    eprintln!("Bulletin Board error {:?}",error);
    QuestionError::CouldNotWriteToBulletinBoard
}

impl NewQuestionCommand {

    /// API function to add a question to the server
    pub async fn add_question(question:&ClientSigned<NewQuestionCommand>) -> Result<NewQuestionCommandResponse,QuestionError> {
        if question.parsed.question_text.len()>MAX_QUESTION_LENGTH { return Err(QuestionError::QuestionTooLong); }
        if question.parsed.question_text.len()<MIN_QUESTION_LENGTH { return Err(QuestionError::QuestionTooShort); }
        question.parsed.non_defining_fields.check_legal(true,&question.signed_message.user,None).await?;
        let timestamp = timestamp_now().map_err(internal_error)?;
        let defining = QuestionDefiningFields{
            author: question.signed_message.user.to_string(),
            question_text: question.parsed.question_text.to_string(),
            timestamp
        };
        let question_id = defining.compute_hash();
        let for_bb = NewQuestionCommandPostedToBulletinBoard {
            command: question.clone(),
            timestamp,
            question_id
        };
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        let user_id = get_user_id(&question.signed_message.user,QuestionError::NoSuchUser,QuestionError::InternalError,&mut conn)?; // user_id doesn't change - doesn't need to be in transaction. It is conceivable that someone changes their UID, and simultaneously submits a new question, and simultaneously someone else claims the old UID, in which case the question would be assigned to the new claimer. This is not a very credible scenario.
        if let Some(existing_timestamp) = conn.exec_first::<Timestamp,_,_>("select CreatedTimestamp from QUESTIONS where Question=? and CreatedById=? ORDER BY CreatedTimestamp DESC",(&question.parsed.question_text,user_id)).map_err(internal_error)? {
            if existing_timestamp+24*60*60 > timestamp { return Err(QuestionError::YouJustAskedThatQuestion)}
        } // this is first checked outside of the transaction. This will catch most of the duplicates before assigned to the bulletin board.
        let version = LogInBulletinBoard::NewQuestion(for_bb).log_in_bulletin_board().await.map_err(bulletin_board_error)?;
        let mut transaction = conn.start_transaction(TxOpts::default()).map_err(internal_error)?;
        if let Some(existing_timestamp) = transaction.exec_first::<Timestamp,_,_>("select CreatedTimestamp from QUESTIONS where Question=? and CreatedById=? ORDER BY CreatedTimestamp DESC",(&question.parsed.question_text,user_id)).map_err(internal_error)? {
            if existing_timestamp+24*60*60 > timestamp { return Err(QuestionError::YouJustAskedThatQuestion)}
        } // this is repeated inside of the transaction in case there is a delay with the bulletin board and the same question is submitted concurrently multiple times.
        transaction.exec_drop("insert into QUESTIONS (QuestionID,Question,CreatedTimestamp,LastModifiedTimestamp,CreatedById,CanOthersSetWhoShouldAsk,CanOthersSetWhoShouldAnswer,AnswerAccepted) values (?,?,?,?,?,FALSE,FALSE,FALSE)", (question_id.0,&question.parsed.question_text,timestamp,timestamp,user_id)).map_err(internal_error)?;
        question.parsed.non_defining_fields.modify_database(&mut transaction,question_id,version,None,timestamp,&question.signed_message.user).await?;
        transaction.commit().map_err(internal_error)?;
        add_question_to_comparison_database(&question.parsed.question_text,question_id).await.map_err(internal_error)?;
        Ok(NewQuestionCommandResponse{ question_id, version })
    }
}




/*************************************************************************
                       QUERY INFO ABOUT A QUESTION
 *************************************************************************/





/// Information about a question.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct QuestionInfo {
    #[serde(flatten)]
    pub(crate) defining : QuestionDefiningFields,
    #[serde(flatten)]
    pub(crate) non_defining : QuestionNonDefiningFields,
    pub(crate) question_id : QuestionID,
    pub(crate) version : LastQuestionUpdate,
    pub(crate) last_modified : Timestamp,
    /// The total number of times this question has been voted on.
    pub(crate) total_votes : u32,
    /// upvotes-downvotes.
    pub(crate) net_votes : i32,
    pub(crate) censorship_status : CensorshipStatus,
}

/// Convert v into a HashValue where you know v will be a 32 byte value
/// TODO make original functions in bulletin board code public.
pub fn hash_from_value(v:mysql::Value) -> HashValue {
    match v {
        mysql::Value::Bytes(b) if b.len()==32 => HashValue(b.try_into().unwrap()),
        // Value::NULL => {}
        _ => { panic!("Not a 32 byte vector"); }
    }
}

/// Convert v into a HashValue where you know v will be a 32 byte value or null
fn opt_hash_from_value(v:mysql::Value) -> Option<HashValue> {
    match v {
        mysql::Value::Bytes(b) if b.len()==32 => Some(HashValue(b.try_into().unwrap())),
        mysql::Value::NULL => None,
        _ => { panic!("Not a 32 byte vector"); }
    }
}



impl QuestionInfo {
    /// Get information about a question from the database.
    pub async fn lookup(question_id:QuestionID) -> Result<Option<QuestionInfo>,QuestionError> {
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        // mysql crate only handles tuples up to 12 elements. We have 13.
        // Use less pleasant HList another way to handle wide rows is to use HList (requires `mysql_common/frunk` feature)
        use mysql_common::frunk::{HList, hlist_pat};
        type RowType = HList!(String, Timestamp, Timestamp, mysql::Value, String, Option<String>, bool, bool, bool,  mysql::Value, CensorshipStatus,u32,i32);
        if let Some(hlist_pat![question_text,timestamp,last_modified,version,author,background,who_should_ask_the_question_permissions,who_should_answer_the_question_permissions,answer_accepted,is_followup_to,censorship_status,total_votes,net_votes]) = conn.exec_first::<RowType,_,_>("SELECT Question,CreatedTimestamp,LastModifiedTimestamp,Version,USERS.UID,Background,CanOthersSetWhoShouldAsk,CanOthersSetWhoShouldAnswer,AnswerAccepted,FollowUpTo,CensorshipStatus,TotalVotes,NetVotes from QUESTIONS inner join USERS ON CreatedById=USERS.id where QuestionID=?",(question_id.0,)).map_err(internal_error)? {
            if censorship_status==CensorshipStatus::Censored { return Err(QuestionError::Censored); }
            match opt_hash_from_value(version) {
                None => Ok(None),
                Some(version) => {
                    Ok(Some(QuestionInfo{
                        defining: QuestionDefiningFields { author, question_text, timestamp },
                        non_defining: QuestionNonDefiningFields {
                            background, // : convert_null_allowed_value_to_option(background),
                            mp_who_should_ask_the_question : PersonID::get_for_question(&mut conn,'Q',question_id).map_err(internal_error)?,
                            who_should_ask_the_question_permissions: if who_should_ask_the_question_permissions { Permissions::Others } else { Permissions::WriterOnly } ,
                            entity_who_should_answer_the_question: PersonID::get_for_question(&mut conn,'A',question_id).map_err(internal_error)?,
                            who_should_answer_the_question_permissions: if who_should_answer_the_question_permissions { Permissions::Others } else { Permissions::WriterOnly } ,
                            answers: QuestionAnswer::get_for_question(&mut conn,question_id).map_err(internal_error)?,
                            answer_accepted,
                            hansard_link: HansardLink::get_for_question(&mut conn,question_id).map_err(internal_error)?,
                            is_followup_to : opt_hash_from_value(is_followup_to),
                        },
                        question_id,
                        version,
                        last_modified,
                        total_votes,
                        net_votes,
                        censorship_status,
                    }))
                }
            }
        } else { Ok(None) }
    }

    /// This should be replaced by something that gets a smaller list.
    pub async fn get_list_of_all_questions() -> mysql::Result<Vec<QuestionID>> {
        let mut conn = get_rta_database_connection().await?;
        let elements : Vec<QuestionID> = conn.exec_map("SELECT QuestionID from QUESTIONS ORDER BY LastModifiedTimestamp DESC",(),|(v,)|hash_from_value(v))?;
        Ok(elements)
    }

    /// Get all questions from a particular user.
    pub async fn get_questions_created_by_user(uid:&str) -> mysql::Result<Vec<QuestionID>> {
        let mut conn = get_rta_database_connection().await?;
        let elements : Vec<QuestionID> = conn.exec_map("SELECT QuestionID from QUESTIONS inner join USERS ON QUESTIONS.CreatedById=USERS.id where USERS.UID=? ORDER BY LastModifiedTimestamp DESC",(uid,),|(v,)|hash_from_value(v))?;
        Ok(elements)
    }

}





/*************************************************************************
                        EDIT A QUESTION
 *************************************************************************/






/// Edit question. This takes an existing question, and changes some of the
/// non-defining fields.
///
/// A version is provided. If it does not match the actual last provided update,
/// it will refuse to do anything. This stops multiple simultaneous edits.
///
/// The edits are updates to the non-defining fields. See [QuestionNonDefiningFields] for
/// details about how such edits change. Generally, an empty field will cause no changes.
///
/// When the server executes a command, it will first check the database to see that
/// the question exists, the version is current, and there are no obvious errors in the edits.
/// If ok, it will send the command to the bulletin board as a [EditQuestionCommandPostedToBulletinBoard].
/// The resulting hash will become the new version number for the question.
///
/// The server will then update the database with the changes in [self.edits],
/// the new version, and the last modified timestamp.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct EditQuestionCommand {
    /// The hashvalue that defines the unique ID of the question to be modified
    pub question_id : QuestionID,
    /// The hash value defining the last update done to the question. This is checked to prevent multiple edits.
    /// TODO Should it be an option? Maybe you don't care if there are clashes?
    pub version : LastQuestionUpdate,
    /// the actual work... This contains *updates* to be added to the non-defining fields. Empty fields are to be left unchanged.
    #[serde(flatten)]
    pub edits : QuestionNonDefiningFields,
}

/// The structure posted to the bulletin board in response to an EditQuestionCommand.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct EditQuestionCommandPostedToBulletinBoard {
    pub command : ClientSigned<EditQuestionCommand>,
    pub timestamp : Timestamp,
    /// This will be a link to the prior node in the database. This will be a duplicate of [self.command.parsed.version], but easier to access, and future proof against a change in design where version is not included.
    pub prior : LastQuestionUpdate,
}


impl EditQuestionCommand {

    /// Try to perform the edit.
    /// If success, return the new last edit.
    pub async fn edit(command:&ClientSigned<EditQuestionCommand>) -> Result<LastQuestionUpdate,QuestionError> {
        let question_info = QuestionInfo::lookup(command.parsed.question_id).await?.ok_or_else(||QuestionError::QuestionDoesNotExist)?;
        if question_info.version!=command.parsed.version { return Err(QuestionError::LastUpdateIsNotCurrent); }
        let is_creator = question_info.defining.author == command.signed_message.user;
        command.parsed.edits.check_legal(is_creator,&command.signed_message.user,Some(&question_info)).await?;
        let timestamp = timestamp_now().map_err(internal_error)?;
        let for_bb = EditQuestionCommandPostedToBulletinBoard {
            command: command.clone(),
            timestamp,
            prior : command.parsed.version,
        };
        let version = LogInBulletinBoard::EditQuestion(for_bb).log_in_bulletin_board().await.map_err(bulletin_board_error)?;
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        let mut transaction = conn.start_transaction(TxOpts::default()).map_err(internal_error)?;
        command.parsed.edits.modify_database(&mut transaction,command.parsed.question_id,version,Some(command.parsed.version),timestamp,&command.signed_message.user).await?;
        transaction.commit().map_err(internal_error)?;
        Ok(version)
    }
}

/// Vote on a question
/// This a placeholder plain-text voting while the crypto is being worked out.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct PlainTextVoteOnQuestionCommand {
    /// The hashvalue that defines the unique ID of the question to be modified
    pub question_id: QuestionID,
    /// If true, an up vote. If false, a down vote.
    pub up: bool,
}

/// The structure posted to the bulletin board in response to a PlainTextVoteOnQuestionCommand.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct PlainTextVoteOnQuestionCommandPostedToBulletinBoard {
    pub command : ClientSigned<PlainTextVoteOnQuestionCommand>,
    pub timestamp : Timestamp,
    /// This will be a link to the prior node in the database.
    pub prior : LastQuestionUpdate,
}


impl PlainTextVoteOnQuestionCommand {
    /// TODO should votes change the version?
    pub async fn vote(command:&ClientSigned<PlainTextVoteOnQuestionCommand>) -> Result<LastQuestionUpdate,QuestionError> {
        println!("Vote {} for {} from {}",if command.parsed.up {"Up"} else {"Down"},command.parsed.question_id,command.signed_message.user);
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        let mut transaction = conn.start_transaction(TxOpts::default()).map_err(internal_error)?;
        let user_id = get_user_id(&command.signed_message.user,QuestionError::NoSuchUser,QuestionError::InternalError,&mut transaction)?;
        let times_voted = transaction.exec_first::<u32, _, _>("select count(*) from HAS_VOTED where QuestionId=? and VoterId=?", (command.parsed.question_id.0, user_id)).map_err(internal_error)?.ok_or(QuestionError::InternalError)?;
        if times_voted > 0 { return Err(QuestionError::AlreadyVoted) }
        let (version,) = transaction.exec_first("SELECT Version from QUESTIONS where QuestionID=?", (command.parsed.question_id.0, )).map_err(internal_error)?.ok_or(QuestionError::QuestionDoesNotExist)?;
        let version = opt_hash_from_value(version).ok_or(QuestionError::InternalError)?;
        let timestamp = timestamp_now().map_err(internal_error)?;
        let for_bb = PlainTextVoteOnQuestionCommandPostedToBulletinBoard { command: command.clone(), timestamp, prior: version };
        let version = LogInBulletinBoard::PlainTextVoteQuestion(for_bb).log_in_bulletin_board().await.map_err(bulletin_board_error)?;
        transaction.exec_drop("update QUESTIONS set Version=?,LastModifiedTimestamp=?,TotalVotes=TotalVotes+1,NetVotes=NetVotes+? where QuestionID=?", (version.0, timestamp, if command.parsed.up { 1 } else { -1 }, command.parsed.question_id.0)).map_err(internal_error)?;
        transaction.exec_drop("insert into HAS_VOTED (QuestionID,VoterId) values (?,?)", (command.parsed.question_id.0, user_id)).map_err(internal_error)?;
        transaction.commit().map_err(internal_error)?;
        Ok(version)
    }
}

/// When you query for the best questions matching various things, there is a trade off between various constraints.
/// The list of resulting questions is ordered by a score, which is the sum of the weights below times the individual subscores.
/// If you don't want to use some subscore, set the weight to zero.
#[derive(Serialize,Deserialize,Debug,Clone,Copy)]
pub struct WeightsForScoring {
    /// The weight multiplying the score from text matching. The base score is a max of 10 per word.
    pub text : u64,
    /// The weight multiplying the number of metadata items matching. The base score is 1 per metadata match.
    pub metadata : u64,
    /// The weight multiplying a score reflecting the total number of votes. The base score is ln(total_votes+1), which is 0 for 0 votes and ~0.69 for 1 vote and ~2.4 for 10 votes and ~6.9 for 1000 votes.
    pub total_votes : u64,
    /// The weight multiplying a score reflecting the net votes. The base score is signum(net_votes)*ln(|net_votes|+1). This is 0 for 0 net votes, ~0.69 for 1 net vote and ~-0.69 for -1 net votes
    pub net_votes : u64,
    /// The weight multiplying the recentness score, which is 0 to 1, with 1 meaning just posted and 0 meaning very old.
    pub recentness : u64,
    /// The time scale in seconds of "recentness", which is e^(-(time since inception)/recentness_timescale)
    pub recentness_timescale : u64,
}

/// A token returned from a query, which can be used (as long as it is not stale) to get the next page of the same query
pub type PreviousQueryToken = HashValue;




impl QuestionPagination {
    fn generate_random_token() -> PreviousQueryToken {
        let mut res = [0u8;32];
        rand::thread_rng().fill(&mut res);
        HashValue(res)
    }

    async fn get_similar_question_cache() -> MutexGuard<'static,lru::LruCache<PreviousQueryToken,Vec<ScoredIDs<QuestionID>>>> {
        static CACHE : Lazy<Mutex<lru::LruCache<PreviousQueryToken,Vec<ScoredIDs<QuestionID>>>>> = Lazy::new(|| {
            Mutex::new(lru::LruCache::new(CONFIG.search_cache_size))
        });
        CACHE.lock().await
    }

    /// store a previously computed result in the cache.
    async fn remember_similar_question_result(result:Vec<ScoredIDs<QuestionID>>) -> PreviousQueryToken {
        let token = Self::generate_random_token(); // 256 bit tokens won't clash. And it doesn't matter much even if they did.
        Self::get_similar_question_cache().await.put(token,result);
        token
    }

    fn get_requested_page(&self,all_questions:&Vec<ScoredIDs<QuestionID>>) -> Vec<ScoredIDs<QuestionID>> {
        all_questions[self.from.min(all_questions.len())..self.to.min(all_questions.len())].to_vec()
    }

    /// Get a result from a prior list, if possible.
    async fn try_get_previously_remembered_similar_question_result(&self) -> Option<SimilarQuestionResult> {
        let mut cache = Self::get_similar_question_cache().await;
        if let Some(token) = &self.token {
            if let Some(found) = cache.get(token) {
                Some(SimilarQuestionResult{ token: Some(*token), questions: self.get_requested_page(found) })
            } else { None }
        } else { None }
    }

}


/// Information about which pages you want of the current question
#[derive(Serialize,Deserialize,Debug,Clone,Copy)]
pub struct QuestionPagination {
    /// get questions from the given value (inclusive, first is 0)
    pub from : usize,
    /// Get questions to the given value (exclusive)
    pub to : usize,
    /// If possible, use the token returned from the previous question to get the next page of the same result set.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub token : Option<PreviousQueryToken>,
}

impl Default for QuestionPagination {
    fn default() -> Self {
        QuestionPagination{ from: 0, to: 50, token: None}
    }
}

#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct SimilarQuestionQuery {
    pub weights : WeightsForScoring,
    #[serde(default)]
    pub page : QuestionPagination,
    /// The text of the question
    pub question_text : String,
    // additional metadata that may be requested.
    #[serde(flatten)]
    pub non_defining_fields : QuestionNonDefiningFields,
}

#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct SimilarQuestionResult {
    // if there are more pages, a token that can be used to retrieve them. If a token was provided, and the same token returned, it used the previous results. Otherwise token was stale and a new one was used.
    pub token : Option<PreviousQueryToken>,
    pub questions : Vec<ScoredIDs<QuestionID>>
}

impl SimilarQuestionQuery {

    /// This function doesn't scale. It takes time proportional to the number of questions.
    /// This will need to be addressed at some point, but it will probably require heuristics
    /// which will require knowledge of the types of questions that appear, which will be a lot
    /// easier when more people are using it.
    pub async fn similar_questions(command:&SimilarQuestionQuery) -> Result<SimilarQuestionResult,QuestionError> {
        if let Some(cached_result) = command.page.try_get_previously_remembered_similar_question_result().await {
            return Ok(cached_result); // not just for speed, also to avoid missing/duplicate questions.
        }
        let just_text = find_similar_text_question(&command.question_text).await.map_err(internal_error)?;
        let just_metadata  = QuestionNonDefiningFields::find_similar_metadata(&command.non_defining_fields).await?;
        let mut all_questions: Vec<ScoredIDs<QuestionID>> = if just_metadata.is_empty() {
            if command.question_text.is_empty() { // get trending questions.
                QuestionInfo::get_list_of_all_questions().await.map_err(internal_error)?.into_iter().map(|id|ScoredIDs{id,score:0.0}).collect()
            } else {
                just_text.into_iter().map(|sid|ScoredIDs{ id: sid.id, score: command.weights.text as f64*sid.score}).collect()
            }
        } else if just_text.is_empty() { // if no text matches, just use metadata matches.
            just_metadata.into_iter().map(|(q,n)|ScoredIDs{ id: q, score: command.weights.metadata as f64*(n as f64) }).collect()
        } else { // if both text and metadata matches, use the intersection of just_text and just_metadata, use just the ones with matching text, but add metadata scores.
            just_text.into_iter().filter_map(|s|just_metadata.get(&s.id).map(|metadata_score|ScoredIDs{ id:s.id, score:command.weights.text as f64*s.score+command.weights.metadata as f64*(*metadata_score as f64)})).collect()
            // code if want all text matches with metadata scores added.
            // just_text.into_iter().map(|s|ScoredIDs{ id:s.id, score:command.weights.text as f64*s.score+command.weights.metadata as f64*(just_metadata.get(&s.id).cloned().unwrap_or(0) as f64)}).collect()
        };
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        let now = timestamp_now().map_err(internal_error)?;
        for q in &mut all_questions {
            if let Some((created,total_votes,net_votes)) = conn.exec_first::<(Timestamp,u64,i64),_,_>("SELECT CreatedTimestamp,TotalVotes,NetVotes from QUESTIONS where QuestionID=?", (&q.id.0,)).map_err(internal_error)? {
                let recentness = if created>now {0.0} else {f64::exp((-((now-created)as f64))/(command.weights.recentness_timescale as f64))};
                let total_votes = f64::ln_1p(total_votes as f64);
                let net_votes = (if net_votes>=0 {1.0} else {-1.0})*f64::ln_1p(f64::abs(total_votes as f64));
                q.score+=command.weights.recentness as f64*recentness+command.weights.net_votes as f64*net_votes+command.weights.total_votes as f64*total_votes;
            }
        }
        all_questions.sort_by(|a, b|b.score.partial_cmp(&a.score).unwrap());
        let questions = command.page.get_requested_page(&all_questions);
        let token = if all_questions.len()==questions.len() { None } else { Some(QuestionPagination::remember_similar_question_result(all_questions).await) };
        Ok(SimilarQuestionResult{token,questions})
    }

}

