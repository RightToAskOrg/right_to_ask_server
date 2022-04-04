use right_to_ask_api::parse_upcoming_hearings::{create_hearings_list, update_hearings_list_of_files};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Downloading into data/upcoming_hearings and checking files");
    update_hearings_list_of_files().await?;
    println!("Creating data/upcoming_hearings and checking files/hearings.json");
    create_hearings_list().await?;
    println!("Ran successfully");
    Ok(())
}