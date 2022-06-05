//! Some listed keywords, saying that "Covid", "Covid-19", "Covid 19" and "Cononavirus" are all the same concept.


use std::path::Path;
use csv::ReaderBuilder;

pub struct ListedKeyword(pub Vec<String>);

#[derive(Copy, Clone,Debug,Eq, PartialEq,Hash)]
pub struct ListedKeywordIndex(pub usize);

pub struct ListedKeywords {
    keywords : Vec<ListedKeyword>
}

impl ListedKeywords {
    pub const STD_LOCATION : &'static str = "ListedKeywords.csv";
    pub fn load<P:AsRef<Path>>(path:P) -> std::io::Result<Self> {
        let mut keywords = vec![];
        let mut reader = ReaderBuilder::new().flexible(true).has_headers(false).from_path(path)?;
        for result in reader.records() {
            let record = result?;
            let keyword = ListedKeyword(record.iter().map(|s|s.to_string()).collect());
            keywords.push(keyword);
        }
        Ok(ListedKeywords{keywords})
    }

    /// find a keyword that s starts with, returning the found keyword and the length consumed.
    pub fn find_keyword_starting(&self,s:&str) -> Option<(ListedKeywordIndex,usize)> {
        for i in 0..self.keywords.len() {
            if let Some(used) = self.keywords[i].find_keyword_starting(s) {
                return Some((ListedKeywordIndex(i),used))
            }
        }
        None
    }

    /// Get a canonical example of this word.
    pub fn canonical(&self,index:ListedKeywordIndex) -> &str {
        self.keywords[index.0].0[0].as_str()
    }
}

impl ListedKeyword {
    /// find a keyword that s starts with, returning the length consumed.
    pub fn find_keyword_starting(&self,s:&str) -> Option<usize> {
        for word in &self.0 {
            if s.len()>=word.len() {
                if s.as_bytes()[..word.len()].eq_ignore_ascii_case(word.as_bytes()) { return Some(word.len()) }
            }
        }
        None
    }
}