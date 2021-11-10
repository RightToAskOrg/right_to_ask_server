
//! Human representatives - generalization of MPs, hereafter just referred to as MPs.


use crate::regions::{Electorate};
pub use crate::parse_mp_lists::{update_mp_list_of_files,create_mp_list};
use serde::{Serialize,Deserialize};

/// Information about a MP (or other human elected representative, e.g. senator).
/// Not all fields are known perfectly for each person.
#[derive(Serialize,Deserialize)]
pub struct MP {
    pub first_name : String,
    pub surname : String,
    pub electorate : Electorate,
    pub email : String,
    pub role : String,
}

impl MP {
}