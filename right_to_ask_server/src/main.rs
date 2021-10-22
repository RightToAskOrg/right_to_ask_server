use actix_web::{HttpServer, middleware, web};
use actix_web::{get, post};
use std::path::PathBuf;
use actix_web::web::Json;
use right_to_ask_api::person::{NewRegistration, get_list_of_all_users};
use merkle_tree_bulletin_board::hash::HashValue;
use right_to_ask_api::database::get_bulletin_board;
use merkle_tree_bulletin_board::hash_history::{FullProof, HashInfo};

#[post("/new_registration")]
async fn new_registration(command : Json<NewRegistration>) -> Json<Result<HashValue,String>> {
    Json(command.register().await.map_err(|e|e.to_string()))
}



/// For testing only!
#[get("/get_user_list")]
async fn get_user_list() -> Json<Result<Vec<String>,String>> {
    Json(get_list_of_all_users().await.map_err(|e|e.to_string()))
}


// Bulletin board api calls
#[derive(serde::Deserialize)]
struct Censor {
    leaf_to_censor : HashValue,
}

// TODO put admin authentication on this
#[post("/censor_leaf")]
async fn censor_leaf(command : Json<Censor>) -> Json<Result<(),String>> {
    Json(get_bulletin_board().await.censor_leaf(command.leaf_to_censor).map_err(|e|e.to_string()))
}


#[get("/get_parentless_unpublished_hash_values")]
async fn get_parentless_unpublished_hash_values() -> Json<Result<Vec<HashValue>,String>> {
    Json(get_bulletin_board().await.get_parentless_unpublished_hash_values().map_err(|e|e.to_string()))
}

#[get("/get_most_recent_published_root")]
async fn get_most_recent_published_root() -> Json<Result<Option<HashValue>,String>> {
    Json(get_bulletin_board().await.get_most_recent_published_root().map_err(|e|e.to_string()))
}

// TODO put admin authentication on this.
#[post("/order_new_published_root")]
async fn order_new_published_root() -> Json<Result<HashValue,String>> {
    Json(get_bulletin_board().await.order_new_published_root().map_err(|e|e.to_string()))
}

#[derive(serde::Deserialize)]
struct QueryHash {
    hash : HashValue,
}

#[get("/get_hash_info")]
async fn get_hash_info(query:web::Query<QueryHash>) -> Json<Result<HashInfo,String>> {
    Json(get_bulletin_board().await.get_hash_info(query.hash).map_err(|e|e.to_string()))
}

#[get("/get_proof_chain")]
async fn get_proof_chain(query:web::Query<QueryHash>) -> Json<Result<FullProof,String>> {
    Json(get_bulletin_board().await.get_proof_chain(query.hash).map_err(|e|e.to_string()))
}

#[get("/get_all_published_roots")]
async fn get_all_published_roots() -> Json<Result<Vec<HashValue>,String>> {
    Json(get_bulletin_board().await.get_all_published_roots().map_err(|e|e.to_string()))
}


/// find the path containing web resources, static web files that will be served.
/// This is usually in the directory `WebResources` but the program may be run from
/// other directories. To be as robust as possible it will try likely possibilities.
fn find_web_resources() -> PathBuf {
    let rel_here = std::path::Path::new(".").canonicalize().expect("Could not resolve path .");
    for p in rel_here.ancestors() {
        let pp = p.join("WebResources");
        if pp.is_dir() {return pp;}
        let pp = p.join("right_to_ask_server/WebResources");
        if pp.is_dir() {return pp;}
    }
    panic!("Could not find WebResources. Please run in a directory containing it.")
}


#[actix_rt::main]
async fn main() -> anyhow::Result<()> {
    println!("Running demo webserver on http://localhost:8099");
    HttpServer::new(move|| {
        actix_web::App::new()
            .wrap(middleware::Compress::default())
            .service(new_registration)
            .service(get_user_list)
            .service(censor_leaf)
            .service(get_parentless_unpublished_hash_values)
            .service(get_most_recent_published_root)
            .service(order_new_published_root)
            .service(get_hash_info)
            .service(get_proof_chain)
            .service(get_all_published_roots)
            .service(actix_files::Files::new("/journal/", "journal").use_last_modified(true).use_etag(true).show_files_listing())
            .service(actix_files::Files::new("/", find_web_resources()).use_last_modified(true).use_etag(true).index_file("index.html"))
    })
        .bind("0.0.0.0:8099")?
        .run()
        .await?;
    Ok(())
}