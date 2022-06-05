//! This file contains the actix web server wrapper around the functions




use std::ops::DerefMut;
use actix_web::{HttpServer, middleware, web};
use actix_web::web::Json;
use actix_web::{get, post};
use async_std::sync::Mutex;
use word_comparison::word_file::{WordsInFile, WORD_MMAP_FILE};
use word_comparison::listed_keywords::ListedKeywords;
use word_comparison::comparison_list::{add_question, find_similar_in_database, ScoredIDs};
use std::path::PathBuf;
use word_comparison::database_backend::{InternalQuestionId, WordComparisonDatabaseBackend};
use word_comparison::flatfile_database_backend::FlatfileDatabaseBackend;

/// The external question ID.
type QuestionID = u32;
type QuestionDatabase = FlatfileDatabaseBackend<QuestionID>;

#[derive(serde::Deserialize)]
struct QueryQuestion {
    id : QuestionID,
}

/// Get some particular question
#[get("/get_question")]
async fn get_question(query:web::Query<QueryQuestion>, question_db: web::Data<Mutex<QuestionDatabase>>) -> Json<Result<Option<String>,String>> {
    Json(question_db.lock().await.lookup(query.id).map_err(|s|s.to_string()))
}

/// Get some particular question
#[get("/get_all_questions")]
async fn get_all_questions(question_db: web::Data<Mutex<QuestionDatabase>>) -> Json<Result<Vec<String>,String>> {
    Json(question_db.lock().await.get_all_questions().map_err(|s|s.to_string()))
}


#[derive(serde::Deserialize)]
struct QuerySimilarity {
    question : String,
}

/// Get some particular question
#[get("/get_similar")]
async fn get_similar(query:web::Query<QuerySimilarity>, question_db: web::Data<Mutex<QuestionDatabase>>, words: web::Data<WordsInFile>,keywords: web::Data<ListedKeywords>) -> Json<Result<Vec<ScoredIDs<QuestionID>>,String>> {
    let similar = find_similar_in_database(question_db.lock().await.deref_mut(),&query.question,&words,&keywords);
    Json(similar.map_err(|e|e.to_string()))
}

/// find the path containing web resources, static web files that will be served.
/// This is usually in the directory `WebResources` but the program may be run from
/// other directories. To be as robust as possible it will try likely possibilities.
fn find_web_resources() -> PathBuf {
    let rel_here = std::path::Path::new(".").canonicalize().expect("Could not resolve path .");
    for p in rel_here.ancestors() {
        let pp = p.join("WebResources");
        if pp.is_dir() {return pp;}
        let pp = p.join("word_comparison_demo_webserver/WebResources");
        if pp.is_dir() {return pp;}
    }
    panic!("Could not find WebResources. Please run in a directory containing it.")
}


#[derive(serde::Deserialize)]
struct Publish {
    data : String,
}

#[post("/submit_question")]
async fn submit_question(command : web::Json<Publish>, question_db: web::Data<Mutex<QuestionDatabase>>, words: web::Data<WordsInFile>,keywords: web::Data<ListedKeywords>) -> Json<Result<InternalQuestionId,String>> {
    let external_id = question_db.lock().await.len()*2+7;
    let res = add_question(question_db.lock().await.deref_mut(),&command.data,external_id as u32,&words,&keywords);
    Json(res.map_err(|e|e.to_string()))
}



#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let words = WordsInFile::read_word_file(WORD_MMAP_FILE)?;
    let keywords = ListedKeywords::load(ListedKeywords::STD_LOCATION)?;
    let filename : &str = FlatfileDatabaseBackend::<QuestionID>::STD_FILE_NAME;
    let questions : FlatfileDatabaseBackend<QuestionID> = FlatfileDatabaseBackend::<QuestionID>::new(filename,&words,&keywords)?;
    let questions = web::Data::new(Mutex::new(questions));
    let words = web::Data::new(words);
    let keywords = web::Data::new(keywords);
    //reload_from_textfile(questions.lock().await.deref_mut(),&words,&keywords)?;
    println!("Running demo webserver on http://localhost:8091");
    HttpServer::new(move|| {
        actix_web::App::new()
            .app_data(questions.clone())
            .app_data(words.clone())
            .app_data(keywords.clone())
            .wrap(middleware::Compress::default())
            .service(get_question)
            .service(get_all_questions)
            .service(get_similar)
            .service(submit_question)
            .service(actix_files::Files::new("/", find_web_resources()).use_last_modified(true).use_etag(true).index_file("index.html"))
    })
        .bind("0.0.0.0:8091")?
        .run()
        .await?;
    Ok(())
}

/// Load the database from a file containing a list of questions one per line.
pub fn reload_from_textfile(questions : &mut FlatfileDatabaseBackend<QuestionID>,words:&WordsInFile,keywords:&ListedKeywords) -> anyhow::Result<()> {
    use std::io::BufRead;
    questions.clear_all_reinitialize()?;
    let mut count = 0;
    if let Ok(file) = std::fs::File::open("SampleTextDatabase.txt") {
        for line in std::io::BufReader::new(file).lines() {
            add_question(questions,&line?,count,words,keywords)?;
            count+=1;
        }
    }
    Ok(())
}