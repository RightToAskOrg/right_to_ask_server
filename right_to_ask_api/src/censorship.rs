//! Dealing with censorship of questions. Could conceivably be part of question.rs.

use std::collections::HashMap;
use std::fmt;
use anyhow::anyhow;
use merkle_tree_bulletin_board::hash::HashValue;
use merkle_tree_bulletin_board::hash_history::{HashSource, LeafHashHistory, Timestamp, timestamp_now};
use mysql::Error::MySqlError;
use mysql::prelude::Queryable;
use mysql::TxOpts;
use mysql_common::value::convert::{ConvIr, FromValue, FromValueError};
use mysql_common::value::Value;
use crate::database::{get_bulletin_board, get_rta_database_connection, LogInBulletinBoard, remove_question_from_comparison_database};
use crate::question::{bulletin_board_error, hash_from_value, internal_error, LastQuestionUpdate, modify_question_database_version_and_time, QuestionError, QuestionID, QuestionInfo};
use crate::signing::ClientSigned;
use serde::{Serialize, Deserialize};
use crate::person::UserID;

/// Why a question could be censored.
#[derive(Debug,Copy,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub enum CensorshipReason {
    NotAQuestion,
    ThreateningViolence,
    IncludesPrivateInformation,
    IncitesHatredOrDiscrimination,
    EncouragesHarm,
    TargetedHarassment,
    /// You're allowed to ask a real question, including some that may be perceived as offensive, but you're not allowed to ask
    /// questions that presuppose misbehaviour unless it is a matter of public record.
    /// e.g. it's OK to ask, "Is it true, as alleged by X, that you accepted a bribe..."
    /// but you're not allowed to ask "When are you going to stop taking bribes"?
    DefamatoryInsinuation,
    Illegal,
    Impersonation,
    Spam,
}

/// Whether a question is censored or not... or things inbetween.
/// * The state always starts off as NotFlagged.
/// * When a question is reported/flagged, NotFlagged->Flagged, and StructureChanged->StructureChangedThenFlagged
/// * When a question is moderated, it is converted to Censored or Allowed.
/// * When a question is modified, it is converted Allowed->StructureChanged.
#[derive(Debug,Copy,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub enum CensorshipStatus {
    /// no one has complained about it.
    NotFlagged,
    /// Someone has complained about it, but a moderator has not yet made a decision
    Flagged,
    /// it was flagged, a moderator has looked at it and decided not to censor it.
    Allowed,
    /// It was Allowed, but the structure has changed since then - e.g. background added, or an answer added.
    StructureChanged,
    /// It was Allowed, but the structure has changed since then, and then it was flagged again.
    StructureChangedThenFlagged,
    /// The moderator decided this should not be shown
    Censored,
}

/* Boilerplate to make it easy to transfer CensorshipReason to SQL */

// Provide Display & to_string() for State enum
impl fmt::Display for CensorshipReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<CensorshipReason> for Value {
    fn from(s: CensorshipReason) -> Self {
        Value::Bytes(s.to_string().into_bytes())
    }
}

impl TryFrom<&str> for CensorshipReason {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.as_bytes() {
            b"NotAQuestion" => Ok(CensorshipReason::NotAQuestion),
            b"ThreateningViolence" => Ok(CensorshipReason::ThreateningViolence),
            b"IncludesPrivateInformation" => Ok(CensorshipReason::IncludesPrivateInformation),
            b"IncitesHatredOrDiscrimination" => Ok(CensorshipReason::IncitesHatredOrDiscrimination),
            b"EncouragesHarm" => Ok(CensorshipReason::EncouragesHarm),
            b"TargetedHarassment" => Ok(CensorshipReason::TargetedHarassment),
            b"DefamatoryInsinuation" => Ok(CensorshipReason::DefamatoryInsinuation),
            b"Illegal" => Ok(CensorshipReason::Illegal),
            b"Impersonation" => Ok(CensorshipReason::Impersonation),
            b"Spam" => Ok(CensorshipReason::Spam),
            _ => Err(anyhow!("Invalid state {}",value)),
        }
    }
}
impl ConvIr<CensorshipReason> for CensorshipReason {
    fn new(v: Value) -> Result<Self, FromValueError> {
        match v {
            Value::Bytes(bytes) => match bytes.as_slice() {
                b"NotAQuestion" => Ok(CensorshipReason::NotAQuestion),
                b"ThreateningViolence" => Ok(CensorshipReason::ThreateningViolence),
                b"IncludesPrivateInformation" => Ok(CensorshipReason::IncludesPrivateInformation),
                b"IncitesHatredOrDiscrimination" => Ok(CensorshipReason::IncitesHatredOrDiscrimination),
                b"EncouragesHarm" => Ok(CensorshipReason::EncouragesHarm),
                b"TargetedHarassment" => Ok(CensorshipReason::TargetedHarassment),
                b"DefamatoryInsinuation" => Ok(CensorshipReason::DefamatoryInsinuation),
                b"Illegal" => Ok(CensorshipReason::Illegal),
                b"Impersonation" => Ok(CensorshipReason::Impersonation),
                b"Spam" => Ok(CensorshipReason::Spam),
                _ => Err(FromValueError(Value::Bytes(bytes))),
            },
            v => Err(FromValueError(v)),
        }
    }

    fn commit(self) -> Self { self }
    fn rollback(self) -> Value { self.into() }
}

impl FromValue for CensorshipReason {
    type Intermediate = Self;
}



impl Default for CensorshipStatus {
    fn default() -> Self { CensorshipStatus::NotFlagged }
}

/* Boilerplate to make it easy to transfer CensorshipStatus to SQL */

// Provide Display & to_string() for State enum
impl fmt::Display for CensorshipStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<CensorshipStatus> for Value {
    fn from(s: CensorshipStatus) -> Self {
        Value::Bytes(s.to_string().into_bytes())
    }
}

impl TryFrom<&str> for CensorshipStatus {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.as_bytes() {
            b"NotFlagged" => Ok(CensorshipStatus::NotFlagged),
            b"Flagged" => Ok(CensorshipStatus::Flagged),
            b"Allowed" => Ok(CensorshipStatus::Allowed),
            b"StructureChanged" => Ok(CensorshipStatus::StructureChanged),
            b"StructureChangedThenFlagged" => Ok(CensorshipStatus::StructureChangedThenFlagged),
            b"Censored" => Ok(CensorshipStatus::Censored),
            _ => Err(anyhow!("Invalid state {}",value)),
        }
    }
}
impl ConvIr<CensorshipStatus> for CensorshipStatus {
    fn new(v: Value) -> Result<Self, FromValueError> {
        match v {
            Value::Bytes(bytes) => match bytes.as_slice() {
                b"NotFlagged" => Ok(CensorshipStatus::NotFlagged),
                b"Flagged" => Ok(CensorshipStatus::Flagged),
                b"Allowed" => Ok(CensorshipStatus::Allowed),
                b"StructureChanged" => Ok(CensorshipStatus::StructureChanged),
                b"StructureChangedThenFlagged" => Ok(CensorshipStatus::StructureChangedThenFlagged),
                b"Censored" => Ok(CensorshipStatus::Censored),
                _ => Err(FromValueError(Value::Bytes(bytes))),
            },
            v => Err(FromValueError(v)),
        }
    }

    fn commit(self) -> Self { self }
    fn rollback(self) -> Value { self.into() }
}

impl FromValue for CensorshipStatus {
    type Intermediate = Self;
}


#[derive(Serialize,Deserialize,Debug,Clone)]
/// A command by an administrator to perform a censorship.
pub struct CensorQuestionCommand {
    pub reason : CensorshipReason,
    /// If true, censor the logs as well. Otherwise just censor in the app.
    pub censor_logs : bool,
    /// If set, don't censor the question, just the answer that was submitted in the given bulletin board entry.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub just_answer : Option<HashValue>,
    pub question_id : QuestionID,
    /// the version number of the question being censored.
    pub version : HashValue,
}

impl CensorQuestionCommand {
    pub async fn censor_question(&self) -> Result<HashValue,QuestionError> {
        let question_info = QuestionInfo::lookup(self.question_id).await?.ok_or_else(||QuestionError::QuestionDoesNotExist)?;  // Makes sure the question exists and is not censored already.
        if question_info.version!=self.version { return Err(QuestionError::LastUpdateIsNotCurrent); }
        let timestamp = timestamp_now().map_err(internal_error)?;
        let mut removed : Vec<CensoredBulletinBoardQuestionElement> = Vec::new();
        let version = if self.censor_logs { // work out exactly what we want to censor, and put it in "removed".
            let history = QuestionHistory::lookup(self.question_id).await?;
            for h in &history.history {
                match &h.action {
                    Some(LogInBulletinBoard::NewQuestion(_)) => { removed.push(CensoredBulletinBoardQuestionElement{id:h.id,prior:None})}
                    Some(LogInBulletinBoard::EditQuestion(q)) => { removed.push(CensoredBulletinBoardQuestionElement{id:h.id,prior:Some(q.prior)})}
                    _ => {} // don't censor user flags or censorship!
                }
            }
            if let Some(answer_id) = self.just_answer {
                removed.retain(|e|e.id==answer_id);
                if removed.len()!=1 { return Err(QuestionError::NotAnUncensoredAnswer)}
            }
            history.history[0].id
        } else {
            if let Some(answer_id) = self.just_answer {
                if !question_info.non_defining.answers.iter().any(|a|a.version==Some(answer_id) && a.censorship_status!=CensorshipStatus::Censored) { return Err(QuestionError::NotAnUncensoredAnswer)}
            }
            question_info.version
        };
        let for_bb = CensorQuestionCommandPostedToBulletinBoard{
            command : self.clone(),
            prior : version,
            removed : removed.clone(),
        };
        let response = LogInBulletinBoard::CensorQuestion(for_bb).log_in_bulletin_board().await.map_err(bulletin_board_error)?;
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        let mut transaction = conn.start_transaction(TxOpts::default()).map_err(internal_error)?;
        modify_question_database_version_and_time(&mut transaction,self.question_id,response,Some(version),timestamp).await?;
        if let Some(answer_id) = self.just_answer {
            transaction.exec_drop("update Answer set CensorshipStatus='Censored' where version=?", (answer_id.0,)).map_err(internal_error)?;
            transaction.exec_drop("update QUESTIONS set NumFlags=NumFlags-??? where QuestionID=?", (self.question_id.0,)).map_err(internal_error)?; // TODO properly

        } else { // censor the whole question
            transaction.exec_drop("update QUESTIONS set CensorshipStatus='Censored' where QuestionID=?", (self.question_id.0,)).map_err(internal_error)?; // TODO update NumFlags
        }
        transaction.commit().map_err(internal_error)?;
        // TODO it would make sense to put some message in the BB saying that the just posted entry did not make it into the database for some reason if there were an error above.
        for remove in removed { // don't censor things until stored in the database otherwise we will be unhappy.
            get_bulletin_board().await.censor_leaf(remove.id).map_err(bulletin_board_error)?;
        }
        remove_question_from_comparison_database(self.question_id).await.map_err(internal_error)?;
        Ok(response)
    }
}

/// The structure posted to the bulletin board in response to an EditQuestionCommand.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct CensorQuestionCommandPostedToBulletinBoard {
    pub command : CensorQuestionCommand,
    /// This will be a link to the prior node in the database.
    pub prior : LastQuestionUpdate,
    #[serde(skip_serializing_if = "Vec::is_empty",default)]
    pub removed : Vec<CensoredBulletinBoardQuestionElement>
}

/// Censoring an element in the bulletin board disrupts the linked list. This provides the prior elements for disrupted elements.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct CensoredBulletinBoardQuestionElement {
    /// The id that was removed from the Bulletin Board
    pub id : HashValue,
    /// The prior element in the linked list.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub prior : Option<HashValue>,
}

#[derive(Serialize,Deserialize,Debug,Clone)]
/// This is used to flag a question as deserving of censorship.
/// Its intention is to allow people to inform the server of questions that are threatening, abusive, etc.
/// Exactly how this translates into a censor instruction to the BB is undefined - for example, it could be automatic based on the fraction of viewers who flag it, or it could require human intervention.
/// There is still a lot of work to go here.
pub struct ReportQuestionCommand {
    pub reason : CensorshipReason,
    pub question_id : QuestionID,
    /// If set, don't censor the question, just the answer that was submitted in the given bulletin board entry.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    pub just_answer : Option<HashValue>,
}

/// The structure posted to the bulletin board in response to an ReportQuestionCommand.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct ReportQuestionCommandPostedToBulletinBoard {
    pub command : ClientSigned<ReportQuestionCommand>,
    /// This will be a link to the prior node in the database.
    pub prior : LastQuestionUpdate,
}


impl ReportQuestionCommand {
    pub async fn report_question(command:&ClientSigned<ReportQuestionCommand>) -> Result<(),QuestionError> /* Should produce a HashValue if want to post on BB */{
        /* Used if we want to post report questions on the bulletin board
        let question_info = QuestionInfo::lookup(command.parsed.question_id).await?.ok_or_else(||QuestionError::QuestionDoesNotExist)?;
        let timestamp = timestamp_now().map_err(internal_error)?;
        let for_bb = ReportQuestionCommandPostedToBulletinBoard{
            command : command.clone(),
            prior : question_info.version,
        };
        let response = LogInBulletinBoard::ReportQuestion(for_bb).log_in_bulletin_board().await.map_err(bulletin_board_error)?; */
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        let mut transaction = conn.start_transaction(TxOpts::default()).map_err(internal_error)?;
        /* Used if we want to post report questions on the bulletin board
        modify_question_database_version_and_time(&mut transaction,command.parsed.question_id,response,Some(question_info.version),timestamp).await?;
         */
        let user_id : UserID = transaction.exec_first("select id from USERS where UID=?",(&command.signed_message.user,)).map_err(internal_error)?.ok_or(QuestionError::NoSuchUser)?;
        let insert_result = if let Some(answer) = command.parsed.just_answer {
            transaction.exec_drop("INSERT INTO AnswerReportedReasons (QuestionId,reason,answer,user_id) VALUES (?,?,?,?)",(command.parsed.question_id.0,command.parsed.reason,answer.0,user_id))
        } else {
            transaction.exec_drop("INSERT INTO QuestionReportedReasons (QuestionId,reason,user_id) VALUES (?,?,?)",(command.parsed.question_id.0,command.parsed.reason,user_id))
        };
        match insert_result {
            Ok(()) => {},
            Err(MySqlError(e)) if e.code== (mysql::ServerError::ER_DUP_ENTRY as u16) => {return Err(QuestionError::AlreadyReported)},
            Err(e) => {return Err(internal_error(e))}
        }
        transaction.exec_drop("update QUESTIONS set NumFlags=NumFlags+1, CensorshipStatus = IF(CensorshipStatus='NotFlagged','Flagged', IF(CensorshipStatus='StructureChanged','StructureChangedThenFlagged', CensorshipStatus))  where QuestionId=?",(&command.parsed.question_id.0,)).map_err(internal_error)?;
        transaction.commit().map_err(internal_error)?;
        Ok(()) // Should return response if want to post report questions on the bulletin board
    }
}

/// A summary list of questions that have a reported count > 0.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct ReportedQuestionSummary {
    id : QuestionID,
    question_text : String,
    /// the number of times it has been flagged since last count.
    num_flags : usize,
    censorship_status : CensorshipStatus,
}

impl ReportedQuestionSummary {
    /// Get a list of all the reported questions since last moderation.
    pub async fn get_reported_questions()  -> mysql::Result<Vec<ReportedQuestionSummary>> {
        let mut conn = get_rta_database_connection().await?;
        let elements : Vec<ReportedQuestionSummary> = conn.exec_map("SELECT QuestionID,Question,NumFlags,CensorshipStatus from QUESTIONS where NumFlags>0 ORDER BY NumFlags DESC",(),|(id,question_text,num_flags,censorship_status)|ReportedQuestionSummary{id:hash_from_value(id), question_text, num_flags, censorship_status })?;
        Ok(elements)
    }
}

/// Why and how many people wanted to censor a question
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct ReportedQuestionReasonSummary {
    num_flags : usize,
    censorship_status : CensorshipStatus,
    reasons : Vec<SingleReasonSummary>,
}

/// The number of people that gave a specific reason for censoring.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct SingleReasonSummary {
    reason : CensorshipReason,
    count : usize,
    /// if this pertains to a specific answer, the identifier for the answer.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    answer : Option<HashValue>,
}

impl ReportedQuestionReasonSummary {
    /// Get the reasons people reported a question for a given question.
    pub async fn get_reasons_reported(id:QuestionID)  -> Result<ReportedQuestionReasonSummary,QuestionError> {
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        let (num_flags,censorship_status) = conn.exec_first("SELECT NumFlags,CensorshipStatus from QUESTIONS where QuestionID=?",(id.0,)).map_err(internal_error)?.ok_or(QuestionError::NoSuchUser)?;
        let mut reasons : Vec<SingleReasonSummary> = conn.exec_map("SELECT reason,COUNT(user_id) from QuestionReportedReasons where QuestionId=? group by reason",(id.0,),|(reason,count)|SingleReasonSummary{reason,count,answer:None}).map_err(internal_error)?;
        let mut reasons_from_answers = conn.exec_map("SELECT reason,answer,COUNT(user_id) from AnswerReportedReasons where QuestionId=? group by reason,answer",(id.0,),|(reason,answer,count)|SingleReasonSummary{reason,count,answer:Some(hash_from_value(answer))}).map_err(internal_error)?;
        reasons.append(&mut reasons_from_answers);
        Ok(ReportedQuestionReasonSummary{num_flags,censorship_status,reasons})
    }
}


/// Whenever a question is changed (including censorship), the change is stored in the
/// public bulletin board. Each entry contains a link to the previous entry, should it exist.
/// This effectively produces a linked list. The RTA database stores the head of this
/// list, as the "version".
///
/// The linked list may get disrupted by censorship. The censorship commands provide enough
/// information in their logs to reconstruct whatever is reconstructable.
///
/// This structure effectively contains the linked list.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct QuestionHistory {
    /// The linked list of BB entries, most recent first.
    history : Vec<QuestionHistoryElement>
}

/// A single entry in the bulletin board.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct QuestionHistoryElement {
    /// The bulletin board id of this element.
    id : HashValue,
    timestamp : Timestamp,
    /// The action (value in the BB). None if it has been censored.
    #[serde(skip_serializing_if = "Option::is_none",default)]
    action : Option<LogInBulletinBoard>,
}

impl QuestionHistory {
    /// Given a question, get its history from the bulletin board.
    pub async fn lookup(question_id:QuestionID) -> Result<QuestionHistory,QuestionError> {
        // first load the question record from the database to get the head of the linked list.
        // this is somewhat overkill as we only want the version.
        let question_info = QuestionInfo::lookup(question_id).await?.ok_or_else(||QuestionError::QuestionDoesNotExist)?;
        let mut next_version = Some(question_info.version);
        let bb = get_bulletin_board().await;
        let mut history : Vec<QuestionHistoryElement> = Vec::new();
        let mut censored : HashMap<HashValue,Option<HashValue>> = HashMap::new(); // a map from censored entries to their predecessors.
        while let Some(bb_id) = next_version.take() {
            let bb_contents = bb.get_hash_info(bb_id).map_err(bulletin_board_error)?;
            if let HashSource::Leaf(LeafHashHistory{data,timestamp}) = bb_contents.source {
                let action= if let Some(serialized_data) = data {
                    let found_action : LogInBulletinBoard = serde_json::from_str(&serialized_data).map_err(|_|{println!("Could not decode json found in Bulletin board : {}",&serialized_data); QuestionError::BulletinBoardHistoryIsCorrupt})?;
                    next_version = match &found_action {
                        LogInBulletinBoard::NewQuestion(_) => None,
                        LogInBulletinBoard::EditQuestion(q) => Some(q.prior),
                        LogInBulletinBoard::ReportQuestion(r) => Some(r.prior),
                        LogInBulletinBoard::CensorQuestion(c ) => {
                            for h in &c.removed { censored.insert(h.id,h.prior); }
                            Some(c.prior)
                        }
                        LogInBulletinBoard::PlainTextVoteQuestion(v) => Some(v.prior),
                        _ => { println!("Unexpected action found in Bulletin board"); return Err(QuestionError::BulletinBoardHistoryIsCorrupt) }
                    };
                    Some(found_action)
                } else { // censored.
                    next_version = censored.remove(&bb_id).ok_or(QuestionError::BulletinBoardHistoryIsCorrupt)?; // should know about the censorship.
                    None
                };
                history.push(QuestionHistoryElement{id:bb_id,timestamp,action})
            } else { println!("Bulletin board version chain includes a non-leaf node");  return Err(QuestionError::BulletinBoardHistoryIsCorrupt); }
        }
        if !censored.is_empty() {
            println!("Censored list is not empty when evaluating history.") // shouldn't happen.
        }
        Ok(QuestionHistory{history})
    }
}

