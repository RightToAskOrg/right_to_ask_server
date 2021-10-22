//! Connect to database.
//! The file database_url should contain something like "mysql://bulletinboard:ThisShouldBeReplacedByAPassword@localhost:3306/bulletinboard" without the quotes, and with the password something sensible.
//! The file bulletin_board_url should contain something like "mysql://bulletinboard:ThisShouldBeReplacedByAPassword@localhost:3306/bulletinboard" without the quotes, and with the password something sensible.

use std::fs;
use mysql::{Pool, PooledConn, Conn, Opts};
use once_cell::sync::Lazy;
use futures::lock::{Mutex, MutexGuard};
use merkle_tree_bulletin_board::backend_journal::{BackendJournal, StartupVerification};
use merkle_tree_bulletin_board_backend_mysql::BackendMysql;
use merkle_tree_bulletin_board::BulletinBoard;
use mysql::prelude::Queryable;

fn get_rta_database_pool_raw() -> Pool {
    let url = fs::read_to_string("database_url").expect("No file database_url");
    let opts = Opts::from_url(&url).expect("Could not parse database_url url");
    Pool::new(opts).expect("Could not connect to database")
}

/// Get a database connection from a pool.
pub async fn get_rta_database_connection() -> mysql::Result<PooledConn> {
    static DATABASE_POOL : Lazy<Mutex<Pool>> = Lazy::new(|| { Mutex::new(get_rta_database_pool_raw())  });
    let pool = DATABASE_POOL.lock().await;
    pool.get_conn()
}

fn get_bulletin_board_connection() -> Conn {
    let url = fs::read_to_string("bulletin_board_url").expect("No file bulletin_board_url");
    let opts = Opts::from_url(&url).expect("Could not parse bulletin_board_url url");
    Conn::new(opts).expect("Could not connect to bulletin board database")
}

/// Get the main bulletin board object. Idempotent (well, within MutexGuard)
pub async fn get_bulletin_board() -> MutexGuard<'static,BulletinBoard<BackendJournal<BackendMysql<Box<Conn>>>>> {
    static BACKEND : Lazy<Mutex<BulletinBoard<BackendJournal<BackendMysql<Box<Conn>>>>>> = Lazy::new(|| {
        let conn = get_bulletin_board_connection();
        let backend = merkle_tree_bulletin_board_backend_mysql::BackendMysql{ connection: std::sync::Mutex::new(Box::new(conn)) };
        let backend_journal = BackendJournal::new(backend,"journal",StartupVerification::SanityCheckAndRepairPending).expect("Cannot create journal");
        let bulletin_board = BulletinBoard::new(backend_journal).expect("Cannot create bulletin board");
        Mutex::new(bulletin_board)
    });
    BACKEND.lock().await
}


/// Delete all data and recreate the schema.
pub fn initialize_bulletin_board_database() -> anyhow::Result<()> {
    let mut conn = get_bulletin_board_connection();
    conn.query_drop("drop table if exists PUBLISHED_ROOTS")?;
    conn.query_drop("drop table if exists PUBLISHED_ROOT_REFERENCES")?;
    conn.query_drop("drop table if exists BRANCH")?;
    conn.query_drop("drop table if exists LEAF")?;

    let schema = include_str!("../../../bulletin-board/merkle-tree-bulletin-board-backend-mysql/src/bin/Schema.sql"); // TODO put somewhere more sensible.
    conn.query_drop(schema)?;
    Ok(())
}

pub fn initialize_right_to_ask_database() -> anyhow::Result<()> {
    let mut conn = get_rta_database_pool_raw().get_conn().expect("Could not get rta database connection");
    let schema = include_str!("RTASchema.sql");
    conn.query_drop(schema)?;
    Ok(())
}