use right_to_ask_api::mp::{update_mp_list_of_files, update_mp_list_of_files_for_jurisdictions, create_mp_list, create_mp_list_for_jurisdictions};
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
        let print_jurisdictions = jurisdictions.join(", ");
        let js: Vec<Jurisdiction>
            = jurisdictions.into_iter().map(|j| Jurisdiction::try_from(j.to_ascii_uppercase().as_str())
              .expect("Bad jurisdiction - use Federal, ACT, NSW, NT, QLD, SA, TAS, VIC, WA.\n")).collect();
        println!("Downloading into MP_Source/ and checking files for {}", print_jurisdictions);
        update_mp_list_of_files_for_jurisdictions(&js).await?;
        println!("Creating MP_source/MPs.json for {print_jurisdictions}");
        create_mp_list_for_jurisdictions(&js).await?;
    }
    println!("Ran successfully");
    Ok(())
}