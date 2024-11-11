

//! Political regions - states, electorates, etc.

use std::convert::TryFrom;
use serde::{Serialize, Deserialize};
use std::fmt;
use mysql::prelude::{FromValue, ConvIr};
use mysql::{Value, FromValueError};
use std::fmt::{Display, Formatter};
use anyhow::anyhow;

#[derive(Debug,Clone,Copy,Serialize,Deserialize,Eq,PartialEq,Hash)]
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

impl TryFrom<&str> for State {
	type Error = anyhow::Error;
	fn try_from(value: &str) -> Result<Self, Self::Error> {
		match value.as_bytes() {
			b"ACT" => Ok(State::ACT),
			b"NSW" => Ok(State::NSW),
			b"NT" => Ok(State::NT),
			b"QLD" => Ok(State::QLD),
			b"SA" => Ok(State::SA),
			b"TAS" => Ok(State::TAS),
			b"VIC" => Ok(State::VIC),
			b"WA" => Ok(State::WA),
			_ => Err(anyhow!("Invalid state {}",value)),
		}
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
#[derive(Debug,Clone,Copy,Serialize,Deserialize,Eq,PartialEq,Hash)]
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
				b"SA_House_Of_Assembly" => Ok(Chamber::SA_House_Of_Assembly),
				b"SA_Legislative_Council" => Ok(Chamber::SA_Legislative_Council),
				b"Vic_Legislative_Assembly" => Ok(Chamber::Vic_Legislative_Assembly),
				b"Vic_Legislative_Council" => Ok(Chamber::Vic_Legislative_Council),
				b"Tas_House_Of_Assembly" => Ok(Chamber::Tas_House_Of_Assembly),
				b"Tas_Legislative_Council" => Ok(Chamber::Tas_Legislative_Council),
				b"WA_Legislative_Assembly" => Ok(Chamber::WA_Legislative_Assembly),
				b"WA_Legislative_Council" => Ok(Chamber::WA_Legislative_Council),
				_ => {
					println!("Found unexpected chamber {:?} in region.rs/ConvIr<Chamber>",String::from_utf8_lossy(&bytes));
					Err(FromValueError(Value::Bytes(bytes)))
				},
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


/// Who is responsible? Union of a state or "Federal" or a chamber.
#[derive(Debug,Clone,Copy,Serialize,Deserialize,Eq,PartialEq,Hash)]
#[allow(non_camel_case_types)]
pub enum Jurisdiction {
	ACT,NSW,NT,QLD,SA,TAS,VIC,WA,
	Federal,
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
impl fmt::Display for Jurisdiction {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:?}", self)
	}
}

impl From<Jurisdiction> for Value {
	fn from(s: Jurisdiction) -> Self {
		Value::Bytes(s.to_string().into_bytes())
	}
}

impl ConvIr<Jurisdiction> for Jurisdiction {
	fn new(v: Value) -> Result<Self, FromValueError> {
		match v { // May have to deal with int and uint if it is an enumeration on the server.
			Value::Bytes(bytes) => match bytes.as_slice() {
				b"ACT_Legislative_Assembly" => Ok(Jurisdiction::ACT_Legislative_Assembly),
				b"Australian_House_Of_Representatives" => Ok(Jurisdiction::Australian_House_Of_Representatives),
				b"Australian_Senate" => Ok(Jurisdiction::Australian_Senate),
				b"NSW_Legislative_Assembly" => Ok(Jurisdiction::NSW_Legislative_Assembly),
				b"NSW_Legislative_Council" => Ok(Jurisdiction::NSW_Legislative_Council),
				b"NT_Legislative_Assembly" => Ok(Jurisdiction::NT_Legislative_Assembly),
				b"Qld_Legislative_Assembly" => Ok(Jurisdiction::Qld_Legislative_Assembly),
				b"SA_House_Of_Assembly" => Ok(Jurisdiction::SA_House_Of_Assembly),
				b"SA_Legislative_Council" => Ok(Jurisdiction::SA_Legislative_Council),
				b"Vic_Legislative_Assembly" => Ok(Jurisdiction::Vic_Legislative_Assembly),
				b"Vic_Legislative_Council" => Ok(Jurisdiction::Vic_Legislative_Council),
				b"Tas_House_Of_Assembly" => Ok(Jurisdiction::Tas_House_Of_Assembly),
				b"Tas_Legislative_Council" => Ok(Jurisdiction::Tas_Legislative_Council),
				b"WA_Legislative_Assembly" => Ok(Jurisdiction::WA_Legislative_Assembly),
				b"WA_Legislative_Council" => Ok(Jurisdiction::WA_Legislative_Council),
				b"ACT" => Ok(Jurisdiction::ACT),
				b"NSW" => Ok(Jurisdiction::NSW),
				b"NT" => Ok(Jurisdiction::NT),
				b"QLD" => Ok(Jurisdiction::QLD),
				b"SA" => Ok(Jurisdiction::SA),
				b"TAS" => Ok(Jurisdiction::TAS),
				b"VIC" => Ok(Jurisdiction::VIC),
				b"WA" => Ok(Jurisdiction::WA),
				b"Federal" => Ok(Jurisdiction::Federal),
				_ => {
					println!("Found unexpected jurisduction {:?} in region.rs/ConvIr<Jurisdiction>",String::from_utf8_lossy(&bytes));
					Err(FromValueError(Value::Bytes(bytes)))
				},
			},
			v => Err(FromValueError(v)),
		}
	}

	fn commit(self) -> Self { self }
	fn rollback(self) -> Value { self.into() }
}

impl FromValue for Jurisdiction {
	type Intermediate = Self;
}


/// A generalized electorate, being a chamber, and the particular region for that chamber, unless the chamber has no regions.
#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq,Hash)]
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


/// Nested political divisions.
/// For instance
///  * federal electorates are in a state;
///  * Vic districts are in a region
#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct RegionContainingOtherRegions {
	pub super_region : String,
	pub regions: Vec<String>,
}

impl RegionContainingOtherRegions {
	pub fn new(super_region:&str,regions:&[&str]) -> Self {
		RegionContainingOtherRegions{
			super_region: super_region.to_string(),
			regions : regions.iter().map(|s|s.to_string()).collect()
		}
	}
}

impl Jurisdiction {
	/// return true if the jurisdiction is an appropriate one for a politician in a given chamber.
	/// * If the jurisdiction is a chamber, thyey should match.
	/// * If the jurisdiction is a place, it should be hold the chamber
	pub fn compatible_with(self,chamber:Chamber) -> bool {
		match self {
			Jurisdiction::ACT => chamber==Chamber::ACT_Legislative_Assembly,
			Jurisdiction::NSW => chamber==Chamber::NSW_Legislative_Council || chamber==Chamber::NSW_Legislative_Assembly,
			Jurisdiction::NT => chamber==Chamber::NT_Legislative_Assembly,
			Jurisdiction::QLD => chamber==Chamber::Qld_Legislative_Assembly,
			Jurisdiction::SA => chamber==Chamber::SA_House_Of_Assembly || chamber==Chamber::SA_Legislative_Council,
			Jurisdiction::TAS => chamber==Chamber::Tas_House_Of_Assembly || chamber==Chamber::Tas_Legislative_Council,
			Jurisdiction::VIC => chamber==Chamber::Vic_Legislative_Assembly || chamber==Chamber::Vic_Legislative_Council,
			Jurisdiction::WA => chamber==Chamber::WA_Legislative_Assembly || chamber==Chamber::WA_Legislative_Council,
			Jurisdiction::Federal => chamber==Chamber::Australian_House_Of_Representatives || chamber==Chamber::Australian_Senate,
			Jurisdiction::ACT_Legislative_Assembly => chamber==Chamber::ACT_Legislative_Assembly,
			Jurisdiction::Australian_House_Of_Representatives => chamber==Chamber::Australian_House_Of_Representatives,
			Jurisdiction::Australian_Senate => chamber==Chamber::Australian_Senate,
			Jurisdiction::NSW_Legislative_Assembly => chamber==Chamber::NSW_Legislative_Assembly,
			Jurisdiction::NSW_Legislative_Council => chamber==Chamber::NSW_Legislative_Council,
			Jurisdiction::NT_Legislative_Assembly => chamber==Chamber::NT_Legislative_Assembly,
			Jurisdiction::Qld_Legislative_Assembly => chamber==Chamber::Qld_Legislative_Assembly,
			Jurisdiction::SA_House_Of_Assembly => chamber==Chamber::SA_House_Of_Assembly,
			Jurisdiction::SA_Legislative_Council => chamber==Chamber::SA_Legislative_Council,
			Jurisdiction::Vic_Legislative_Assembly => chamber==Chamber::Vic_Legislative_Assembly,
			Jurisdiction::Vic_Legislative_Council => chamber==Chamber::Vic_Legislative_Council,
			Jurisdiction::Tas_House_Of_Assembly => chamber==Chamber::Tas_House_Of_Assembly,
			Jurisdiction::Tas_Legislative_Council => chamber==Chamber::Tas_Legislative_Council,
			Jurisdiction::WA_Legislative_Assembly => chamber==Chamber::WA_Legislative_Assembly,
			Jurisdiction::WA_Legislative_Council => chamber==Chamber::WA_Legislative_Council,
		}
	}

}