use std::path::Path;
use right_to_ask_api::database::{get_rta_database_version, initialize_bulletin_board_database, initialize_right_to_ask_database, RTA_DATABASE_VERSION_REQUIRED, upgrade_right_to_ask_database};
use clap::Parser;

/// Program to set up the RightToAsk database, or upgrade and existing version.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, action)]
    /// Try to upgrade if you can.
    upgrade: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match get_rta_database_version().await {
        Ok(version) => {
            println!("Current RTA database version {}. Required version {}",version,RTA_DATABASE_VERSION_REQUIRED);
            if args.upgrade {
                let mut current_version = version;
                while current_version<RTA_DATABASE_VERSION_REQUIRED {
                    println!("Trying to upgrade version {} to version {}",current_version,current_version+1);
                    upgrade_right_to_ask_database(current_version)?;
                    println!("Upgraded version {} to version {}",current_version,current_version+1);
                    current_version+=1;
                    if get_rta_database_version().await?!=current_version {
                        println!("Something went wrong in the upgrade! This is bad news.");
                        return Ok(());
                    }
                }
                println!("Upgraded to required version!");
                return Ok(());
            }
        }
        Err(e) => {
            println!("Could not find current version of RTA database. You are running the correct program to fix this! Error {}",e);
        }
    }
    if args.upgrade {
        println!("Sorry, I could not upgrade automatically.");
        return Ok(());
    }
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
