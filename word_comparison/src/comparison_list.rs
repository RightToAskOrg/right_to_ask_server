use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use crate::listed_keywords::ListedKeywords;
use crate::word::WordIndex;
use crate::word_file::WordsInFile;
use crate::sentences::{TokenizedSentence, SentencePart};
use serde::{Serialize,Deserialize};
use crate::database_backend::{InternalQuestionId, ParsedQuestion, WordComparisonDatabaseBackend};


/// Add a new question to the database.
pub fn add_question<B:WordComparisonDatabaseBackend>(backend:&mut B,question:&str,external_id:B::ExternalQuestionId,words:&WordsInFile,keywords:&ListedKeywords) -> anyhow::Result<InternalQuestionId> {
    let question = question.replace('\n'," ");
    let parsed_question = ParsedQuestion::new(question, words, keywords);
    backend.add_sentence_and_components(external_id,parsed_question)
}


/// the score for a Keyword - one of the words from the ListedKeywords list.
const SCORE_KEYWORD : f64 = 10.0;
/// The score for a matching word that is not in either the ListedKeywords or general lexicon. Possibly a hashtag?
const SCORE_UNIQUE : f64 = 10.0;
/// The score for a word in the general vocabulary. More obscure words are worth more points.
fn score_known(word : WordIndex) -> f64 {
    if word.0 < 100 { 1.0 }
    else if word.0 < 500 { 2.0 }
    else if word.0 < 1000 { 3.0 }
    else if word.0 < 10000 { 4.0 }
    else if word.0 < 100000 { 6.0 }
    else { 8.0 }
}


/// Find questions in the database that are similar to this one.
pub fn find_similar_in_database<B:WordComparisonDatabaseBackend>(backend:&mut B, question:&str, words:&WordsInFile, keywords:&ListedKeywords) -> anyhow::Result<Vec<ScoredIDs<B::ExternalQuestionId>>> {
    let tokenized = TokenizedSentence::tokenize(&question,words,keywords);
    // println!();
    // tokenized.explain(words,keywords);
    let mut scores = SentenceScores::default();
    for token in &tokenized.parts {
        match token {
            SentencePart::Listed(word) => {
                scores.add_maybe(backend.sentences_containing_listed_word(*word)?,SCORE_KEYWORD);
            },
            SentencePart::Known(word) => {
                if word.0>100 {
                    let score = score_known(*word);
                    let mut avoid_twice = HashSet::new();
                    scores.add_maybe_avoid_counting_twice(backend.sentences_containing_general_lexicon_word(*word)?,score,&mut avoid_twice);
                    for e in words.synonyms(*word) {
                        scores.add_maybe_avoid_counting_twice(backend.sentences_containing_general_lexicon_word(e.word)?,score*e.value as f64,&mut avoid_twice);
                    }
                }
            }
            SentencePart::Unknown(word) => {
                scores.add_maybe(backend.sentences_containing_unknown_word(word)?,SCORE_UNIQUE);
            },
        }
    }
    let internal_ids = scores.extract_ordered();
    backend.convert_internal_ids_to_external_ids(internal_ids)
}



#[derive(Default)]
struct SentenceScores {
    scores : HashMap<InternalQuestionId,f64>,
}

#[derive(Copy, Clone,Debug,Serialize,Deserialize)]
pub struct ScoredIDs<ID> {
    pub id : ID,
    pub score : f64,
}


impl SentenceScores {
    /// Add a set of questions containing this id.
    /// Assign the given number of points.
    fn add_maybe(&mut self,qs:Option<Cow<Vec<InternalQuestionId>>>,points:f64) {
        if let Some(qs) = &qs {
            for &q in qs.as_ref() {
                *self.scores.entry(q).or_insert(0.0)+=points;
            }
        }
    }

    /// like add_maybe, but
    /// Don't assign points if it is in the avoid_twice optional list (and add ones that you do add points for).
    fn add_maybe_avoid_counting_twice(&mut self,qs:Option<Cow<Vec<InternalQuestionId>>>,points:f64,avoid_twice:&mut HashSet<InternalQuestionId>) {
        if let Some(qs) = qs {
            for &q in qs.as_ref() {
                if avoid_twice.insert(q) {
                    *self.scores.entry(q).or_insert(0.0)+=points;
                }
            }
        }
    }

    pub fn extract_ordered(self) -> Vec<ScoredIDs<InternalQuestionId>> {
        let mut res : Vec<ScoredIDs<InternalQuestionId>> = self.scores.iter().map(|(&id,&score)|ScoredIDs{id,score}).collect();
        res.sort_by(|a,b|b.score.partial_cmp(&a.score).unwrap());
        res
    }
}