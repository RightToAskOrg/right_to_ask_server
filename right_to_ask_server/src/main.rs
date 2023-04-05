use std::ops::Deref;
use actix_web::{HttpServer, middleware, web};
use actix_web::{get, post};
use std::path::PathBuf;
use actix_web::web::Json;
use right_to_ask_api::person::{NewRegistration, get_list_of_all_users, get_count_of_all_users, UserInfo, get_user_by_id, RequestEmailValidation, EmailProof, EmailAddress, EditUserDetails, MiniUserInfo, search_for_users, TimesSent, RequestEmailValidationResult, BlockUserError,BlockUserCommand};
use merkle_tree_bulletin_board::hash::HashValue;
use right_to_ask_api::database::{check_rta_database_version_current, find_similar_text_question, get_bulletin_board};
use merkle_tree_bulletin_board::hash_history::{FullProof, HashInfo};
use right_to_ask_api::censorship::{CensorQuestionCommand, QuestionHistory, ReportedQuestionReasonSummary, ReportedQuestionSummary, ReportQuestionCommand};
use right_to_ask_api::signing::{get_server_public_key_base64encoded, ServerSigned, get_server_public_key_raw_hex, get_server_public_key_raw_base64, ClientSigned};
use right_to_ask_api::common_file::{COMMITTEES, HEARINGS, MPS};
use right_to_ask_api::question::{EditQuestionCommand, NewQuestionCommand, PlainTextVoteOnQuestionCommand, QuestionID, QuestionInfo, QuestionNonDefiningFields, SimilarQuestionQuery, SimilarQuestionResult};
use word_comparison::comparison_list::ScoredIDs;

#[post("/new_registration")]
async fn new_registration(command : Json<NewRegistration>) -> Json<Result<ServerSigned,String>> {
    Json(ServerSigned::sign_string(command.register().await))
}

#[post("/edit_user")]
async fn edit_user(command : Json<ClientSigned<EditUserDetails>>) -> Json<Result<ServerSigned,String>> {
    if let Err(signing_error) = command.signed_message.check_signature(false).await {
        Json(Err(signing_error.to_string()))
    } else {
        let res = EditUserDetails::edit_user(&command).await;
        let signed = ServerSigned::sign_string(res);
        Json(signed)
    }
}

const SCORE_FOR_SINGLE_METADATA_MATCH : f64 = 20.0;

async fn similar_questions_work(command:&NewQuestionCommand) -> Result<Vec<ScoredIDs<QuestionID>>,String> {
    let just_text = find_similar_text_question(&command.question_text).await.map_err(|e|e.to_string())?;
    let just_metadata  = QuestionNonDefiningFields::find_similar_metadata(&command.non_defining_fields).await.map_err(|e|e.to_string())?;
    Ok(if just_metadata.is_empty() { just_text } else { // if no metadata matches, just use text matches
        let mut unordered: Vec<ScoredIDs<QuestionID>> = if just_text.is_empty() { // if no text matches, just use metadata matches.
            just_metadata.into_iter().map(|(q,n)|ScoredIDs{ id: q, score: SCORE_FOR_SINGLE_METADATA_MATCH*(n as f64) }).collect()
        } else { // if both text and metadata matches, use just the ones with matching text, but add metadata scores.
            just_text.into_iter().map(|s|ScoredIDs{ id:s.id, score:s.score+SCORE_FOR_SINGLE_METADATA_MATCH*(just_metadata.get(&s.id).cloned().unwrap_or(0) as f64)}).collect()
        };
        unordered.sort_by(|a,b|b.score.partial_cmp(&a.score).unwrap());
        unordered
    })
}
/// This function is deprecated and replaced by get_similar_questions.
#[post("/similar_questions")]
async fn similar_questions(command : Json<NewQuestionCommand>) -> Json<Result<Vec<ScoredIDs<QuestionID>>,String>> {
    Json(similar_questions_work(&command).await)
}

/// This is idempotent; it is a post because the sent data structure is quite complex.
#[post("/get_similar_questions")]
async fn get_similar_questions(command : Json<SimilarQuestionQuery>) -> Json<Result<SimilarQuestionResult,String>> {
    Json(SimilarQuestionQuery::similar_questions(&command).await.map_err(|e|e.to_string()))
}


#[post("/new_question")]
async fn new_question(command : Json<ClientSigned<NewQuestionCommand>>) -> Json<Result<ServerSigned,String>> {
    if let Err(signing_error) = command.signed_message.check_signature(true).await {
        Json(Err(signing_error.to_string()))
    } else {
        let res = NewQuestionCommand::add_question(&command).await;
        let signed = ServerSigned::sign(res);
        Json(signed)
    }
}

#[post("/edit_question")]
async fn edit_question(command : Json<ClientSigned<EditQuestionCommand>>) -> Json<Result<ServerSigned,String>> {
    if let Err(signing_error) = command.signed_message.check_signature(true).await {
        Json(Err(signing_error.to_string()))
    } else {
        let res = EditQuestionCommand::edit(&command).await;
        let signed = ServerSigned::sign_string(res);
        Json(signed)
    }
}

#[post("/plaintext_vote_question")]
async fn plaintext_vote_question(command : Json<ClientSigned<PlainTextVoteOnQuestionCommand>>) -> Json<Result<(),String>> {
    if let Err(signing_error) = command.signed_message.check_signature(true).await {
        Json(Err(signing_error.to_string()))
    } else {
        let res = PlainTextVoteOnQuestionCommand::vote(&command).await;
        //let signed = ServerSigned::sign_string(res);
        Json(res.map_err(|e|e.to_string()))
    }
}



#[post("/request_email_validation")]
async fn request_email_validation(command : Json<ClientSigned<RequestEmailValidation,EmailAddress>>) -> Json<Result<RequestEmailValidationResult<ServerSigned>,String>> {
    if let Err(signing_error) = command.signed_message.check_signature(false).await {
        Json(Err(signing_error.to_string()))
    } else {
        let res = RequestEmailValidation::process(&command).await;
        Json(match res {
            Err(e) => Err(e.to_string()),
            Ok(RequestEmailValidationResult::EmailSent(h)) => Ok(RequestEmailValidationResult::EmailSent(h)),
            Ok(RequestEmailValidationResult::AlreadyValidated(None)) => Ok(RequestEmailValidationResult::AlreadyValidated(None)),
            Ok(RequestEmailValidationResult::AlreadyValidated(Some(h))) => {
                match ServerSigned::new(&h) {
                    Err(signing_error) => Err(signing_error.to_string()),
                    Ok(signed) => Ok(RequestEmailValidationResult::AlreadyValidated(Some(signed)))
                }
            }
        })
    }
}

#[post("/email_proof")]
async fn email_proof(command : Json<ClientSigned<EmailProof>>) -> Json<Result<Option<ServerSigned>,String>> {
    if let Err(signing_error) = command.signed_message.check_signature(false).await {
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
#[derive(serde::Deserialize)]
struct SearchUser {
    search : String,
    #[serde(default)]
    badges : bool,
}
#[get("/get_user")]
async fn get_user(query:web::Query<QueryUser>) -> Json<Result<Option<UserInfo>,String>> {
    Json(get_user_by_id(&query.uid).await.map_err(|e|e.to_string()))
}


#[get("/search_user")]
async fn search_user(query:web::Query<SearchUser>) -> Json<Result<Vec<MiniUserInfo>,String>> {
    Json(search_for_users(&query.search,query.badges).await.map_err(|e|e.to_string()))
}

#[derive(serde::Deserialize)]
struct QueryQuestion {
    question_id : QuestionID,
}
#[get("/get_question")]
async fn get_question(query:web::Query<QueryQuestion>) -> Json<Result<Option<QuestionInfo>,String>> {
    Json(QuestionInfo::lookup(query.question_id).await.map_err(|e|e.to_string()))
}

#[get("/get_question_history")]
async fn get_question_history(query:web::Query<QueryQuestion>) -> Json<Result<QuestionHistory,String>> {
    Json(QuestionHistory::lookup(query.question_id).await.map_err(|e|e.to_string()))
}



/// For testing only!
#[get("/get_question_list")]
async fn get_question_list() -> Json<Result<Vec<QuestionID>,String>> {
    Json(QuestionInfo::get_list_of_all_questions().await.map_err(|e|e.to_string()))
}

#[get("/get_questions_created_by_user")]
async fn get_questions_created_by_user(query:web::Query<QueryUser>) -> Json<Result<Vec<QuestionID>,String>> {
    Json(QuestionInfo::get_questions_created_by_user(&query.uid).await.map_err(|e|e.to_string()))
}


#[post("/moderation/censor_question")]
async fn censor_question(command : Json<CensorQuestionCommand>) -> Json<Result<HashValue,String>> {
    Json(command.censor_question().await.map_err(|e|e.to_string()))
}

#[get("/moderation/get_reported_questions")]
async fn get_reported_questions() -> Json<Result<Vec<ReportedQuestionSummary>,String>> {
    Json(ReportedQuestionSummary::get_reported_questions().await.map_err(|e|e.to_string()))
}

#[get("/moderation/get_reasons_reported")]
async fn get_reasons_reported(query:web::Query<QueryQuestion>) -> Json<Result<ReportedQuestionReasonSummary,String>> {
    Json(ReportedQuestionReasonSummary::get_reasons_reported(query.question_id).await.map_err(|e|e.to_string()))
}



#[post("/report_question")]
async fn report_question(command : Json<ClientSigned<ReportQuestionCommand>>) -> Json<Result<(),String>> { // ServerSigned not () in result if want to put on bulletin board
    if let Err(signing_error) = command.signed_message.check_signature(true).await {
        Json(Err(signing_error.to_string()))
    } else {
        let res = ReportQuestionCommand::report_question(&command).await;
        let signed = res.map_err(|e|e.to_string()); //.map(|h|ServerSigned::new_string(h.to_string()));
        Json(signed)
    }
}


// Bulletin board api calls
#[derive(serde::Deserialize)]
struct Censor {
    leaf_to_censor : HashValue,
}

#[post("/moderation/censor_leaf")]
async fn censor_leaf(command : Json<Censor>) -> Json<Result<(),String>> {
    Json(get_bulletin_board().await.censor_leaf(command.leaf_to_censor).map_err(|e|e.to_string()))
}


#[post("/moderation/block_user")]
async fn block_user(command : Json<BlockUserCommand>)-> Json<Result<(),BlockUserError>> {
    Json(command.apply().await)
}

#[get("/get_parentless_unpublished_hash_values")]
async fn get_parentless_unpublished_hash_values() -> Json<Result<Vec<HashValue>,String>> {
    Json(get_bulletin_board().await.get_parentless_unpublished_hash_values().map_err(|e|e.to_string()))
}

#[get("/get_most_recent_published_root")]
async fn get_most_recent_published_root() -> Json<Result<Option<HashValue>,String>> {
    Json(get_bulletin_board().await.get_most_recent_published_root().map_err(|e|e.to_string()))
}

#[post("/admin/order_new_published_root")]
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
async fn mps() -> Result<Vec<u8>,Box<dyn std::error::Error + 'static>> {
    let data =MPS.get_data()?;
    Ok(data.deref().clone()) // UGH!!! Why do I have to clone this?????
}

#[get("/committees.json")]
async fn committees() -> Result<Vec<u8>,Box<dyn std::error::Error + 'static>> {
    let data =COMMITTEES.get_data()?;
    Ok(data.deref().clone()) // UGH!!! Why do I have to clone this?????
}

#[get("/hearings.json")]
async fn hearings() -> Result<Vec<u8>,Box<dyn std::error::Error + 'static>> {
    let data =HEARINGS.get_data()?;
    Ok(data.deref().clone()) // UGH!!! Why do I have to clone this?????
}

/// Information that the client should get at the very start to see if the client is too old, and
/// whether lists should be downloaded.
#[derive(serde::Serialize)]
struct Info {
    /// This should be increased each time there is a change API that will break prior clients.
    api_level : usize,
    /// SHA2 hash of the MPs.json file
    hash_mps : HashValue,
    /// SHA2 hash of the committees.json file
    hash_committees : HashValue,
    /// SHA2 hash of the hearings.json file
    hash_hearings : HashValue,
}
#[get("/info.json")]
async fn info() -> Result<Json<Info>,Box<dyn std::error::Error + 'static>> {
    Ok(Json(Info{
        api_level: 0, // This should be increased each time there is a change API that will break prior clients.
        hash_mps: MPS.get_hash()?,
        hash_committees: COMMITTEES.get_hash()?,
        hash_hearings: HEARINGS.get_hash()?,
    }))
}

#[post("/admin/reload_info")]
/// Force the server to reload the MPs.json file, the committees.json file, and the hearings.json file (without restarting).
async fn reload_info() -> &'static str {
    MPS.reset();
    COMMITTEES.reset();
    HEARINGS.reset();
    "OK"
}

#[post("/admin/put_on_do_not_email_list")]
async fn put_on_do_not_email_list(command : Json<EmailAddress>) -> Json<Result<(),String>> {
    Json(command.change_do_not_email_list(true).await.map_err(|e|e.to_string()))
}
#[post("/admin/take_off_do_not_email_list")]
async fn take_off_do_not_email_list(command : Json<EmailAddress>) -> Json<Result<(),String>> {
    Json(command.change_do_not_email_list(false).await.map_err(|e|e.to_string()))
}

#[get("/admin/get_do_not_email_list")]
async fn get_do_not_email_list() -> Json<Result<Vec<EmailAddress>,String>> {
    Json(EmailAddress::get_do_not_email_list().await.map_err(|e|e.to_string()))
}
#[post("/admin/reset_times_sent")]
async fn reset_times_sent(command : Json<u32>) -> Json<Result<(),String>> {
    Json(EmailAddress::reset_times_sent(command.0).await.map_err(|e|e.to_string()))
}
#[derive(serde::Deserialize)]
struct QueryTimescale {
    timescale : u32,
}
#[get("/admin/get_times_sent")]
async fn get_times_sent(query:web::Query<QueryTimescale>) -> Json<Result<Vec<TimesSent>,String>> {
    Json(EmailAddress::get_times_sent(query.timescale).await.map_err(|e|e.to_string()))
}
#[post("/admin/take_off_times_sent_list")]
async fn take_off_times_sent_list(command : Json<EmailAddress>) -> Json<Result<(),String>> {
    Json(command.take_off_times_sent_list().await.map_err(|e|e.to_string()))
}


#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // check whether everything is working before starting the web server. Don't want to find out in the middle of a transaction.
    check_rta_database_version_current().await?;
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
            .service(similar_questions)
            .service(get_similar_questions)
            .service(new_question)
            .service(edit_question)
            .service(plaintext_vote_question)
            .service(get_user_list)
            .service(get_user)
            .service(search_user)
            .service(get_question_list)
            .service(get_questions_created_by_user)
            .service(get_question)
            .service(get_question_history)
            .service(censor_question)
            .service(get_reported_questions)
            .service(get_reasons_reported)
            .service(report_question)
            .service(censor_leaf)
            .service(block_user)
            .service(get_parentless_unpublished_hash_values)
            .service(get_most_recent_published_root)
            .service(order_new_published_root)
            .service(get_hash_info)
            .service(get_proof_chain)
            .service(get_all_published_roots)
            .service(mps)
            .service(committees)
            .service(hearings)
            .service(info)
            .service(reload_info)
            .service(take_off_do_not_email_list)
            .service(put_on_do_not_email_list)
            .service(get_do_not_email_list)
            .service(reset_times_sent)
            .service(get_times_sent)
            .service(take_off_times_sent_list)
            .service(actix_files::Files::new("/journal/", "journal").use_last_modified(true).use_etag(true).show_files_listing())
            .service(actix_files::Files::new("/", find_web_resources()).use_last_modified(true).use_etag(true).index_file("index.html"))
    })
        .bind("0.0.0.0:8099")?
        .run()
        .await?;
    Ok(())
}