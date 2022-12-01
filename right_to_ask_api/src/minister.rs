//! A description of ministers that may be sensible targets for "who ta ask a question"
//! Note that this is very similar to committee.rs

use mysql::prelude::Queryable;
use serde::{Serialize, Deserialize};
use crate::regions::Jurisdiction;

/// An identifier for a minister (or similar role) at some point in time. Analogous to [MPId]
#[derive(Serialize,Deserialize,Clone,Debug,Eq,PartialEq,Hash)]
pub struct MinisterId {
    pub jurisdiction : Jurisdiction,
    pub name : String,
}


/// the id field in the Committee_IDs table
pub type MinisterIndexInDatabaseTable = usize;

impl MinisterId {
    /// get information on an mp. Very similar to the analogous query for MPId
    pub fn read_from_database(conn:&mut impl Queryable,id : MinisterIndexInDatabaseTable) -> mysql::Result<Option<MinisterId>> {
        Ok(if let Some((jurisdiction,name)) = conn.exec_first::<(Jurisdiction,String),_,_>("select Jurisdiction,Name from Minister_IDs where id=?",(id,))? {
            Some(MinisterId{
                jurisdiction,
                name,
            })
        } else {
            None
        })
    }

    /// given a Committee, get their id, should it exist.
    pub fn get_id_from_database_if_there(&self,conn:&mut impl Queryable) -> mysql::Result<Option<MinisterIndexInDatabaseTable>> {
        conn.exec_first("select id from Minister_IDs where Jurisdiction=? and Name=?",(self.jurisdiction,&self.name))
    }
    /// given an Committee, get their id, inserting a new one if it is not already there.
    pub fn get_id_from_database(&self,conn:&mut impl Queryable) -> mysql::Result<MinisterIndexInDatabaseTable> {
        if let Some(id) = self.get_id_from_database_if_there(conn)? {
            // it is already there.
            Ok(id)
        } else {
            // it needs to be inserted.
            conn.exec_drop("insert into Minister_IDs (Jurisdiction,Name) values (?,?)",(self.jurisdiction,&self.name))?;
            let id : MinisterIndexInDatabaseTable  = conn.exec_first("SELECT LAST_INSERT_ID()",())?.unwrap();
            Ok(id)
        }
    }

}