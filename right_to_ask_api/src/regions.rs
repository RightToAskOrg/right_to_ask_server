

//! Political regions - states, electorates, etc.

use serde::{Serialize,Deserialize};
use std::fmt;
use mysql::prelude::{FromValue, ConvIr};
use mysql::{Value, FromValueError};
use std::fmt::{Display, Formatter};

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

impl From<State> for Value {
	fn from(s: State) -> Self {
		Value::Bytes(s.to_string().into_bytes())
	}
}

impl ConvIr<State> for State {
	fn new(v: Value) -> Result<Self, FromValueError> {
		match v {
			Value::Bytes(bytes) => match bytes.as_slice() {
				b"ACT" => Ok(State::ACT),
				b"NSW" => Ok(State::NSW),
				b"NT" => Ok(State::NT),
				b"QLD" => Ok(State::QLD),
				b"SA" => Ok(State::SA),
				b"TAS" => Ok(State::TAS),
				b"VIC" => Ok(State::VIC),
				b"WA" => Ok(State::WA),
				_ => Err(FromValueError(Value::Bytes(bytes))),
			},
			v => Err(FromValueError(v)),
		}
	}

	fn commit(self) -> Self { self }
	fn rollback(self) -> Value { self.into() }
}

impl FromValue for State {
	type Intermediate = Self;
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
	SA_House_Of_Assembly,
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

impl From<Chamber> for Value {
	fn from(s: Chamber) -> Self {
		Value::Bytes(s.to_string().into_bytes())
	}
}

impl ConvIr<Chamber> for Chamber {
	fn new(v: Value) -> Result<Self, FromValueError> {
		match v { // May have to deal with int and uint if it is an enumeration on the server.
			Value::Bytes(bytes) => match bytes.as_slice() {
				b"ACT_Legislative_Assembly" => Ok(Chamber::ACT_Legislative_Assembly),
				b"Australian_House_Of_Representatives" => Ok(Chamber::Australian_House_Of_Representatives),
				b"Australian_Senate" => Ok(Chamber::Australian_Senate),
				b"NSW_Legislative_Assembly" => Ok(Chamber::NSW_Legislative_Assembly),
				b"NSW_Legislative_Council" => Ok(Chamber::NSW_Legislative_Council),
				b"NT_Legislative_Assembly" => Ok(Chamber::NT_Legislative_Assembly),
				b"Qld_Legislative_Assembly" => Ok(Chamber::Qld_Legislative_Assembly),
				b"SA_Legislative_Assembly" => Ok(Chamber::SA_House_Of_Assembly),
				b"SA_Legislative_Council" => Ok(Chamber::SA_Legislative_Council),
				b"Vic_Legislative_Assembly" => Ok(Chamber::Vic_Legislative_Assembly),
				b"Vic_Legislative_Council" => Ok(Chamber::Vic_Legislative_Council),
				b"Tas_House_Of_Assembly" => Ok(Chamber::Tas_House_Of_Assembly),
				b"Tas_Legislative_Council" => Ok(Chamber::Tas_Legislative_Council),
				b"WA_Legislative_Assembly" => Ok(Chamber::WA_Legislative_Assembly),
				b"WA_Legislative_Council" => Ok(Chamber::WA_Legislative_Council),
				_ => Err(FromValueError(Value::Bytes(bytes))),
			},
			v => Err(FromValueError(v)),
		}
	}

	fn commit(self) -> Self { self }
	fn rollback(self) -> Value { self.into() }
}

impl FromValue for Chamber {
	type Intermediate = Self;
}


/// A generalized electorate, being a chamber, and the particular region for that chamber, unless the chamber has no regions.
#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq)]
pub struct Electorate {
    pub(crate) chamber : Chamber,
	#[serde(skip_serializing_if = "Option::is_none",default)]
    pub(crate) region: Option<String>,
}

impl Display for Electorate {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		if let Some(region) = &self.region {
			write!(f, "{} in {}", region, self.chamber)
		} else {
			write!(f, "{}", self.chamber)
		}
	}
}