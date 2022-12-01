//! A description of committees that may be sensible targets for "who ta ask a question"
//! Note that this is very similar to minister.rs

use mysql::prelude::Queryable;
use serde::{Serialize, Deserialize};
use crate::regions::Jurisdiction;

/// An identifier for a committee at some point in time. Analogous to [MPId]
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
    #[serde(default,skip_serializing_if = "Option::is_none")]
    pub url : Option<String>,
    #[serde(default,skip_serializing_if = "Option::is_none")]
    pub committee_type : Option<String>,
}

/// the id field in the Committee_IDs table
pub type CommitteeIndexInDatabaseTable = usize;

impl CommitteeId {
    /// get information on an mp. Very similar to the analogous query for MPId
    pub fn read_from_database(conn:&mut impl Queryable,id : CommitteeIndexInDatabaseTable) -> mysql::Result<Option<CommitteeId>> {
        Ok(if let Some((jurisdiction,name)) = conn.exec_first::<(Jurisdiction,String),_,_>("select Jurisdiction,Name from Committee_IDs where id=?",(id,))? {
            Some(CommitteeId{
                jurisdiction,
                name,
            })
        } else {
            None
        })
    }

    /// given a Committee, get their id, should it exist.
    pub fn get_id_from_database_if_there(&self,conn:&mut impl Queryable) -> mysql::Result<Option<CommitteeIndexInDatabaseTable>> {
        conn.exec_first("select id from Committee_IDs where Jurisdiction=? and Name=?",(self.jurisdiction,&self.name))
    }
    /// given an Committee, get their id, inserting a new one if it is not already there.
    pub fn get_id_from_database(&self,conn:&mut impl Queryable) -> mysql::Result<CommitteeIndexInDatabaseTable> {
        if let Some(id) = self.get_id_from_database_if_there(conn)? {
            // it is already there.
            Ok(id)
        } else {
            // it needs to be inserted.
            conn.exec_drop("insert into Committee_IDs (Jurisdiction,Name) values (?,?)",(self.jurisdiction,&self.name))?;
            let id : CommitteeIndexInDatabaseTable  = conn.exec_first("SELECT LAST_INSERT_ID()",())?.unwrap();
            Ok(id)
        }
    }

}