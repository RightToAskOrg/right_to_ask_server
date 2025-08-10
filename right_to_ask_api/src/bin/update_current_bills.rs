use right_to_ask_api::parse_current_bills::{create_bills_list, update_bills_list_of_files};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Downloading into data/current_bills and checking files");
    update_bills_list_of_files().await?;
    println!("Creating data/current_bills and checking files.");
    create_bills_list().await?;
    println!("Ran successfully");
    Ok(())
}
