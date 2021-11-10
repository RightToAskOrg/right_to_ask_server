use right_to_ask_api::mp::{update_mp_list_of_files, create_mp_list};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    update_mp_list_of_files().await?;
    create_mp_list()?;
    println!("Ran successfully");
    Ok(())
}