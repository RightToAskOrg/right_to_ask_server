

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
pub enum Chamber { // TODO make reasonable
    LegislativeCouncil,
    LegislativeAssembly,
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
    chamber : Chamber,
    location : Option<String>,
}
