//! A description of committees that may be sensible targets for "who ta ask a question"

use serde::{Serialize, Deserialize};
use crate::regions::Jurisdiction;

/// An identifier for a committee at some point in time. Analagous to [MPId]
#[derive(Serialize,Deserialize,Clone,Debug,Eq,PartialEq,Hash)]
pub struct CommitteeId {
    pub jurisdiction : Jurisdiction,
    pub name : String,
}

/// General information about a current committee. Analagous to [MP].
#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct CommitteeInfo {
    pub jurisdiction : Jurisdiction,
    pub name : String,
    pub url : Option<String>,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    pub committee_type : Option<String>,
}