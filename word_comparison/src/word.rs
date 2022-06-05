use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use std::io::BufRead;
use std::ops::Range;
use std::iter::Map;

/// Words are represented by a lookup table. This is an index into that table. More common words are "lesser" by the Ord trait.
#[derive(Eq, PartialEq,Debug,Ord, PartialOrd,Copy, Clone,Hash)]
pub struct WordIndex(pub u32);

pub trait WordSource {
    /// The number of words
    fn len(&self) -> usize;
    /// The text of the word
    fn word(&self,index:WordIndex) -> &str ;
    /// The index of the word. Smaller values are more common.
    fn index(&self,word:&str) -> Option<WordIndex> ;
    /// Returns an iterator over all words in order most common to least common.
    fn all_indices(&self) -> Map<Range<usize>, fn(usize) -> WordIndex> {
        (0..self.len()).map(|i|WordIndex(i as u32))
    }
}


pub struct MemoryWords {
    words : Vec<String>,
    lookup : HashMap<String,WordIndex>,
}

impl WordSource for MemoryWords {
    fn len(&self) -> usize { self.words.len() }
    fn word(&self,index:WordIndex) -> &str { &self.words[index.0 as usize]}

    fn index(&self,word:&str) -> Option<WordIndex> { self.lookup.get(word).cloned() }
}
impl MemoryWords {

    fn add(&mut self,s:&str) -> WordIndex {
        let res = WordIndex(self.words.len() as u32);
        self.words.push(s.to_string());
        self.lookup.insert(s.to_string(),res);
        res
    }
}

pub struct WordVec {
    vec : Vec<f64>,
    mag : f64
}

fn dot_product(v1:&[f64],v2:&[f64]) -> f64 {
    assert_eq!(v1.len(),v2.len());
    let mut res = 0.0;
    for i in 0..v1.len() {
        res+=v1[i]*v2[i];
    }
    res
}
fn distance(v1:&[f64],v2:&[f64]) -> f64 {
    assert_eq!(v1.len(),v2.len());
    let mut res = 0.0;
    for i in 0..v1.len() {
        let d = v1[i]-v2[i];
        res+=d*d;
    }
    res.sqrt()
}


impl WordVec {
    //fn len(&self) -> usize { self.vec.len() }

    pub fn dot_product(&self,v2:&WordVec) -> f64 {
        dot_product(&self.vec,&v2.vec)
    }

    pub fn cosine(&self,v2:&WordVec) -> f64 {
        self.dot_product(v2)/(self.mag*v2.mag)
    }

    pub fn distance(&self,v2:&WordVec) -> f64 {
        distance(&self.vec,&v2.vec)
    }

    pub fn new(vec:Vec<f64>) -> Self {
        let mag = dot_product(&vec,&vec).sqrt();
        WordVec{ vec, mag }
    }
}


/// same indices as Words.
pub struct WordVecs {
    vecs : Vec<WordVec>,
}

impl WordVecs {
    pub fn get(&self,word:WordIndex) -> &WordVec { &self.vecs[word.0 as usize] }
}

/// Read a glove format file, up to max_words if not None.
pub fn read_glove<P:AsRef<Path>>(path:P,max_words:Option<usize>) -> std::io::Result<(MemoryWords, WordVecs)> {
    let file = File::open(path)?;
    let mut words = MemoryWords { words: vec![], lookup: Default::default() };
    let mut wordvecs=WordVecs{ vecs: vec![] };
    for line in std::io::BufReader::new(file).lines() {
        let line = line?;
        let line = line.split(' ').collect::<Vec<_>>();
        let word = line[0];
        words.add(word);
        let nums : Vec<f64> = line[1..].iter().map(|l|l.parse().unwrap()).collect();
        wordvecs.vecs.push(WordVec::new(nums));
        if let Some(max) = max_words {
            if words.len()==max { break; }
        }
    }
    Ok((words,wordvecs))
}