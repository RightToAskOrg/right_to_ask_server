use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};
use merkle_tree_bulletin_board::hash::HashValue;
use once_cell::sync::{Lazy};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use crate::committee::CommitteeInfo;
use crate::mp::MPSpec;
use crate::parse_upcoming_hearings::UpcomingHearing;

/// Represent a file on disk containing a JSON encoded data structure that may be frequently
pub struct CommonFile<T> {
    path : PathBuf,
    contents : Mutex<Option<CommonFileContents<T>>>,
}

const COMMON_BASE_DIR : &'static str = "data";

impl <T:DeserializeOwned> CommonFile<T> {
    fn new(file:&str) -> Self {
        let path = PathBuf::from_str(COMMON_BASE_DIR).unwrap().join(file);
        CommonFile { path, contents:Mutex::new(None) }
    }

    /// mark the current data as invalid. It will be reloaded from disk.
    pub fn reset(&self) {
        let mut lock = self.contents.lock().unwrap();
        (*lock)=None;
    }


    /// Get a lock structure with a guaranteed ability to call .as_ref().unwrap().
    fn get_loaded(&self) -> anyhow::Result<MutexGuard<'_, Option<CommonFileContents<T>>>> { // -> Arc<CommonFileContents<T>>>
        let mut lock = self.contents.lock().unwrap();
        if (*lock).is_none() {
            (*lock)=Some(CommonFileContents::load(&self.path)?)
        }
        //Ok(lock.as_ref().unwrap().clone())
        Ok(lock)
    }

    /// get the hash value for the data
    pub fn get_hash(&self) -> anyhow::Result<HashValue> {
        Ok(self.get_loaded()?.as_ref().unwrap().hash)
    }
    /// get the actual raw data
    pub fn get_data(&self) -> anyhow::Result<Arc<Vec<u8>>> {
        Ok(self.get_loaded()?.as_ref().unwrap().data.clone())
    }
    /// get the interpreted data
    pub fn get_interpreted(&self) -> anyhow::Result<Arc<T>> {
        Ok(self.get_loaded()?.as_ref().unwrap().interpreted.clone())
    }
}
struct CommonFileContents<T> {
    hash : HashValue,
    data : Arc<Vec<u8>>,
    interpreted : Arc<T>,
}

impl <T:DeserializeOwned> CommonFileContents<T> {
    fn load(path:&PathBuf) -> anyhow::Result<CommonFileContents<T>> {
        let data = Arc::new(std::fs::read(path)?);
        let mut hasher = Sha256::default();
        hasher.update(&*data);
        let hash = HashValue(<[u8; 32]>::from(hasher.finalize()));
        let interpreted = Arc::new(serde_json::from_slice(&data)?);
        Ok(CommonFileContents{ data, hash, interpreted })
    }
}

pub static COMMITTEES: Lazy<CommonFile<Vec<CommitteeInfo>>> = Lazy::new(||CommonFile::new("upcoming_hearings/committees.json"));
pub static HEARINGS: Lazy<CommonFile<Vec<UpcomingHearing>>> = Lazy::new(||CommonFile::new("upcoming_hearings/hearings.json"));
pub static MPS: Lazy<CommonFile<MPSpec>> = Lazy::new(||CommonFile::new("MP_source/MPs.json"));

