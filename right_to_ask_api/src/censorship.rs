//! Dealing with censorship of questions. Could conceivably be part of question.rs.

use merkle_tree_bulletin_board::hash::HashValue;
use mysql::prelude::Queryable;
use crate::database::{get_bulletin_board, get_rta_database_connection, LogInBulletinBoard, remove_question_from_comparison_database};
use crate::question::{bulletin_board_error, internal_error, QuestionError, QuestionID, QuestionInfo};
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
    pub question_id : QuestionID,
}

impl CensorQuestionCommand {
    pub async fn censor_question(&self) -> Result<HashValue,QuestionError> {
        let question_info = QuestionInfo::lookup(self.question_id).await?.ok_or_else(||QuestionError::QuestionDoesNotExist)?;  // Makes sure the question exists and is not censored already.
        let mut conn = get_rta_database_connection().await.map_err(internal_error)?;
        conn.exec_drop("update QUESTIONS set censored=true where QuestionID=?", (self.question_id.0,)).map_err(internal_error)?;
        remove_question_from_comparison_database(self.question_id).await.map_err(internal_error)?;
        let response = LogInBulletinBoard::CensorQuestion(self.clone()).log_in_bulletin_board().await.map_err(bulletin_board_error)?;
        if self.censor_logs {
            // TODO What exactly are we censoring? There may be multiple entries in the bulletin board for this element.
            get_bulletin_board().await.censor_leaf(question_info.version).map_err(bulletin_board_error)?;
        }
        Ok(response)
    }
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

impl ReportQuestionCommand {
    pub async fn report_question(question:&ClientSigned<ReportQuestionCommand>) -> Result<HashValue,QuestionError> {
        // TODO do something sensible. It is not clear what this is.
        let response = LogInBulletinBoard::ReportQuestion(question.clone()).log_in_bulletin_board().await.map_err(bulletin_board_error)?;
        Ok(response)
    }
}