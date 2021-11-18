//! Parse various files from parliament websites giving lists of MPs.

// From datasources listed on https://github.com/RightToAskOrg/technical-docs/blob/main/ParliamentaryDataSources.md


use tempfile::NamedTempFile;
use std::path::{PathBuf, Path};
use std::io::Write;
use std::fs::File;
use crate::mp::MP;
use crate::regions::{Electorate, Chamber};
use std::str::FromStr;
use anyhow::anyhow;
use std::collections::{HashMap, HashSet};
use scraper::Selector;
use itertools::Itertools;

/// Temporary file directory. Should be in same filesystem as MP_SOURCE.
const TEMP_DIR : &'static str = "data/temp";
const MP_SOURCE : &'static str = "data/MP_source";
/// Download from a URL to a temporary file.
async fn download_to_file(url:&str) -> anyhow::Result<NamedTempFile> {
    println!("Downloading {}",url);
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

/// A PDF TJ operation takes a string, or rather an array of strings and other stuff. Extract just the string. Also works for Tj
fn extract_string(op:&pdf::content::Operation) -> String {
    let mut res = String::new();
    for o in &op.operands {
        if let Ok(a) = o.as_array() {
            for p in a {
                if let Ok(s) = p.as_string() {
                    if let Ok(s) = s.as_str() {
                        res.push_str(&s);
                    }
                }
            }
        } else if let Ok(s) = o.as_string() {
            if let Ok(s) = s.as_str() {
                res.push_str(&s);
            }
        }
    }
    res
}

/// Parse the PDF file of house of reps containing emails. Warning - brittle!
/// Return a map from electorate to email.
fn parse_australian_house_reps_pdf(path:&Path, electorates:&HashSet<String>) -> anyhow::Result<HashMap<String,String>> {
    // println!("Electorates : {:?}",electorates);
    let pdf = pdf::file::File::open(path)?;
    let mut history : Vec<String> = Vec::new();
    let mut electorate_to_email : HashMap<String,String> = HashMap::new();
    for page in pdf.pages() {
        let page = page?;
        if let Some(content) = &page.contents {
            for op in &content.operations {
                //println!("{}",op.to_string());
                if op.operator=="TJ" {
                    let text= extract_string(op);
                    if text.starts_with("Email: ") {
                        let email = text[7..].to_string();
                        if history.len()<3 { return Err(anyhow!("Email {} without prior recognisable electorate.",email)) }
                        let mut electorate = history[history.len()-3].to_owned();
                        if history.len()>=4 && history[history.len()-4].ends_with(' ') && !history[history.len()-4].starts_with(',') && !electorates.contains(electorate.trim_end_matches(',')) {
                            electorate=history[history.len()-4].to_owned()+&electorate;
                        }
                        if !electorate.ends_with(",") { return Err(anyhow!("Electorate {} not ending in comma.",electorate)) }
                        let electorate = electorate.trim_end_matches(',').to_string();
                        // println!("Electorate {} email {}",electorate,email);
                        if electorate_to_email.contains_key(&electorate) { return Err(anyhow!("Duplicate Electorate {} found.",electorate)) }
                        electorate_to_email.insert(electorate,email);
                        history.clear();
                    } else { history.push(text); }
                }
                // println!("{} : {}",op.operator,op.to_string())
            }
        }
    }
    Ok(electorate_to_email)
}

struct ParsedAustralianSenatePDF {
    /// A map from surname to a vector of (first name,email)
    map : HashMap<String,Vec<(String,String)>>
}
impl ParsedAustralianSenatePDF {
    fn add_email(&self,mp : &mut MP) -> anyhow::Result<()> {
        if let Some(v) = self.map.get(&mp.surname) {
            for (first,email) in v {
                if first.contains(&mp.first_name) {
                    mp.email=email.to_string();
                    return Ok(())
                }
            }
            Err(anyhow!("Could not match Australian Senate first name {} for surname {} with email data",&mp.first_name,&mp.surname))
        } else { Err(anyhow!("No email for anyone with surname {}",mp.surname))}
    }
}
struct ParseAustralianSenatePDFWork {
    history : Option<String>,
    current_name : Option<(String,String)>,
    last_was_just_email : bool,
    result : ParsedAustralianSenatePDF,
}

impl ParseAustralianSenatePDFWork {
    fn add_text(&mut self,text:String) -> anyhow::Result<()> {
        let mut text = text.trim().to_string();
        // println!("   {}",text);
        if let Some(pos) = text.find("Email: ") {
            if pos>0 { text=text[pos..].to_string(); }
        }
        if text.starts_with("Senator") && self.history.as_ref().map(|f|f.ends_with(",")).unwrap_or(false) {
            text = ", ".to_string()+&text;
            self.history=Some(self.history.take().unwrap().trim_end_matches(",").to_string())
        }
        if text.starts_with(", Senator ") {
            if let Some(surname) = self.history.take() {
                let first = text.trim_start_matches(", Senator ").trim_start_matches("the Hon ").trim().to_string();
                if self.current_name.is_some() { return Err(anyhow!("Haven't dealt with current name"))}
                self.current_name=Some((first,surname));
            }
        } else if self.last_was_just_email || text.starts_with("Email:") {
            let email = if self.last_was_just_email { text} else { text[6..].trim().to_string() };
            if email.is_empty() { self.last_was_just_email = true }
            else {
                self.last_was_just_email=false;
                if let Some((first,surname)) = self.current_name.take() {
                    // println!("Australian Senate First {} Surname {} email {}",first,surname,email);
                    self.result.map.entry(surname).or_insert_with(||vec![]).push((first,email))
                } else {
                    return Err(anyhow!("Email {} without prior recognisable name.",email));
                }
            }
        } else { self.history=Some(text); }
        Ok(())
    }
}
/// Parse the PDF file of senators containing emails. Warning - exceedingly brittle! This file feels hand edited.
/// Return a map from electorate to email.
fn parse_australian_senate_pdf(path:&Path) -> anyhow::Result<ParsedAustralianSenatePDF> {
    let pdf = pdf::file::File::open(path)?;
    let mut tm_y : Option<f32> = None;
    let mut tm_x : Option<f32> = None;
    let mut last_text_and_tm_y : Option<(String,f32)> = None;
    let mut last_font : Option<String> = None;
    let mut current_font : Option<String> = None;
    let mut work = ParseAustralianSenatePDFWork{history:None, current_name:None, last_was_just_email:false, result:ParsedAustralianSenatePDF{ map: Default::default() } };
    let mut had_bt_since_last_text = false; // really BT or Tf
    for page in pdf.pages() {
        let page = page?;
        if let Some(content) = &page.contents {
            for op in &content.operations {
                // println!("{}",op.to_string());
                match op.operator.to_uppercase().as_str() {
                    "BT" => {  tm_y = None; tm_x=None; had_bt_since_last_text=true; last_font=current_font.take(); current_font=None; }
                    "TF" if op.operands.len()==2 => {  had_bt_since_last_text=true; current_font=Some(op.operands[0].as_name()?.to_string()); }
                    "TM" if op.operands.len()==6 => {
                        if let Ok(y) = op.operands[5].as_number() {
                            tm_y=Some(y)
                        }
                        if let Ok(x) = op.operands[4].as_number() {
                            tm_x=Some(x)
                        }
                    }
                    "TJ" => { // a brittle, messy, horrible hack to concatenate strings at the same y position, if font is not set.
                        let text= extract_string(op);
                        if last_font!=current_font { last_text_and_tm_y=last_text_and_tm_y.take().map(|(t,_)|(t,f32::NAN)) } // hack to make fonted stuff be on a different line.
                        if !had_bt_since_last_text { tm_y = last_text_and_tm_y.as_ref().map(|(_,y)|*y) }
                        if let Some((last_text,last_tm_y)) = last_text_and_tm_y.take() {
                            if let Some(tm_y) = tm_y {
                                if tm_y==last_tm_y || (last_text.starts_with(", Senator")&&(last_font==current_font)&&tm_x.is_some()&&tm_x.unwrap()<230.0) {
                                    last_text_and_tm_y=Some((last_text+&text,tm_y));
                                } else {
                                    work.add_text(last_text)?;
                                    last_text_and_tm_y=Some((text,tm_y));
                                }
                            } else {
                                work.add_text(last_text)?;
                                work.add_text(text)?;
                            }
                        } else {
                            if let Some(tm_y) = tm_y {
                                last_text_and_tm_y=Some((text,tm_y));
                            } else {
                                work.add_text(text)?;
                            }
                        }
                        had_bt_since_last_text=false;
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(work.result)
}

fn parse_act_la(path:&Path) -> anyhow::Result<Vec<MP>> {
    let mut mps = Vec::new();
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let table = html.select(&Selector::parse("table > tbody").unwrap()).next().ok_or_else(||anyhow!("Could not find table in ACT html file"))?;
    let select_td = Selector::parse("td").unwrap();
    for tr in table.select(&Selector::parse("tr").unwrap()) {
        let tds : Vec<_> = tr.select(&select_td).collect();
        if tds.len()!=6 { return Err(anyhow!("Unexpected number of columns in ACT table"))}
        let name = tds[0].text().next().ok_or_else(||anyhow!("Could not find name in ACT html file"))?.trim();
        let role = tds[1].text().map(|t|t.trim()).join(", ");
        let electorate = tds[2].text().next().ok_or_else(||anyhow!("Could not find electorate in ACT html file"))?.trim();
        let email = tds[4].text().find(|t|t.trim().ends_with("act.gov.au")).ok_or_else(||anyhow!("Could not find email in ACT html file"))?.trim();
        if let Some((surname,first_name)) = name.split_once(',') {
            // println!("name : {} electorate {} email {} role {}",name,electorate,email,role);
            let mp = MP{
                first_name: first_name.trim().to_string(),
                surname: surname.trim().to_string(),
                electorate: Electorate { chamber: Chamber::ACT_Legislative_Assembly, region: Some(electorate.to_string()) },
                email: email.to_string(),
                role
            };
            mps.push(mp);
        } else { return Err(anyhow!("Name {} does not contain a comma in ACT table",name))}
    }
    Ok(mps)
}

fn extract_electorates(mps : &[MP]) -> anyhow::Result<HashSet<String>> {
    mps.iter().map(|mp|mp.electorate.region.as_ref().map(|s|s.to_string()).ok_or_else(||anyhow!("Missing electorate"))).collect()
}

/// Download, check, and if valid replace the downloaded files with MP lists.
pub async fn update_mp_list_of_files() -> anyhow::Result<()> {
    std::fs::create_dir_all(TEMP_DIR)?;
    std::fs::create_dir_all(MP_SOURCE)?;
    let dir = PathBuf::from_str(MP_SOURCE)?;
    // Federal CSVs.
    let house_reps = download_to_file("https://www.aph.gov.au/-/media/03_Senators_and_Members/Address_Labels_and_CSV_files/FamilynameRepsCSV.csv").await?;
    let australian_house_reps_res = parse_australian_house_reps(house_reps.reopen()?)?;
    house_reps.persist(dir.join(Chamber::Australian_House_Of_Representatives.to_string()+".csv"))?;
    let senate = download_to_file("https://www.aph.gov.au/-/media/03_Senators_and_Members/Address_Labels_and_CSV_files/Senators/allsenel.csv").await?;
    parse_australian_senate(senate.reopen()?)?;
    senate.persist(dir.join(Chamber::Australian_Senate.to_string()+".csv"))?;
    // Federal PDFs.
    let senate_pdf = download_to_file("https://www.aph.gov.au/-/media/03_Senators_and_Members/31_Senators/contacts/los.pdf").await?;
    parse_australian_senate_pdf(senate_pdf.path())?;
    senate_pdf.persist(dir.join(Chamber::Australian_Senate.to_string()+".pdf"))?;
    let house_reps_pdf = download_to_file("https://www.aph.gov.au/-/media/03_Senators_and_Members/32_Members/Lists/Members_List_2021.pdf").await?;
    parse_australian_house_reps_pdf(house_reps_pdf.path(),&extract_electorates(&australian_house_reps_res)?)?;
    house_reps_pdf.persist(dir.join(Chamber::Australian_House_Of_Representatives.to_string()+".pdf"))?;

    // NSW
    let la = download_to_file("https://www.parliament.nsw.gov.au/_layouts/15/NSWParliament/memberlistservice.aspx?members=LA&format=Excel").await?;
    parse_nsw_la(la.reopen()?)?;
    la.persist(dir.join(Chamber::NSW_Legislative_Assembly.to_string()+".csv"))?;
    let lc = download_to_file("https://www.parliament.nsw.gov.au/_layouts/15/NSWParliament/memberlistservice.aspx?members=LA&format=Excel").await?;
    parse_nsw_lc(lc.reopen()?)?;
    lc.persist(dir.join(Chamber::NSW_Legislative_Council.to_string()+".csv"))?;

    // ACT
    let la = download_to_file("https://www.parliament.act.gov.au/members/members-of-the-assembly").await?;
    parse_act_la(la.path())?;
    la.persist(dir.join(Chamber::ACT_Legislative_Assembly.to_string()+".html"))?;

    Ok(())

}

/// Create "data/MP_source/MPs.json" from the source files downloaded by update_mp_list_of_files()
pub fn create_mp_list() -> anyhow::Result<()> {
    let dir = PathBuf::from_str(MP_SOURCE)?;
    let mut mps = Vec::new();
    { // deal with Federal (Senate and House of Reps).
        let mut reps_from_csvs = parse_australian_house_reps(File::open(dir.join(Chamber::Australian_House_Of_Representatives.to_string()+".csv"))?)?;
        let senate_emails = parse_australian_senate_pdf(&dir.join(Chamber::Australian_Senate.to_string()+".pdf"))?;
        let reps_emails = parse_australian_house_reps_pdf(&dir.join(Chamber::Australian_House_Of_Representatives.to_string()+".pdf"),&extract_electorates(&reps_from_csvs)?)?;
        let mut senate_from_csvs = parse_australian_senate(File::open(dir.join(Chamber::Australian_Senate.to_string()+".csv"))?)?;
        for mp in &mut senate_from_csvs {
            senate_emails.add_email(mp)?;
        }
        mps.extend(senate_from_csvs);
        for mp in &mut reps_from_csvs {
            mp.email = reps_emails.get(mp.electorate.region.as_ref().ok_or_else(||anyhow!("No electorate for house of reps"))?).ok_or_else(||anyhow!("No email from pdf for house of reps {} {} member for {}",mp.first_name,mp.surname,mp.electorate.region.as_ref().unwrap()))?.to_string();
        }
        mps.extend(reps_from_csvs);
    }
    { // Deal with Assembly of the ACT
        mps.extend(parse_act_la(&dir.join(Chamber::ACT_Legislative_Assembly.to_string()+".html"))?);
    }
    { // Deal with NSW
        mps.extend(parse_nsw_la(File::open(dir.join(Chamber::NSW_Legislative_Assembly.to_string()+".csv"))?)?);
        mps.extend(parse_nsw_lc(File::open(dir.join(Chamber::NSW_Legislative_Council.to_string()+".csv"))?)?);
    }
    serde_json::to_writer(File::create(dir.join("MPs.json"))?,&mps)?;
    Ok(())
}