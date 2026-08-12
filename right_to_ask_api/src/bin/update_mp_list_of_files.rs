use right_to_ask_api::mp::{update_mp_list_of_files, update_mp_list_of_files_for_jurisdictions, create_mp_list};
use right_to_ask_api::regions::Jurisdiction;

#[tokio::main]
/// If any jurisdictions are listed at the command line, only those are updated; if none are listed,
/// they're all updated.
async fn main() -> anyhow::Result<()> {
    let jurisdictions: Vec<String> = std::env::args().skip(1).collect();
    if jurisdictions.len() == 0 {
        println!("Downloading into MP_Source/ and checking files");
        update_mp_list_of_files().await?;
        println!("Creating MP_source/MPs.json");
        create_mp_list().await?;
    } else {
        let jurisdictions: Vec<Jurisdiction>
            = jurisdictions.into_iter().map(|j| Jurisdiction::try_from(j.to_ascii_uppercase().as_str())
              .expect("Bad jurisdiction - use Federal, ACT, NSW, NT, QLD, SA, TAS, VIC, WA.\n")).collect();
        println!("Downloading into MP_Source/ and checking files for {}", jurisdictions.iter().map(|j| j.to_string()).collect::<Vec<String>>().join(", "));
        update_mp_list_of_files_for_jurisdictions(jurisdictions).await?;
        println!("Creating MP_source/MPs.json");
        // create_mp_list().await?;

    }
    println!("Ran successfully");
    Ok(())
}