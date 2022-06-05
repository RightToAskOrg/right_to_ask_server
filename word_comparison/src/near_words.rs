use crate::word::{WordIndex, WordSource};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Debug, PartialEq,Copy, Clone)]
pub struct WordAndValue {
    pub word : WordIndex,
    pub value : f32,
}



impl Eq for WordAndValue {
}
impl Ord for WordAndValue {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.value.partial_cmp(&other.value).unwrap() {
            Ordering::Equal => self.word.cmp(&other.word),
            res => res,
        }
    }
}
impl PartialOrd for WordAndValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

/// Stores the smallest n values
pub struct SmallestN {
    n : usize,
    values : BinaryHeap<WordAndValue>,
}

impl SmallestN {
    pub fn new(n:usize) -> Self {
        SmallestN{n,values:BinaryHeap::with_capacity(n+1)}
    }

    pub fn add(&mut self,w:WordAndValue) {
        if self.values.len()<self.n || (w.value<self.values.peek().unwrap().value && {self.values.pop(); true}) {
            self.values.push(w);
        }
    }

    pub fn into_sorted_vec(self) -> Vec<WordAndValue> { self.values.into_sorted_vec() }
}

pub fn print_near_words_vec<W:WordSource>(words:&W, v:&[WordAndValue], reversed_sign:bool) -> String {
    let mut res = String::new();
    for e in v {
        res.push('\t');
        res.push_str(words.word(e.word));
        res.push('\t');
        let value = if reversed_sign { -e.value } else { e.value };
        res.push_str(&format!("{:.4}",value))
    }
    res
}
