//! Parse various files from parliament websites giving lists of MPs.



use tempfile::NamedTempFile;
use std::path::{PathBuf};
use std::io::Write;
use std::fs::File;
use crate::mp::MP;
use crate::regions::{Electorate, Chamber};
use std::str::FromStr;
use anyhow::anyhow;

/// Temporary file directory. Should be in same filesystem as MP_SOURCE.
const TEMP_DIR : &'static str = "data/temp";
const MP_SOURCE : &'static str = "data/MP_source";
/// Download from a URL to a temporary file.
async fn download_to_file(url:&str) -> anyhow::Result<NamedTempFile> {
    let mut file = NamedTempFile::new_in(TEMP_DIR)?;
    let response = reqwest::get(url).await?;
    let content = response.bytes().await?;
    file.write_all(&content)?;
    file.flush()?;
    Ok(file)
}

fn parse_australian_senate(file : File) -> anyhow::Result<Vec<MP>> {
    parse_csv(file, Chamber::Australian_Senate, "Surname", &["Preferred Name", "First Name"], None, Some("State"), &["Parliamentary Titles"])
}
fn parse_australian_house_reps(file : File) -> anyhow::Result<Vec<MP>> {
    parse_csv(file, Chamber::Australian_House_Of_Representatives, "Surname", &["Preferred Name", "First Name"], None, Some("Electorate"), &["Parliamentary Title", "Ministerial Title"])
}
fn parse_nsw_la(file : File) -> anyhow::Result<Vec<MP>> {
    parse_csv(file, Chamber::NSW_Legislative_Assembly, "SURNAME", &["INITIALS"], Some("CONTACT ADDRESS EMAIL"), Some("ELECTORATE"), &["MINISTRY", "OFFICE HOLDER"])
}
fn parse_nsw_lc(file : File) -> anyhow::Result<Vec<MP>> {
    parse_csv(file, Chamber::NSW_Legislative_Council, "SURNAME", &["INITIALS"], Some("CONTACT ADDRESS EMAIL"), None, &["MINISTRY", "OFFICE HOLDER"])
}

/// Parse a CSV file of contacts, given the headings
fn parse_csv(file : File,chamber:Chamber,surname_heading:&str,first_name_heading:&[&str],email_heading:Option<&str>,electorate_heading:Option<&str>,role_heading:&[&str]) -> anyhow::Result<Vec<MP>> {
    let mut reader = csv::Reader::from_reader(file);
    let mut mps = Vec::new();
    let headings = reader.headers()?;
    let find_heading = |name:&str|{headings.iter().position(|e|e==name)}.ok_or_else(||anyhow!("No column header {} for surname for {}",surname_heading,chamber));
    let col_surname = find_heading(surname_heading)?;
    let cols_firstname : Vec<usize> = first_name_heading.into_iter().map(|&s|find_heading(s)).collect::<anyhow::Result<Vec<usize>>>()?;
    let cols_role : Vec<usize> = role_heading.into_iter().map(|&s|find_heading(s)).collect::<anyhow::Result<Vec<usize>>>()?;
    let col_electorate : Option<usize> = electorate_heading.map(find_heading).transpose()?;
    let col_email : Option<usize> = email_heading.map(find_heading).transpose()?;
    for record in reader.records() {
        let record = record?;
        let mp = MP {
            first_name: cols_firstname.iter().map(|&c|&record[c]).find(|s|!s.is_empty()).unwrap_or("").to_string(),
            surname: record[col_surname].to_string(),
            electorate: Electorate { chamber, region: col_electorate.map(|c|record[c].to_string()) },
            email: col_email.map(|c|&record[c]).unwrap_or("").to_string(),
            role: cols_role.iter().map(|&c|&record[c]).fold(String::new(),|s,r|if r.is_empty() {s} else {(if s.is_empty() {s} else {s+", "})+r}),
        };
        mps.push(mp);
    }
    Ok(mps)
}

/// Download, check, and if valid replace the downloaded files with MP lists.
pub async fn update_mp_list_of_files() -> anyhow::Result<()> {
    std::fs::create_dir_all(TEMP_DIR)?;
    std::fs::create_dir_all(MP_SOURCE)?;
    let dir = PathBuf::from_str(MP_SOURCE)?;
    // Federal
    let house_reps = download_to_file("https://www.aph.gov.au/-/media/03_Senators_and_Members/Address_Labels_and_CSV_files/FamilynameRepsCSV.csv").await?;
    parse_australian_house_reps(house_reps.reopen()?)?;
    house_reps.persist(dir.join(Chamber::Australian_House_Of_Representatives.to_string()+".csv"))?;
    let senate = download_to_file("https://www.aph.gov.au/-/media/03_Senators_and_Members/Address_Labels_and_CSV_files/Senators/allsenel.csv").await?;
    parse_australian_senate(senate.reopen()?)?;
    senate.persist(dir.join(Chamber::Australian_Senate.to_string()+".csv"))?;

    // NSW
    let la = download_to_file("https://www.parliament.nsw.gov.au/_layouts/15/NSWParliament/memberlistservice.aspx?members=LA&format=Excel").await?;
    parse_nsw_la(la.reopen()?)?;
    la.persist(dir.join(Chamber::NSW_Legislative_Assembly.to_string()+".csv"))?;
    let lc = download_to_file("https://www.parliament.nsw.gov.au/_layouts/15/NSWParliament/memberlistservice.aspx?members=LA&format=Excel").await?;
    parse_nsw_lc(lc.reopen()?)?;
    lc.persist(dir.join(Chamber::NSW_Legislative_Council.to_string()+".csv"))?;
    Ok(())
}

/// Create "data/MP_source/MPs.json" from the source files downloaded by update_mp_list_of_files()
pub fn create_mp_list() -> anyhow::Result<()> {
    let dir = PathBuf::from_str(MP_SOURCE)?;
    let mut mps = Vec::new();
    mps.extend(parse_australian_house_reps(File::open(dir.join(Chamber::Australian_House_Of_Representatives.to_string()+".csv"))?)?);
    mps.extend(parse_australian_senate(File::open(dir.join(Chamber::Australian_Senate.to_string()+".csv"))?)?);
    mps.extend(parse_nsw_la(File::open(dir.join(Chamber::NSW_Legislative_Assembly.to_string()+".csv"))?)?);
    mps.extend(parse_nsw_lc(File::open(dir.join(Chamber::NSW_Legislative_Council.to_string()+".csv"))?)?);
    serde_json::to_writer(File::create(dir.join("MPs.json"))?,&mps)?;
    Ok(())
}