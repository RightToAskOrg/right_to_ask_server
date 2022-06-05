use right_to_ask_api::database::recreate_word_comparison_database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Recreating the word comparison database.\nThis is best done while the right_to_ask_server is not running. This recreates the word comparison database from the Right To Ask database.\nDo you want to do this now (y to continue)");
    let mut confirm = String::new();
    let _ = std::io::stdin().read_line(&mut confirm).unwrap();
    if confirm.starts_with('y') || confirm.starts_with('Y') {
        recreate_word_comparison_database().await?;
    } else {
        println!("Nothing done.")
    }
    Ok(())
}
