// Utility for storing email verification codes.


use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

/// Store values associated with a key for some limited time.
pub struct TimeLimitedHashMap<K,V> {
    duration_to_store : Duration,
    map : HashMap<K,(Instant,V)>, // map from key to the time at which it should be deleted, and an associated value.
    last_size_when_gc_called : usize, // the
}

impl <K: Eq + Hash,V> TimeLimitedHashMap<K,V> {
    pub fn new(duration_to_store : Duration) -> Self {
        TimeLimitedHashMap{
            duration_to_store,
            map: Default::default(),
            last_size_when_gc_called: 0
        }
    }
    pub fn insert(&mut self,key:K,value:V) {
        self.map.insert(key,(Instant::now()+self.duration_to_store,value));
        if self.map.len()>2*self.last_size_when_gc_called { self.gc(); }
    }

    /// throw away old stuff.
    /// This is automatically called when the length of the map reaches twice the value the last time gc was called, to prevent indefinite growth.
    pub fn gc(&mut self) {
        let now = Instant::now();
        self.map.retain(|_,(t,_)|*t>now);
        self.last_size_when_gc_called = self.map.len();
    }
    
    pub fn get(&self, key:&K) -> Option<&V> {
        let without_checking_time = self.map.get(key);
        let after_checking_time = without_checking_time.and_then(|(t,v)| if *t>=Instant::now() { Some(v) } else {None});
        //println!("Found : {} time ok {}",without_checking_time.is_some(),after_checking_time.is_some());
        after_checking_time
    }
    
    pub fn remove(&mut self,key:&K) {
        self.map.remove(key);
    }
}

