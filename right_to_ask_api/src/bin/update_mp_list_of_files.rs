use right_to_ask_api::mp::{update_mp_list_of_files, create_mp_list};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Downloading into MP_Source/ and checking files");
    //update_mp_list_of_files().await?;
    println!("Creating MP_source/MPs.json");
    create_mp_list()?;
    println!("Ran successfully");
    Ok(())
}