use actix_web::{HttpServer, middleware, web};
use actix_web::{get, post};
use std::path::PathBuf;
use actix_web::web::Json;
use right_to_ask_api::person::{NewRegistration, get_list_of_all_users, get_count_of_all_users, UserInfo, get_user_by_id, RequestEmailValidation, EmailProof, EmailAddress, EditUserDetails};
use merkle_tree_bulletin_board::hash::HashValue;
use right_to_ask_api::database::get_bulletin_board;
use merkle_tree_bulletin_board::hash_history::{FullProof, HashInfo};
use right_to_ask_api::signing::{get_server_public_key_base64encoded, ServerSigned, get_server_public_key_raw_hex, get_server_public_key_raw_base64, ClientSigned};
use actix_web::http::header::{ContentDisposition, DispositionType, DispositionParam};
use actix_files::NamedFile;
use right_to_ask_api::question::{EditQuestionCommand, NewQuestionCommand, QuestionID, QuestionInfo};

#[post("/new_registration")]
async fn new_registration(command : Json<NewRegistration>) -> Json<Result<ServerSigned,String>> {
    Json(ServerSigned::sign_string(command.register().await))
}

#[post("/edit_user")]
async fn edit_user(command : Json<ClientSigned<EditUserDetails>>) -> Json<Result<ServerSigned,String>> {
    if let Err(signing_error) = command.signed_message.check_signature().await {
        Json(Err(signing_error.to_string()))
    } else {
        let res = EditUserDetails::edit_user(&command).await;
        let signed = ServerSigned::sign_string(res);
        Json(signed)
    }
}



#[post("/new_question")]
async fn new_question(command : Json<ClientSigned<NewQuestionCommand>>) -> Json<Result<ServerSigned,String>> {
    if let Err(signing_error) = command.signed_message.check_signature().await {
        Json(Err(signing_error.to_string()))
    } else {
        let res = NewQuestionCommand::add_question(&command).await;
        let signed = ServerSigned::sign(res);
        Json(signed)
    }
}

#[post("/edit_question")]
async fn edit_question(command : Json<ClientSigned<EditQuestionCommand>>) -> Json<Result<ServerSigned,String>> {
    if let Err(signing_error) = command.signed_message.check_signature().await {
        Json(Err(signing_error.to_string()))
    } else {
        let res = EditQuestionCommand::edit(&command).await;
        let signed = ServerSigned::sign_string(res);
        Json(signed)
    }
}



#[post("/request_email_validation")]
async fn request_email_validation(command : Json<ClientSigned<RequestEmailValidation,EmailAddress>>) -> Json<Result<ServerSigned,String>> {
    if let Err(signing_error) = command.signed_message.check_signature().await {
        Json(Err(signing_error.to_string()))
    } else {
        let res = RequestEmailValidation::process(&command).await;
        let signed = ServerSigned::sign_string(res);
        Json(signed)
    }
}

#[post("/email_proof")]
async fn email_proof(command : Json<ClientSigned<EmailProof>>) -> Json<Result<Option<ServerSigned>,String>> {
    if let Err(signing_error) = command.signed_message.check_signature().await {
        Json(Err(signing_error.to_string()))
    } else {
        let res = EmailProof::process(&command).await;
        let signed = res.map_err(|e|e.to_string()).map(|oh|oh.map(|h|ServerSigned::new_string(h.to_string())));
        Json(signed)
    }
}





/// Get server public key, in base64 encoded SPKI format (PEM body).
#[get("/get_server_public_key_spki")]
async fn get_server_public_key_spki() -> Json<String> {
    Json(get_server_public_key_base64encoded())
}

/// Get server public key, in hex raw 32 bytes (64 hex characters).
#[get("/get_server_public_key_hex")]
async fn get_server_public_key_hex() -> Json<String> {
    Json(get_server_public_key_raw_hex())
}

/// Get server public key, in hex raw 32 bytes (64 hex characters).
#[get("/get_server_public_key_raw")]
async fn get_server_public_key_raw() -> Json<String> {
    Json(get_server_public_key_raw_base64())
}

/// For testing only!
#[get("/get_user_list")]
async fn get_user_list() -> Json<Result<Vec<String>,String>> {
    Json(get_list_of_all_users().await.map_err(|e|e.to_string()))
}

#[derive(serde::Deserialize)]
struct QueryUser {
    uid : String,
}
#[get("/get_user")]
async fn get_user(query:web::Query<QueryUser>) -> Json<Result<Option<UserInfo>,String>> {
    Json(get_user_by_id(&query.uid).await.map_err(|e|e.to_string()))
}

#[derive(serde::Deserialize)]
struct QueryQuestion {
    question_id : QuestionID,
}
#[get("/get_question")]
async fn get_question(query:web::Query<QueryQuestion>) -> Json<Result<Option<QuestionInfo>,String>> {
    Json(QuestionInfo::lookup(query.question_id).await.map_err(|e|e.to_string()))
}

/// For testing only!
#[get("/get_question_list")]
async fn get_question_list() -> Json<Result<Vec<QuestionID>,String>> {
    Json(QuestionInfo::get_list_of_all_questions().await.map_err(|e|e.to_string()))
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

#[get("/MPs.json")]
async fn mps() -> std::io::Result<NamedFile> {
    let file = NamedFile::open("data/MP_source/MPs.json")?;
    Ok(file
        .use_last_modified(true)
        .set_content_disposition(ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![DispositionParam::Filename("MPs.json".to_string())],
        }))
}


#[actix_rt::main]
async fn main() -> anyhow::Result<()> {
    // check whether everything is working before starting the web server. Don't want to find out in the middle of a transaction.
    println!("Server public key {}",get_server_public_key_raw_base64());
    println!("Bulletin board latest published root {:?}",get_bulletin_board().await.get_most_recent_published_root()?);
    println!("{} users in the database",get_count_of_all_users().await?);
    println!("Running demo webserver on http://localhost:8099 stop with control C.");
    HttpServer::new(move|| {
        actix_web::App::new()
            .wrap(middleware::Compress::default())
            .service(get_server_public_key_hex)
            .service(get_server_public_key_spki)
            .service(get_server_public_key_raw)
            .service(new_registration)
            .service(edit_user)
            .service(request_email_validation)
            .service(email_proof)
            .service(new_question)
            .service(edit_question)
            .service(get_user_list)
            .service(get_user)
            .service(get_question_list)
            .service(get_question)
            .service(censor_leaf)
            .service(get_parentless_unpublished_hash_values)
            .service(get_most_recent_published_root)
            .service(order_new_published_root)
            .service(get_hash_info)
            .service(get_proof_chain)
            .service(get_all_published_roots)
            .service(mps)
            .service(actix_files::Files::new("/journal/", "journal").use_last_modified(true).use_etag(true).show_files_listing())
            .service(actix_files::Files::new("/", find_web_resources()).use_last_modified(true).use_etag(true).index_file("index.html"))
    })
        .bind("0.0.0.0:8099")?
        .run()
        .await?;
    Ok(())
}