use right_to_ask_api::mp::{update_mp_list_of_files_for_jurisdictions,create_mp_list_for_jurisdictions,STATE_AND_FEDERAL_JURISDICTIONS};
use right_to_ask_api::regions::Jurisdiction;

#[tokio::main]
/// If any jurisdictions are listed at the command line, only those are updated; if none are listed,
/// they're all updated.
///
/// Usage: Run with no arguments to do everything.
/// Otherwise specify a list of jurisdictions that apply.
/// One can also add a pseudo-jurisdiction "NoWiki" that means don't download all the Wikipedia stuff (which takes most of the time).
async fn main() -> anyhow::Result<()> {
    let mut jurisdictions: Vec<String> = std::env::args().skip(1).collect();
    let should_get_wiki: bool = !jurisdictions.contains(&"NoWiki".to_string());
    jurisdictions.retain(|s| s != "NoWiki");
    // let print_jurisdictions = if jurisdictions.is_empty() { "Everything".to_string() } else { jurisdictions.join(", ") };
    let jurisdictions : Vec<Jurisdiction> = jurisdictions.into_iter().map(|j| Jurisdiction::try_from(j.to_ascii_uppercase().as_str()).expect("Bad jurisdiction - use Federal, ACT, NSW, NT, QLD, SA, TAS, VIC, WA.\n")).collect();
    let jurisdictions_as_slice : &[Jurisdiction] = if jurisdictions.is_empty() { &STATE_AND_FEDERAL_JURISDICTIONS } else { &jurisdictions };
    let print_jurisdictions = jurisdictions.iter().map(Jurisdiction::to_string).collect::<Vec<String>>().join(", ");
    println!("Downloading into MP_Source/ and checking files for {}", print_jurisdictions);
    update_mp_list_of_files_for_jurisdictions(jurisdictions_as_slice,should_get_wiki).await?;
    println!("Creating MP_source/MPs.json for {print_jurisdictions}");
    create_mp_list_for_jurisdictions(jurisdictions_as_slice).await?;
    println!("Ran successfully");
    Ok(())
}