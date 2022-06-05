//! Code to deal with sentences rather than words.

use crate::listed_keywords::{ListedKeywordIndex, ListedKeywords};
use crate::word::{WordIndex, WordSource};
use crate::word_file::WordsInFile;

pub enum SentencePart {
    Listed(ListedKeywordIndex),
    Known(WordIndex),
    Unknown(String),
}

impl SentencePart {
    pub fn explain(&self,words:&WordsInFile,keywords:&ListedKeywords) -> String {
        match self {
            SentencePart::Listed(keyword) => format!("Keyword {} : {}",keyword.0,keywords.canonical(*keyword)),
            SentencePart::Known(word) => format!("Word {} : {}",word.0,words.word(*word)),
            SentencePart::Unknown(token) => format!("Unknown {}",token),
        }
    }
}

pub struct TokenizedSentence {
    pub parts : Vec<SentencePart>,
}

/// Get the length of the next token. 0 if starts with whitespace.
/// look for something terminated by whitespace, and remove trailing punctuation.
fn len_next_token(s:&str) -> usize {
    let mut last_was_not_punctuation = false;
    let mut last_start_punctuation = 0;
    for (pos,c) in s.char_indices() {
        if c.is_whitespace() {
            return if last_was_not_punctuation || last_start_punctuation==0 { pos } else { last_start_punctuation }
        } else if c.is_alphanumeric() {
            last_was_not_punctuation=true;
        } else {
            if last_was_not_punctuation {
                last_was_not_punctuation=false;
                last_start_punctuation=pos;
            }
        }
    }
    if last_was_not_punctuation || last_start_punctuation==0 { s.len() } else { last_start_punctuation }
}

impl TokenizedSentence {
    pub fn tokenize(text:&str, words:&WordsInFile, keywords:&ListedKeywords) -> Self {
        let mut parts = vec![];
        let lower_case = text.to_lowercase();
        let mut left = lower_case.trim();
        while !left.is_empty() {
            let used = if let Some((keyword,used))=keywords.find_keyword_starting(left) {
                parts.push(SentencePart::Listed(keyword));
                used
            } else if let Some((keyword,used))=words.index_starting(left) {
                parts.push(SentencePart::Known(keyword));
                used
            } else {
                let len = len_next_token(left);
                parts.push(SentencePart::Unknown(left[..len].to_string()));
                len
            };
            left=left[used..].trim_start();
        }
        TokenizedSentence {parts}
    }

    pub fn explain(&self,words:&WordsInFile,keywords:&ListedKeywords) {
        for part in &self.parts {
            println!(" {}",part.explain(words,keywords));
        }
    }
}