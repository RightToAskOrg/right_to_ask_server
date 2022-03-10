
//! Human representatives - generalization of MPs, hereafter just referred to as MPs.


use crate::regions::{Electorate, RegionContainingOtherRegions};
pub use crate::parse_mp_lists::{update_mp_list_of_files,create_mp_list};
use serde::{Serialize,Deserialize};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use once_cell::sync::OnceCell;
use crate::parse_mp_lists::MP_SOURCE;

/// Information about a MP (or other human elected representative, e.g. senator).
/// Not all fields are known perfectly for each person.
#[derive(Serialize,Deserialize)]
pub struct MP {
    pub first_name : String,
    pub surname : String,
    pub electorate : Electorate,
    pub email : String,
    pub role : String,
    pub party : String,
}

impl MP {
    /// Get the name associated with a badge for an MP.
    /// This is `FirstName surname @emaildomain`
    pub fn badge_name(&self) -> String {
        self.first_name.to_string()+" "+&self.surname+" "+self.email.trim_start_matches(|c|c!='@')
    }
}

impl Display for MP {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {} party {} electorate {} {} {}", self.surname, self.first_name,self.party,self.electorate,self.email,self.role)
    }
}

/// Information identifying an MP.
#[derive(Serialize,Deserialize,Clone,Debug)]
pub struct MPId {
    pub first_name : String,
    pub surname : String,
    pub electorate : Electorate,
}

/// A list of MPs and some useful things for working out regions.
#[derive(Serialize,Deserialize)]
pub struct MPSpec {
    pub mps : Vec<MP>,
    pub federal_electorates_by_state : Vec<RegionContainingOtherRegions>,
    pub vic_districts : Vec<RegionContainingOtherRegions>,
}

impl MPSpec {
    fn read_from_file() -> anyhow::Result<MPSpec> {
        let dir = PathBuf::from_str(MP_SOURCE)?;
        let source = File::open(dir.join("MPs.json"))?;
        Ok(serde_json::from_reader(source)?)
    }

    /// Get the current list of MPs. Cached.
    pub fn get() -> Arc<anyhow::Result<MPSpec>> {
        static INSTANCE: OnceCell<Arc<anyhow::Result<MPSpec>>> = OnceCell::new();
        INSTANCE.get_or_init(|| Arc::new(MPSpec::read_from_file())).clone()
    }

    /// find the MP with a given email.
    pub fn find_by_email(&self, email:&str) -> Option<&MP> {
        self.mps.iter().find(|mp|mp.email.eq_ignore_ascii_case(email))
    }
}