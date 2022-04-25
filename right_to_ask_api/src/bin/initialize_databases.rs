use std::path::Path;
use right_to_ask_api::database::{initialize_bulletin_board_database, initialize_right_to_ask_database};

fn main() -> anyhow::Result<()> {
    println!("Initializing databases. This destroys all prior data. Do you really want to do this (y to continue)");
    let mut confirm = String::new();
    let _ = std::io::stdin().read_line(&mut confirm).unwrap();
    if confirm.starts_with('y') || confirm.starts_with('Y') {
        initialize_bulletin_board_database()?;
        println!("Bulletin board database initialized.");
        initialize_right_to_ask_database()?;
        println!("Right To Ask database initialized.");
        if Path::new("journal").exists() {
            std::fs::remove_dir_all("journal")?;
            println!("Removed old journal.");
        } else {
            println!("No old journal to remove.");
        }
    } else {
        println!("Nothing done.")
    }
    Ok(())
}
