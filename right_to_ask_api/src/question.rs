//! Information about a question. Includes APIs for modifying the database.

// Functions here
// - submit New Question
// - edit existing question
// - look up current info on a specific question.
// - TODO look for similar questions


use std::collections::HashSet;
use std::convert::TryInto;
use std::fmt::{Debug, Display, Formatter};
use serde::{Serialize, Deserialize};
use merkle_tree_bulletin_board::hash::HashValue;
use merkle_tree_bulletin_board::hash_history::{Timestamp, timestamp_now};
use mysql::prelude::Queryable;
use mysql::TxOpts;
use sha2::{Digest, Sha256};
use crate::database::{get_rta_database_connection, LogInBulletinBoard};
use crate::mp::{get_org_id_from_database, MPId, MPIndexInDatabaseTable, MPSpec, OrgIndexInDatabaseTable};
use crate::person::{is_user_mp_or_staffer, user_exists, UserUID};
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
    OnlyMPCanAnswerQuestion,
    BackgroundTooLong,
    SameQuestionSubmittedRecently,
    OnlyAuthorCanChangeBackground,
    OnlyAuthorCanChangePermissions,
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
    /// The user to ask/answer the question does not exist.
    InvalidUserSpecified,
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
}

impl PersonID {
    /// Get the people who should ask (role='Q') or answer (role='A') a question.
    fn get_for_question(conn:&mut impl Queryable,role:char,question:QuestionID) -> mysql::Result<Vec<PersonID>> {
        let elements : Vec<(Option<UserUID>,Option<MPIndexInDatabaseTable>,Option<OrgIndexInDatabaseTable>)> = conn.exec_map("SELECT UID,MP,ORG from PersonForQuestion where QuestionId=? and ROLE=?",(&question.0,role.to_string()),|(uid,mp,org)|(uid,mp,org))?;
        let mut res = vec![];
        for (uid,mp,org) in elements {
            let decoded = {
                if let Some(uid) = uid { PersonID::User(uid) }
                else if let Some(mp) = mp { // we may want to cache this for performance.
                    if let Some(mp_id) = MPId::read_from_database(conn,mp)? {
                        PersonID::MP(mp_id)
                    } else {
                        eprintln!("Missing mp {} for question {} role {}",mp,question,role);
                        continue;
                    }
                } else if let Some(org) = org { // we may want to cache this for performance.
                    if let Some(org_id) = conn.exec_first::<String,_,_>("select OrgID from Organisations where id=?",(org,))? {
                        PersonID::Organisation(org_id)
                    } else {
                        eprintln!("Missing organisation {} for question {} role {}",org,question,role);
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
    fn add_for_question(conn:&mut impl Queryable,role:char,question:QuestionID,people:HashSet<&PersonID>) -> mysql::Result<()> {
        let mut references : Vec<(Option<UserUID>,Option<MPIndexInDatabaseTable>,Option<OrgIndexInDatabaseTable>)> = vec![];
        for &person in people.iter() {
            match person {
                PersonID::User(uid) => {
                    references.push((Some(uid.clone()),None,None));
                }
                PersonID::MP(mp_id) => {
                    let id = mp_id.get_id_from_database(conn)?;
                    references.push((None,Some(id),None));
                }
                PersonID::Organisation(org_name) => {
                    let id = get_org_id_from_database(org_name,conn)?;
                    references.push((None,None,Some(id)));
                }
            }
        }
        let role = role.to_string();
        conn.exec_batch("insert into PersonForQuestion (QuestionId,ROLE,UID,MP,ORG) values (?,?,?,?,?)",references.into_iter().map(|(uid,mp,org)|(question.0,&role,uid,mp,org)))?;
        Ok(())
    }

    fn check_sane(&self,conn:&mut impl Queryable) -> Result<(),QuestionError> {
        match self {
            PersonID::User(uid) => {
                if !user_exists(uid,conn).map_err(internal_error)? { return Err(QuestionError::InvalidUserSpecified) }
            }
            PersonID::MP(mp_id) => {
                let mps = MPSpec::get();
                let mps = (*mps).as_ref().map_err(internal_error)?;
                if !mps.contains(mp_id) { return Err(QuestionError::InvalidMP) }
            }
            PersonID::Organisation(org) => {
                if org.len()>50 { return Err(QuestionError::OrganisationNameTooLong); }
            }
        }
        Ok(())
    }
}


fn is_false(x:&bool) -> bool { !*x }

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
    /// Merge rule: TODO consider whether the version check changes this. Eliminate duplicates (including with values already present). If the total number of values doesn't exceed the limit, accept. If the limit has already been exceeded, reject. If it hasn't, but would if this update was accepted, send a merge request back to the client (pick at most m out of the n you tried to submit...).  Note that this might cause cascading merges that need to be manually resolved, but that's less trouble than allowing locks.
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
    /// Validity : domain must be aph.gov.au, parliament.vic.gov.au, etc. (preloaded permit-list - note that url sanitation is nontrivial). TODO work out nontrivial stuff
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

#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct QuestionAnswer {
    /// must be a MP
    answered_by : UserUID,
    mp : MPId,
    answer : String,
    /// set by server - client should not set this when sending to server.
    timestamp : Option<Timestamp>,
}

///  domain must be aph.gov.au, parliament.vic.gov.au, etc. (preloaded permit-list - note that url sanitation is nontrivial).
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct HansardLink {
    pub url : String, // Should this be more structured?
}


impl QuestionNonDefiningFields {
    pub async fn check_legal(&self,is_creator:bool,user:&UserUID,existing:Option<&QuestionInfo>) -> Result<(),QuestionError> {
        if let Some(background) = &self.background {
            if background.len()>MAX_BACKGROUND_LENGTH { return Err(QuestionError::BackgroundTooLong); }
            if !is_creator { return Err(QuestionError::OnlyAuthorCanChangeBackground); }
            if !existing.and_then(|info|info.non_defining.background.as_ref()).map(|e|background.starts_with(e)).unwrap_or(true) { return Err(QuestionError::CanOnlyExtendBackground); }
        }
        if self.answers.iter().any(|a|a.answer.len()>MAX_ANSWER_LENGTH)  { return Err(QuestionError::AnswerTooLong); }
        if self.answers.iter().any(|a|a.answer.len()>MAX_ANSWER_LENGTH)  { return Err(QuestionError::AnswerTooLong); }
        if (!self.answers.is_empty()) && !is_user_mp_or_staffer(user).await.map_err(internal_error)?  { return Err(QuestionError::OnlyMPCanAnswerQuestion); }
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
        // TODO finish legal checks.
        Ok(())
    }

    /// Add a simple question to the database, without any extra information yet.
    async fn modify_database(&self,question_id:QuestionID,new_version:LastQuestionUpdate,expecting_version:Option<LastQuestionUpdate>,timestamp:Timestamp) -> Result<(),QuestionError> {
        println!("modify_database with question non-defining fields {:?}",self);
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        let mut transaction = conn.start_transaction(TxOpts::default()).map_err(internal_error)?;
        if let Some(current_version) = transaction.exec_first::<mysql::Value,_,_>("select Version from QUESTIONS where QuestionID=?",(question_id.0,)).map_err(internal_error)? {
            let expected : mysql::Value = expecting_version.map(|v|v.0).into();
            if expected!=current_version { return Err(QuestionError::LastUpdateIsNotCurrent); }
        } else { return Err(QuestionError::QuestionDoesNotExist); }
        transaction.exec_drop("update QUESTIONS set LastModifiedTimestamp=?,Version=? where QuestionID=?", (timestamp,new_version.0,question_id.0)).map_err(internal_error)?;
        if let Some(background) = &self.background {
            println!("Setting background to {}",background);
            transaction.exec_drop("update QUESTIONS set Background=? where QuestionID=?", (background,question_id.0)).map_err(internal_error)?;
        }
        if !self.who_should_ask_the_question_permissions.is_no_change() {
            transaction.exec_drop("update QUESTIONS set CanOthersSetWhoShouldAsk=? where QuestionID=?", (self.who_should_ask_the_question_permissions==Permissions::Others,question_id.0)).map_err(internal_error)?;
        }
        if !self.who_should_answer_the_question_permissions.is_no_change() {
            transaction.exec_drop("update QUESTIONS set CanOthersSetWhoShouldAnswer=? where QuestionID=?", (self.who_should_answer_the_question_permissions==Permissions::Others,question_id.0)).map_err(internal_error)?;
        }
        if !self.mp_who_should_ask_the_question.is_empty() {
            let existing = PersonID::get_for_question(&mut transaction,'Q',question_id).map_err(internal_error)?.into_iter().collect::<HashSet<_>>();
            let extra : HashSet<_> = self.mp_who_should_ask_the_question.iter().filter(|&m|!existing.contains(m)).collect();
            if existing.len()+extra.len() > MAX_MPS_WHO_SHOULD_ASK_THE_QUESTION { return Err(QuestionError::TooLongListOfPeopleAskingQuestion);}
            PersonID::add_for_question(&mut transaction,'Q',question_id,extra).map_err(internal_error)?;
        }
        if !self.entity_who_should_answer_the_question.is_empty() {
            let existing = PersonID::get_for_question(&mut transaction,'A',question_id).map_err(internal_error)?.into_iter().collect::<HashSet<_>>();
            let extra : HashSet<_> = self.entity_who_should_answer_the_question.iter().filter(|&m|!existing.contains(m)).collect();
            if existing.len()+extra.len() > MAX_MPS_WHO_SHOULD_ASK_THE_QUESTION { return Err(QuestionError::TooLongListOfPeopleAskingQuestion);}
            PersonID::add_for_question(&mut transaction,'A',question_id,extra).map_err(internal_error)?;
        }
        if let Some(follow_up_to) = self.is_followup_to {
            transaction.exec_drop("update QUESTIONS set FollowUpTo=? where QuestionID=?", (follow_up_to.0,question_id.0)).map_err(internal_error)?;
        }
        // TODO insert stuff.
        transaction.commit().map_err(internal_error)?;
        Ok(())
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

fn internal_error<T:Debug>(error:T) -> QuestionError {
    eprintln!("Internal error {:?}",error);
    QuestionError::InternalError
}
fn bulletin_board_error(error:anyhow::Error) -> QuestionError {
    eprintln!("Bulletin Board error {:?}",error);
    QuestionError::CouldNotWriteToBulletinBoard
}

impl NewQuestionCommand {
    /// Add a simple question to the database, without any extra information yet.
    async fn add_question_stub(user:&str,question:&str,timestamp:Timestamp) -> Result<QuestionID,QuestionError> {
        let defining = QuestionDefiningFields{
            author: user.to_string(),
            question_text: question.to_string(),
            timestamp
        };
        let question_id = defining.compute_hash();
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        let mut transaction = conn.start_transaction(TxOpts::default()).map_err(internal_error)?;
        if let Some(existing_timestamp) = transaction.exec_first::<Timestamp,_,_>("select CreatedTimestamp from QUESTIONS where Question=? and CreatedBy=? ORDER BY CreatedTimestamp DESC",(question,user)).map_err(internal_error)? {
            if existing_timestamp+24*60*60 > timestamp { return Err(QuestionError::YouJustAskedThatQuestion)}
        }
        transaction.exec_drop("insert into QUESTIONS (QuestionID,Question,CreatedTimestamp,LastModifiedTimestamp,CreatedBy,CanOthersSetWhoShouldAsk,CanOthersSetWhoShouldAnswer,AnswerAccepted) values (?,?,?,?,?,FALSE,FALSE,FALSE)", (question_id.0,question,timestamp,timestamp,user)).map_err(internal_error)?;
        transaction.commit().map_err(internal_error)?;
        Ok(question_id)
    }

    /// API function to add a question to the server
    pub async fn add_question(question:&ClientSigned<NewQuestionCommand>) -> Result<NewQuestionCommandResponse,QuestionError> {
        if question.parsed.question_text.len()>MAX_QUESTION_LENGTH { return Err(QuestionError::QuestionTooLong); }
        if question.parsed.question_text.len()<MIN_QUESTION_LENGTH { return Err(QuestionError::QuestionTooShort); }
        question.parsed.non_defining_fields.check_legal(true,&question.signed_message.user,None).await?;
        let timestamp = timestamp_now().map_err(internal_error)?;
        let question_id = Self::add_question_stub(&question.signed_message.user,&question.parsed.question_text,timestamp).await?;
        let for_bb = NewQuestionCommandPostedToBulletinBoard {
            command: question.clone(),
            timestamp,
            question_id
        };
        let version = LogInBulletinBoard::NewQuestion(for_bb).log_in_bulletin_board().await.map_err(bulletin_board_error)?;
        question.parsed.non_defining_fields.modify_database(question_id,version,None,timestamp).await?;
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
    defining : QuestionDefiningFields,
    #[serde(flatten)]
    non_defining : QuestionNonDefiningFields,
    question_id : QuestionID,
    version : LastQuestionUpdate,
    last_modified : Timestamp,
}

/// Convert v into a HashValue where you know v will be a 32 byte value
/// TODO make original functions in bulletin board code public.
fn hash_from_value(v:mysql::Value) -> HashValue {
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
        if let Some((question_text,timestamp,last_modified,version,author,background,who_should_ask_the_question_permissions,who_should_answer_the_question_permissions,answer_accepted,is_followup_to)) = conn.exec_first("SELECT Question,CreatedTimestamp,LastModifiedTimestamp,Version,CreatedBy,Background,CanOthersSetWhoShouldAsk,CanOthersSetWhoShouldAnswer,AnswerAccepted,FollowUpTo from QUESTIONS where QuestionID=?",(question_id.0,)).map_err(internal_error)? {
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
                            answers: vec![], // TODO
                            answer_accepted,
                            hansard_link: vec![], // TODO
                            is_followup_to : opt_hash_from_value(is_followup_to),
                        },
                        question_id,
                        version,
                        last_modified,
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
        command.parsed.edits.modify_database(command.parsed.question_id,version,Some(command.parsed.version),timestamp).await?;
        Ok(version)
    }
}






/*************************************************************************
                        FLAG A QUESTION
 *************************************************************************/

#[derive(Serialize,Deserialize,Copy,Clone)]
pub enum FlagReason {
    NotAQuestion,
    ThreateningViolence,
    IncludesPrivateInformation,
    IncitesHatred,
    EncouragesHarm,
    TargetedHarassment,
    DefamatoryInsinuation, // You're allowed to ask a real question, including some that may be perceived as offensive, but you're not allowed to ask
                           // questions that presuppose misbehaviour unless it is a matter of public record.
                           // e.g. it's OK to ask, "Is it true, as alleged by X, that you accepted a bribe..."
                           // but you're not allowed to ask "When are you going to stop taking bribes"?
}



/// This is used to flag a question as deserving of censorship.
/// Its intention is to allow people to inform the server of questions that are threatening, abusive, etc.
/// Exactly how this translates into a censor instruction to the BB is undefined - for example, it could be automatic based on the fraction of viewers who flag it, or it could require human intervention.
/// There is still a lot of work to go here.
#[derive(Serialize,Deserialize)]
pub struct FlagQuestion {
    question_id : QuestionID,
    reason : FlagReason,
}