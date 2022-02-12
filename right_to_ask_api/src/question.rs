//! Information about a question. Includes APIs for modifying the database.

// Functions here
// - submit New Question
// - edit existing question
// - look up current info on a specific question.
// - TODO look for similar questions


use serde::{Serialize, Deserialize};
use merkle_tree_bulletin_board::hash::HashValue;
use merkle_tree_bulletin_board::hash_history::Timestamp;
use sha2::{Digest, Sha256};
use crate::person::PersonUID;
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
    AnswerTooLong,
    BackgroundTooLong,
    SameQuestionSubmittedRecently,
    OnlyAuthorCanChangeBackground,
    // someone who has to be an MP was not.
    NotMP(PersonUID),
    /// The provided question_id does not exist.
    QuestionDoesNotExist,
    /// The provided last_update hash is not the current last update
    LastUpdateIsNotCurrent,
}

/// The maximum number of characters in a question.
const MAX_QUESTION_LENGTH : usize = 200;
const MIN_QUESTION_LENGTH : usize = 10;
const MAX_BACKGROUND_LENGTH : usize = 200;
const MAX_ANSWER_LENGTH : usize = 1000;



/*************************************************************************
                         COMMON DATA STRUCTURES
 *************************************************************************/




/// The fields that never change for a question and are used to define the hash for this field.
///
///
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct QuestionDefiningFields {
    /// The UID of the person asking the question
    author : PersonUID,
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

pub enum Permissions {
    WriterOnly,
    Others,
    NoChange
}

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
    pub background : Option<String>,
    /// Validity: must be an MP or a user. (If a user is associated with an MP then tag for the MP.)
    /// Permission: defined by who_should_ask_the_question_permissions
    /// Merge rule: TODO consider whether the version check changes this. Eliminate duplicates (including with values already present). If the total number of values doesn't exceed the limit, accept. If the limit has already been exceeded, reject. If it hasn't, but would if this update was accepted, send a merge request back to the client (pick at most m out of the n you tried to submit...).  Note that this might cause cascading merges that need to be manually resolved, but that's less trouble than allowing locks.
    pub mp_who_should_ask_the_question : Vec<PersonUID>,
    /// Permission: must be from the question-writer
    /// Merge rule: overwrite, unless it's 'NoChange'
    pub who_should_ask_the_question_permissions : Permissions,
    /// TODO is this a person? - Ans: *** It's either an MP or a user. Think about this because multiple users may be attached to one MP, so we want in
    /// that case to ref the MP not the (perhaps multiple) user(s).
    /// Validity : see above TODO
    /// Permission: Defined by who_should_answer_the_question_permissions
    /// Merge rule : same as mp_who_should_ask_the_question
    pub entity_who_should_answer_the_question : Vec<PersonUID>,
    /// Permission: must be from the question-writer
    /// Merge rule: overwrite, unless it's 'NoChange'
    pub who_should_answer_the_question_permissions : Permissions,
    /// Validity : character length; answerer must match the sig.
    /// Permission Must be from MP. TODO Can an entity_who_should_answer_the_question ... answer the question?
    /// VT: Counterintuitively, No. I am assuming that public authorities won't join the system, only MPs. And then it seems only fair to let other
    /// MPs answer, even if they are not the person tagged in the system.
    /// Merge rule : just add. No problems with multiple answers from different people. Or even multiple answers from the same person, e.g. MP day 1: "I will ask that for you." Day 3: "They said 42."
    pub answers : Vec<QuestionAnswer>,
    /// Permission: must be from the question-writer
    /// Merge rule : may be changed from false to true.
    pub answer_accepted : bool,
    /// Validity : domain must be aph.gov.au, parliament.vic.gov.au, etc. (preloaded permit-list - note that url sanitation is nontrivial). TODO work out nontrivial stuff
    /// Permission: n/a TODO can anyone add? VT: Yes. We'll need to check urls.
    /// Merge rule : same as mp_who_should_ask_the_question
    pub hansard_link : Vec<HansardLink>,
    /// Validity: must be a pre-existing Question-Id
    /// Permissions: TODO: think about whether only the question-writer can write a followup. VT: I think yes.
    /// Merge rule:  Reject updates unless currently blank. TODO check. (should this be a list? probably simpler if not). VT: Agree. Let's just have one at a time. People can make a linear chain. (Twitter actually allows a tree, and it's a complete pain. Lines are better.)
    pub is_followup_to : Option<QuestionID>,
    /// TODO: VT I think we want expiry dates, probably with a short default (2 weeks?) Agree we don't need keywords or categories.
    // Note that I have not included
    /*
    Note that I have not included, from the tech docs,
     * Keywords: List(String)
        * Validity: short list of short words
        * Permission: n/a
        * Merge rule : mp_who_should_ask_the_question
     * Category: List(Topics)
        * Validity: short list of pre-loaded topics
        * Permission: n/a
        * Merge rule : mp_who_should_ask_the_question
     * Expiry_Date: date
        * Validity: must be later than Upload_Timestamp (and within ?? a year)
        * Permission: must from the question-writer

     */
}

#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct QuestionAnswer {
    /// must be a MP
    answered_by : PersonUID,
    answer : String,
}

///  domain must be aph.gov.au, parliament.vic.gov.au, etc. (preloaded permit-list - note that url sanitation is nontrivial).
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct HansardLink {
    pub url : String, // Should this be more structured?
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
    pub non_defining_fields : QuestionNonDefiningFields,
}

/// The message posted to the bulletin board
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct NewQuestionCommandPostedToBulletinBoard {
    pub command : ClientSigned<NewQuestionCommand>,
    pub timestamp : Timestamp,
    pub uid : QuestionID,
}

/// Successful return from posting a new question.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct NewQuestionCommandResponse {
    pub uid : QuestionID,
    pub last_update : LastQuestionUpdate,
}

impl NewQuestionCommand {
    /// API function to add a question to the
    pub fn add_question(question:&ClientSigned<NewQuestionCommand>) -> Result<NewQuestionCommandResponse,QuestionError> {
        if question.parsed.question_text.len()>MAX_QUESTION_LENGTH { return Err(QuestionError::QuestionTooLong); }
        if question.parsed.question_text.len()<MIN_QUESTION_LENGTH { return Err(QuestionError::QuestionTooShort); }
        if let Some(background) = &question.parsed.non_defining_fields.background {
            if background.len()>MAX_BACKGROUND_LENGTH { return Err(QuestionError::BackgroundTooLong); }
        }
        if question.parsed.non_defining_fields.answers.iter().any(|a|a.answer.len()>MAX_ANSWER_LENGTH)  { return Err(QuestionError::AnswerTooLong); }
        if question.parsed.non_defining_fields.answers.iter().any(|a|a.answer.len()>MAX_ANSWER_LENGTH)  { return Err(QuestionError::AnswerTooLong); }
        todo!() // lots of stuff to do.
    }
}




/*************************************************************************
                       QUERY INFO ABOUT A QUESTION
 *************************************************************************/





/// Information about a question.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct QuestionInfo {
    defining : QuestionDefiningFields,
    non_defining : QuestionNonDefiningFields,
    uid : QuestionID,
    version : LastQuestionUpdate,
    last_modified : Timestamp,
}


impl QuestionInfo {
    /// Get information about a question from the database.
    pub async fn lookup(_uid:QuestionID) -> Result<QuestionInfo,QuestionError> {
        todo!()
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
    pub uid : QuestionID,
    /// The hash value defining the last update done to the question. This is checked to prevent multiple edits.
    /// TODO Should it be an option? Maybe you don't care if there are clashes?
    pub version : LastQuestionUpdate,
    /// the actual work... This contains *updates* to be added to the non-defining fields. Empty fields are to be left unchanged.
    pub edits : QuestionNonDefiningFields,
}

/// The structure posted to the bulletin board in response to an EditQuestionCommand.
pub struct EditQuestionCommandPostedToBulletinBoard {
    pub command : ClientSigned<EditQuestionCommand>,
    /// TODO Do we want this? The bulletin board will keep a timestamp anyway.
    pub timestamp : Timestamp,
    /// This will be a link to the prior node in the database. This will be a duplicate of [self.command.parsed.version], but easier to access, and future proof against a change in design where version is not included.
    pub prior : LastQuestionUpdate,
}


impl EditQuestion {

    /// Try to perform the edit.
    /// If success, return the new last edit.
    pub async fn edit(_command:&ClientSigned<EditQuestionCommand>) -> Result<LastQuestionUpdate,QuestionError> {
        todo!()
    }
}






/*************************************************************************
                        FLAG A QUESTION
 *************************************************************************/





/// This is used to flag a question as deserving of censorship.
/// Its intention is to allow people to inform the server of questions that are threatening, abusive, etc.
/// Exactly how this translates into a censor instruction to the BB is undefined - for example, it could be automatic based on the fraction of viewers who flag it, or it could require human intervention.
/// There is still a lot of work to go here.
#[derive(Serialize,Deserialize)]
pub struct FlagQuestion {
    uid : QuestionID,
    /// complaint details. Should this be a string, or more structured?
    complaint : String,
}