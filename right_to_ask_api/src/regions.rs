

//! Political regions - states, electorates, etc.

use serde::{Serialize,Deserialize};
use std::fmt;

#[derive(Debug,Clone,Copy,Serialize,Deserialize,Eq,PartialEq)]
pub enum State {
    ACT,NSW,NT,QLD,SA,TAS,VIC,WA
}

// Provide Display & to_string() for State enum
impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
/// A chamber of an Australian parliament.
#[derive(Debug,Clone,Copy,Serialize,Deserialize,Eq,PartialEq)]
#[allow(non_camel_case_types)]
pub enum Chamber {
    ACT_Legislative_Assembly,
    Australian_House_Of_Representatives,
	Australian_Senate,
	NSW_Legislative_Assembly,
	NSW_Legislative_Council,
	NT_Legislative_Assembly,
	Qld_Legislative_Assembly,
	SA_Legislative_Assembly,
	SA_Legislative_Council,
	Vic_Legislative_Assembly,
	Vic_Legislative_Council,
	Tas_House_Of_Assembly,
	Tas_Legislative_Council,
	WA_Legislative_Assembly,
	WA_Legislative_Council
}

// Provide Display & to_string() for Chamber enum
impl fmt::Display for Chamber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// A generalized electorate, being a chamber, and the particular region for that chamber, unless the chamber has no regions.
#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub struct Electorate {
    pub(crate) chamber : Chamber,
	#[serde(skip_serializing_if = "Option::is_none",default)]
    pub(crate) location : Option<String>,
}
