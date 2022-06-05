use std::borrow::Cow;
use std::collections::HashSet;
use serde::{Serialize,Deserialize};
use crate::comparison_list::ScoredIDs;
use crate::listed_keywords::{ListedKeywordIndex, ListedKeywords};
use crate::sentences::{SentencePart, TokenizedSentence};
use crate::word::WordIndex;
use crate::word_file::WordsInFile;

/// Some identifier used internally to define sentences. Could be a simple integer in a database table. Done separately in case the external word ID is long.
#[derive(Copy, Clone,Eq, PartialEq,Debug,Hash,Serialize,Deserialize)]
pub struct InternalQuestionId(pub u32);


/// There needs to be a database containing all the sentences.
/// For testing and demo an in-memory database is provided, but long term a more scalable solution is needed.
/// Whatever the backend, it should implement the following commands.
pub trait WordComparisonDatabaseBackend {
    /// Some identifier used to define sentences to the external world. Generally something that the external world imposes.
    type ExternalQuestionId : Clone;

    /// Find sentences containing the listed word (one of the curated words)
    fn sentences_containing_listed_word(&self,word:ListedKeywordIndex) -> anyhow::Result<Option<Cow<Vec<InternalQuestionId>>>>;
    /// Find sentences containing a word in the general lexicon.
    fn sentences_containing_general_lexicon_word(&self,word:WordIndex) -> anyhow::Result<Option<Cow<Vec<InternalQuestionId>>>>;
    /// Find sentences containing a unknown word. Possibly a typo, possibly vital hashtag or jargon.
    fn sentences_containing_unknown_word(&self,word:&str) -> anyhow::Result<Option<Cow<Vec<InternalQuestionId>>>>;

    /// For a sentence that has been divided up into tokens, record said tokens as associated with this sentence.
    fn add_sentence_and_components<'a>(&mut self,external_id:Self::ExternalQuestionId,parsed:ParsedQuestion) -> anyhow::Result<InternalQuestionId>;

    /// Get all questions in the database. Could be slow! Just used for debugging.
    fn get_all_questions(&self) -> anyhow::Result<Vec<String>>;

    /// Get the text associated with a question. Mainly used for debugging.
    fn lookup(&self,id:Self::ExternalQuestionId) -> anyhow::Result<Option<String>>;

    /// Convert the IDs in a vector of ScoredIDs from internal ids to external ids.
    fn convert_internal_ids_to_external_ids(&self,internal_ids:Vec<ScoredIDs<InternalQuestionId>>) -> anyhow::Result<Vec<ScoredIDs<Self::ExternalQuestionId>>>;

    /// Delete everything in the database and reinitialize as an empty database.
    /// To recreate the database, call this, then call [crate::comparison_list::add_question] with each question that should be in it.
    fn clear_all_reinitialize(&mut self) -> anyhow::Result<()>;
}


pub struct ParsedQuestion {
    pub(crate) question : String,
    pub(crate) keywords : HashSet<ListedKeywordIndex>,
    pub(crate) known_words : HashSet<WordIndex>,
    pub(crate) unique_words : HashSet<String>,
}



impl ParsedQuestion {
    pub fn new(question : String,words:&WordsInFile,keywords:&ListedKeywords) -> Self {
        let tokenized = TokenizedSentence::tokenize(&question,words,keywords);
        let mut keywords = HashSet::new();
        let mut known_words = HashSet::new();
        let mut unique_words = HashSet::new();
        for token in tokenized.parts {
            match token {
                SentencePart::Listed(word) => {keywords.insert(word);}
                SentencePart::Known(word) => { if word.0 > 400 { known_words.insert(word); }}
                SentencePart::Unknown(word) => {unique_words.insert(word);}
            }
        }
        ParsedQuestion { question, keywords, known_words, unique_words }
    }
}
