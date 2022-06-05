//! Connect to database.
//! The file database_url should contain something like "mysql://bulletinboard:ThisShouldBeReplacedByAPassword@localhost:3306/bulletinboard" without the quotes, and with the password something sensible.
//! The file bulletin_board_url should contain something like "mysql://bulletinboard:ThisShouldBeReplacedByAPassword@localhost:3306/bulletinboard" without the quotes, and with the password something sensible.

use std::ops::DerefMut;
use mysql::{Pool, PooledConn, Conn, Opts};
use once_cell::sync::Lazy;
use futures::lock::{Mutex, MutexGuard};
use merkle_tree_bulletin_board::backend_journal::{BackendJournal, StartupVerification};
use merkle_tree_bulletin_board_backend_mysql::BackendMysql;
use merkle_tree_bulletin_board::BulletinBoard;
use merkle_tree_bulletin_board::hash::HashValue;
use mysql::prelude::Queryable;
use crate::config::CONFIG;
use crate::person::NewRegistration;
use crate::question::{EditQuestionCommandPostedToBulletinBoard, hash_from_value, NewQuestionCommandPostedToBulletinBoard};
use serde::{Serialize,Deserialize};
use word_comparison::comparison_list::ScoredIDs;
use word_comparison::database_backend::WordComparisonDatabaseBackend;
use word_comparison::flatfile_database_backend::FlatfileDatabaseBackend;
use word_comparison::listed_keywords::ListedKeywords;
use word_comparison::word_file::{WORD_MMAP_FILE, WordsInFile};
use crate::signing::ClientSignedUnparsed;

fn get_rta_database_pool_raw() -> Pool {
    let opts = Opts::from_url(&CONFIG.database.rta).expect("Could not parse database_url url");
    Pool::new(opts).expect("Could not connect to database")
}

/// Get a database connection from a pool.
pub async fn get_rta_database_connection() -> mysql::Result<PooledConn> {
    static DATABASE_POOL : Lazy<Mutex<Pool>> = Lazy::new(|| { Mutex::new(get_rta_database_pool_raw())  });
    let pool = DATABASE_POOL.lock().await;
    pool.get_conn()
}

fn get_bulletin_board_connection() -> Conn {
    let opts = Opts::from_url(&CONFIG.database.bulletinboard).expect("Could not parse bulletin_board_url url");
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

/// Something that may be logged in the bulletin board.
#[derive(Serialize,Deserialize,Debug,Clone)]
pub enum LogInBulletinBoard {
    NewUser(NewRegistration),
    EditUser(ClientSignedUnparsed),
    EmailVerification(ClientSignedUnparsed),
    NewQuestion(NewQuestionCommandPostedToBulletinBoard),
    EditQuestion(EditQuestionCommandPostedToBulletinBoard),
}

impl LogInBulletinBoard {
    pub async fn log_in_bulletin_board(&self) -> anyhow::Result<HashValue> {
        let mut board = get_bulletin_board().await;
        let data = serde_json::ser::to_string(self).unwrap();
        board.submit_leaf(&data)
    }
}

/// Delete all data and recreate the schema.
pub fn initialize_bulletin_board_database() -> anyhow::Result<()> {
    let mut conn = get_bulletin_board_connection();
    conn.query_drop("drop table if exists PUBLISHED_ROOTS")?;
    conn.query_drop("drop table if exists PUBLISHED_ROOT_REFERENCES")?;
    conn.query_drop("drop table if exists BRANCH")?;
    conn.query_drop("drop table if exists LEAF")?;

    let schema = merkle_tree_bulletin_board_backend_mysql::SCHEMA;
    conn.query_drop(schema)?;
    Ok(())
}

pub fn initialize_right_to_ask_database() -> anyhow::Result<()> {
    let mut conn = get_rta_database_pool_raw().get_conn().expect("Could not get rta database connection");
    let schema = include_str!("RTASchema.sql");
    conn.query_drop(schema)?;
    Ok(())
}

static GENERAL_VOCABULARY_WORDS : Lazy<WordsInFile> = Lazy::new(|| { WordsInFile::read_word_file(WORD_MMAP_FILE).unwrap()  });
static LISTED_KEYWORDS : Lazy<ListedKeywords> = Lazy::new(|| { ListedKeywords::load(ListedKeywords::STD_LOCATION).unwrap()  });

const WORD_COMPARISON_PATH: &str = "data/WordComparison/Database.txt";
static WORD_COMPARISON_BACKEND : Lazy<Mutex<FlatfileDatabaseBackend<HashValue>>> = Lazy::new(|| { Mutex::new(FlatfileDatabaseBackend::<HashValue>::new(WORD_COMPARISON_PATH,&GENERAL_VOCABULARY_WORDS,&LISTED_KEYWORDS).unwrap())  });

/// Add a new question to the comparison_database. Typically done
/// * After creating a new question and saving it into the right_to_ask database
/// * When recreating the comparison database.
pub async fn add_question_to_comparison_database(question:&str, id:HashValue) -> anyhow::Result<()> {
    let mut backend =  WORD_COMPARISON_BACKEND.lock().await;
    word_comparison::comparison_list::add_question(backend.deref_mut(),question,id,&GENERAL_VOCABULARY_WORDS,&LISTED_KEYWORDS)?;
    Ok(())
}

pub async fn find_similar_text_question(question:&str) -> anyhow::Result<Vec<ScoredIDs<HashValue>>> {
    let mut backend =  WORD_COMPARISON_BACKEND.lock().await;
    word_comparison::comparison_list::find_similar_in_database(backend.deref_mut(),question,&GENERAL_VOCABULARY_WORDS,&LISTED_KEYWORDS)
}

/// Recreate the word comparison database. This generally doesn't result in any information being
/// lost - it is done by destroying the word comparison database, recreating it, and then
/// loading all questions from the RTA database and loading them into the word comparison database.
pub async fn recreate_word_comparison_database() -> anyhow::Result<()> {
    println!("Extracting existing questions");
    let mut conn = get_rta_database_connection().await?;
    let questions : Vec<(HashValue,String)> = conn.exec_map("SELECT QuestionId,Question from QUESTIONS",(),|(id,question)|(hash_from_value(id),question))?;
    println!("Recreating database");
    {
        let mut backend =  WORD_COMPARISON_BACKEND.lock().await;
        backend.clear_all_reinitialize()?;
    }
    for (id,question) in questions {
        println!("Adding question : {}",question);
        add_question_to_comparison_database(&question,id).await?;
    }
    Ok(())
}