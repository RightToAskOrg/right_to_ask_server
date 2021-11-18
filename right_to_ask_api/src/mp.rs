
//! Human representatives - generalization of MPs, hereafter just referred to as MPs.


use crate::regions::{Electorate};
pub use crate::parse_mp_lists::{update_mp_list_of_files,create_mp_list};
use serde::{Serialize,Deserialize};
use std::fmt::{Display, Formatter};

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
}

impl Display for MP {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {} party {} electorate {} {} {}", self.surname, self.first_name,self.party,self.electorate,self.email,self.role)
    }
}