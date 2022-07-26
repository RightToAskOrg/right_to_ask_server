//! Dealing with censorship of questions. Could conceivably be part of question.rs.

use std::collections::HashMap;
use merkle_tree_bulletin_board::hash::HashValue;
use merkle_tree_bulletin_board::hash_history::{HashSource, LeafHashHistory, Timestamp, timestamp_now};
use mysql::prelude::Queryable;
use mysql::TxOpts;
use crate::database::{get_bulletin_board, get_rta_database_connection, LogInBulletinBoard, remove_question_from_comparison_database};
use crate::question::{bulletin_board_error, internal_error, LastQuestionUpdate, modify_question_database_version_and_time, QuestionError, QuestionID, QuestionInfo};
use crate::signing::ClientSigned;
use serde::{Serialize, Deserialize};

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
}

impl CensorQuestionCommand {
    pub async fn censor_question(&self) -> Result<HashValue,QuestionError> {
        let question_info = QuestionInfo::lookup(self.question_id).await?.ok_or_else(||QuestionError::QuestionDoesNotExist)?;  // Makes sure the question exists and is not censored already.
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
                if !question_info.non_defining.answers.iter().any(|a|a.version==Some(answer_id) && !a.censored) { return Err(QuestionError::NotAnUncensoredAnswer)}
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
            transaction.exec_drop("update Answer set censored=true where version=?", (answer_id.0,)).map_err(internal_error)?;

        } else { // censor the whole question
            transaction.exec_drop("update QUESTIONS set censored=true where QuestionID=?", (self.question_id.0,)).map_err(internal_error)?;
        }
        transaction.commit().map_err(internal_error)?;
        // TODO it would make sense to put some message in the BB saying that the just posted entry did not make it into the database for some reason if there were an error above.
        for remove in removed { // don't censor things until stored in the database otherwise we will be unpappy.
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
}

/// The structure posted to the bulletin board in response to an ReportQuestionCommand.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct ReportQuestionCommandPostedToBulletinBoard {
    pub command : ClientSigned<ReportQuestionCommand>,
    /// This will be a link to the prior node in the database.
    pub prior : LastQuestionUpdate,
}

impl ReportQuestionCommand {
    pub async fn report_question(command:&ClientSigned<ReportQuestionCommand>) -> Result<HashValue,QuestionError> {
        let question_info = QuestionInfo::lookup(command.parsed.question_id).await?.ok_or_else(||QuestionError::QuestionDoesNotExist)?;
        let timestamp = timestamp_now().map_err(internal_error)?;
        let for_bb = ReportQuestionCommandPostedToBulletinBoard{
            command : command.clone(),
            prior : question_info.version,
        };
        let response = LogInBulletinBoard::ReportQuestion(for_bb).log_in_bulletin_board().await.map_err(bulletin_board_error)?;
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        let mut transaction = conn.start_transaction(TxOpts::default()).map_err(internal_error)?;
        modify_question_database_version_and_time(&mut transaction,command.parsed.question_id,response,Some(question_info.version),timestamp).await?;
        // TODO record this in the database.
        transaction.commit().map_err(internal_error)?;
        // TODO it would make sense to put some message in the BB saying that the just posted entry did not make it into the database for some reason if there were an error above.
        Ok(response)
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
                    let found_action : LogInBulletinBoard = serde_json::from_str(&serialized_data).map_err(|_|QuestionError::BulletinBoardHistoryIsCorrupt)?;
                    next_version = match &found_action {
                        LogInBulletinBoard::NewQuestion(_) => None,
                        LogInBulletinBoard::EditQuestion(q) => Some(q.prior),
                        LogInBulletinBoard::ReportQuestion(r) => Some(r.prior),
                        LogInBulletinBoard::CensorQuestion(c ) => {
                            for h in &c.removed { censored.insert(h.id,h.prior); }
                            Some(c.prior)
                        }
                        _ => { return Err(QuestionError::BulletinBoardHistoryIsCorrupt) }
                    };
                    Some(found_action)
                } else { // censored.
                    next_version = censored.remove(&bb_id).ok_or(QuestionError::BulletinBoardHistoryIsCorrupt)?; // should know about the censorship.
                    None
                };
                history.push(QuestionHistoryElement{id:bb_id,timestamp,action})
            } else { return Err(QuestionError::BulletinBoardHistoryIsCorrupt); }
        }
        if !censored.is_empty() {
            println!("Censored list is not empty when evaluating history.") // shouldn't happen.
        }
        Ok(QuestionHistory{history})
    }
}