
//! Extra data about MPs derived from not-necessarily-authoritative sources, e.g. Wikipedia.


use crate::regions::{Chamber, Electorate, RegionContainingOtherRegions};
pub use crate::parse_mp_lists::{update_mp_list_of_files,create_mp_list};
use serde::{Serialize,Deserialize};
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use itertools::Itertools;
use mysql::prelude::Queryable;
use crate::common_file::MPS;
use crate::minister::MinisterId;
use crate::question::OrgID;

/// Information about a MP (or other human elected representative, e.g. senator).
/// Not all fields are known perfectly for each person.
/// This is Information about current MPs, rather than a definition of an MP at some point in time.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct MPNonAuthoritative {
    pub wikipedia_title : String,
    pub img_data : Option<ImageInfo>, // path, filename, attribution
    pub electorate_name : String,
}

#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct ImageInfo {
    pub path: String,
    pub name: String,
    pub artist: String,
    pub attribution_short_name: String,
    pub attribution_url: Option<String>, 
    pub description: String
}

impl MPNonAuthoritative {
    /// Get the name associated with a badge for an MP.
    /// This is `FirstName surname @emaildomain`
    pub fn image_ref(&self) -> Option<String> {
        self.first_name.to_string()+" "+&self.surname+" "+self.email.trim_start_matches(|c|c!='@')
    }
}
