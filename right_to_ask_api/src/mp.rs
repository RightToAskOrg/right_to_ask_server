
//! Human representatives - generalization of MPs, hereafter just referred to as MPs.


use crate::regions::{Chamber, Electorate, RegionContainingOtherRegions};
pub use crate::parse_mp_lists::{update_mp_list_of_files,create_mp_list};
use serde::{Serialize,Deserialize};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use mysql::prelude::Queryable;
use once_cell::sync::OnceCell;
use crate::parse_mp_lists::MP_SOURCE;
use crate::question::OrgID;

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
#[derive(Serialize,Deserialize,Clone,Debug,Eq,PartialEq,Hash)]
pub struct MPId {
    pub first_name : String,
    pub surname : String,
    pub electorate : Electorate,
}
/// the id field in the MP_IDs table
pub type MPIndexInDatabaseTable = usize;
// the id fields in the Organisations table
pub type OrgIndexInDatabaseTable = usize;
/// given an organisation, get their id, inserting a new one if it is not already there.
pub fn get_org_id_from_database(org_name:&OrgID,conn:&mut impl Queryable) -> mysql::Result<MPIndexInDatabaseTable> {
    if let Some(id) = conn.exec_first("select id from Organisations where OrgID=?",(org_name,))? {
        // it is already there.
        Ok(id)
    } else {
        // it needs to be inserted.
        conn.exec_drop("insert into Organisations (OrgID) values (?)",(org_name,))?;
        let id : MPIndexInDatabaseTable  = conn.exec_first("SELECT LAST_INSERT_ID()",())?.unwrap();
        Ok(id)
    }
}

impl MPId {
    /// get information on an mp.
    pub fn read_from_database(conn:&mut impl Queryable,mp_id : MPIndexInDatabaseTable) -> mysql::Result<Option<MPId>> {
        Ok(if let Some((chamber,region,first_name,surname)) = conn.exec_first::<(Chamber,Option<String>,String,String),_,_>("select Chamber,Electorate,FirstName,LastName from MP_IDs where id=?",(mp_id,))? {
            let electorate = Electorate{ chamber, region };
            Some(MPId{
                first_name,
                surname,
                electorate
            })
        } else {
            None
        })
    }
    /// given an MP, get their id, inserting a new one if it is not already there.
    pub fn get_id_from_database(&self,conn:&mut impl Queryable) -> mysql::Result<MPIndexInDatabaseTable> {
        if let Some(id) = conn.exec_first("select id from MP_IDs where Chamber=? and Electorate=? and FirstName=? and LastName=?",(self.electorate.chamber,&self.electorate.region,&self.first_name,&self.surname))? {
            // it is already there.
            Ok(id)
        } else {
            // it needs to be inserted.
            conn.exec_drop("insert into MP_IDs (Chamber,Electorate,FirstName,LastName) values (?,?,?,?)",(self.electorate.chamber,&self.electorate.region,&self.first_name,&self.surname))?;
            let id : MPIndexInDatabaseTable  = conn.exec_first("SELECT LAST_INSERT_ID()",())?.unwrap();
            Ok(id)
        }
    }

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

    pub fn contains(&self,mp_id:&MPId) -> bool {
        self.mps.iter().any(|mp|mp.first_name==mp_id.first_name && mp.surname==mp_id.surname && mp.electorate==mp_id.electorate)
    }
}