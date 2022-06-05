//! A backend implementing database_backend done via a flatfile and memory.


use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::{File, OpenOptions, remove_file};
use std::io::BufRead;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use anyhow::anyhow;
use crate::comparison_list::ScoredIDs;
use crate::database_backend::{InternalQuestionId, ParsedQuestion, WordComparisonDatabaseBackend};
use crate::listed_keywords::{ListedKeywordIndex, ListedKeywords};
use crate::word::WordIndex;
use crate::word_file::WordsInFile;

pub struct FlatfileDatabaseBackend<Q : Clone+Display> {
    filename : PathBuf,
    questions : Vec<String>,
    external_ids : Vec<Q>,
    containing_keyword : HashMap<ListedKeywordIndex,Vec<InternalQuestionId>>,
    containing_known_word : HashMap<WordIndex,Vec<InternalQuestionId>>,
    containing_unique : HashMap<String,Vec<InternalQuestionId>>,
}

impl <Q : Clone+Display+PartialEq+FromStr> WordComparisonDatabaseBackend for FlatfileDatabaseBackend<Q> {
    type ExternalQuestionId = Q;

    fn sentences_containing_listed_word(&self, word: ListedKeywordIndex) -> anyhow::Result<Option<Cow<Vec<InternalQuestionId>>>> {
        Ok(self.containing_keyword.get(&word).map(|v|Cow::Borrowed(v)))
    }

    fn sentences_containing_general_lexicon_word(&self, word: WordIndex) -> anyhow::Result<Option<Cow<Vec<InternalQuestionId>>>> {
        Ok(self.containing_known_word.get(&word).map(|v|Cow::Borrowed(v)))
    }

    fn sentences_containing_unknown_word(&self, word: &str) -> anyhow::Result<Option<Cow<Vec<InternalQuestionId>>>> {
        Ok(self.containing_unique.get(word).map(|v|Cow::Borrowed(v)))
    }

    fn add_sentence_and_components<'a>(&mut self, external_id: Self::ExternalQuestionId,parsed:ParsedQuestion) -> anyhow::Result<InternalQuestionId> {
        let mut file = OpenOptions::new().create(true).write(true).append(true).open(&self.filename)?;
        writeln!(file, "{}\t{}",external_id,parsed.question.replace('\n'," "))?;
        Ok(self.add_work(parsed,external_id))
    }


    /// Get all questions in the database. Could be slow!
    fn get_all_questions(&self) -> anyhow::Result<Vec<String>> {
        Ok(self.questions.iter().map(|q|q.clone()).collect())
    }
    /// Get the text associated with a question. Very inefficient! But this is just for debugging.
    fn lookup(&self,id:Self::ExternalQuestionId) -> anyhow::Result<Option<String>> {
        Ok(self.external_ids.iter().position(|e|*e==id).map(|index|self.questions[index].clone()))
    }

    fn convert_internal_ids_to_external_ids(&self, internal_ids: Vec<ScoredIDs<InternalQuestionId>>) -> anyhow::Result<Vec<ScoredIDs<Self::ExternalQuestionId>>> {
        Ok(internal_ids.into_iter().map(|s|ScoredIDs{id:self.external_ids[s.id.0 as usize].clone(),score:s.score}).collect())
    }

    /// Delete everything in the database and reinitialize as an empty database.
    fn clear_all_reinitialize(&mut self) -> anyhow::Result<()> {
        self.questions.clear();
        self.external_ids.clear();
        self.containing_keyword.clear();
        self.containing_known_word.clear();
        self.containing_unique.clear();
        if Path::new(&self.filename).exists() { remove_file(&self.filename)? };
        Ok(())
    }

}

impl <Q : Clone+Display+FromStr> FlatfileDatabaseBackend<Q> {
    pub const STD_FILE_NAME : &'static str = "QuestionDatabase.txt";
    fn add_work(&mut self, question:ParsedQuestion,external_id:Q) -> InternalQuestionId {
        let id = InternalQuestionId(self.questions.len() as u32);
        fn add<K>(entry:Entry<K,Vec<InternalQuestionId>>,id:InternalQuestionId) {
            let v = entry.or_insert_with(||vec![]);
            if v.is_empty() || id!= *v.last().unwrap() { // make sure the same id is not included twice.
                v.push(id);
            }
        }
        for &word in &question.keywords {
            add(self.containing_keyword.entry(word),id);
        }
        for &word in &question.known_words {
            add(self.containing_known_word.entry(word),id);
            self.containing_known_word.entry(word).or_insert_with(||vec![]).push(id);
        }
        for word in &question.unique_words {
            add(self.containing_unique.entry(word.clone()),id);
            self.containing_unique.entry(word.clone()).or_insert_with(||vec![]).push(id);
        }
        self.questions.push(question.question);
        self.external_ids.push(external_id);
        id
    }

    /// Get a new database, initialised from text file if it exists.
    pub fn new<P:AsRef<Path>+ std::convert::AsRef<std::ffi::OsStr>>(path:P,words:&WordsInFile,keywords:&ListedKeywords) -> anyhow::Result<Self>
        where <Q as FromStr>::Err: std::error::Error + Send + Sync + 'static {
        let mut res = FlatfileDatabaseBackend{
            filename : PathBuf::from(&path),
            questions: vec![],
            external_ids: vec![],
            containing_keyword: Default::default(),
            containing_known_word: Default::default(),
            containing_unique: Default::default()
        };
        if let Ok(file) = File::open(path) {
            for line in std::io::BufReader::new(file).lines() {
                if let Some((external_id,question)) = line?.split_once('\t') {
                    let external_id = Q::from_str(external_id)?;
                    let parsed = ParsedQuestion::new(question.to_owned(),words,keywords);
                    res.add_work(parsed,external_id);
                } else {
                    return Err(anyhow!("Line in wrong format"))
                }
            }
        }
        Ok(res)
    }

    pub fn len(&self) -> usize { self.questions.len() }


}