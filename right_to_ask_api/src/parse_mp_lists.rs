//! Parse various files from parliament websites giving lists of MPs.
//!
//! The general approach is to have a directory (MP_source) containing the source data files
//! and a generated MPs.json file. There are a series of functions which each parse the files
//! in question - these are different for each jurisdiction; files parsed include pdf, json, html, csv, xls, xlsx.
//! There are two stages to generating this file
//! * Download the needed files. After downloading each file, it is parsed and, if there are no errors, placed in MP_source. This is update_mp_list_of_files().
//! * Take all the downloaded files in MP_source, and parse each, accumulating the results and storing in MP_source. This is create_mp_list()
//!
//! This means that each file is parsed twice (who cares - it doesn't take long and is infrequent).
//! The only reason to point this out is that it is somewhat unintuitive. It has the advantage that if
//! a file changes, it doesn't overwrite the old, working, file.
//!
//! From datasources listed on https://github.com/RightToAskOrg/technical-docs/blob/main/ParliamentaryDataSources.md



use std::path::{PathBuf, Path};
use std::fs::File;
use crate::mp::{MP, MPSpec};
use crate::regions::{Electorate, Chamber, State, RegionContainingOtherRegions};
use std::str::FromStr;
use anyhow::anyhow;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use scraper::Selector;
use itertools::Itertools;
use crate::parse_pdf_util::{parse_pdf_to_strings_with_same_font, extract_string};
use regex::Regex;
use calamine::{open_workbook, Xls, Reader, Xlsx};
use crate::parse_util::download_to_file;

pub const MP_SOURCE : &'static str = "data/MP_source";

fn parse_australian_senate(file : File) -> anyhow::Result<Vec<MP>> {
    parse_csv(file, Chamber::Australian_Senate, "Surname", &["Preferred Name", "First Name"], None, Some("State"), &["Parliamentary Titles"],"Political Party")
}
fn parse_australian_house_reps(file : File) -> anyhow::Result<(Vec<MP>,Vec<RegionContainingOtherRegions>)> {
    let (mps,states) = parse_csv_getting_extra(file, Chamber::Australian_House_Of_Representatives, "Surname", &["Preferred Name", "First Name"], None, Some("Electorate"), &["Parliamentary Title", "Ministerial Title"],"Political Party",Some("State"))?;
    let mut regions_per_state : HashMap<State,Vec<String>> = HashMap::new();
    for i in 0..mps.len() {
        let state : State = State::try_from(states[i].as_str())?;
        let region = mps[i].electorate.region.as_ref().unwrap().clone();
        regions_per_state.entry(state).or_insert_with(||Vec::new()).push(region);
    }
    let states = regions_per_state.into_iter().map(|(state,regions)|RegionContainingOtherRegions{super_region:state.to_string(),regions}).collect::<Vec<_>>();
    Ok((mps,states))
}
fn parse_nsw_la(file : File) -> anyhow::Result<Vec<MP>> {
    parse_csv(file, Chamber::NSW_Legislative_Assembly, "SURNAME", &["INITIALS"], Some("CONTACT ADDRESS EMAIL"), Some("ELECTORATE"), &["MINISTRY", "OFFICE HOLDER"],"PARTY")
}
fn parse_nsw_lc(file : File) -> anyhow::Result<Vec<MP>> {
    parse_csv(file, Chamber::NSW_Legislative_Council, "SURNAME", &["INITIALS"], Some("CONTACT ADDRESS EMAIL"), None, &["MINISTRY", "OFFICE HOLDER"],"PARTY")
}
fn parse_vic_la(file : File) -> anyhow::Result<Vec<MP>> {
    parse_csv(file, Chamber::Vic_Legislative_Assembly, "LastName", &["PreferredName"], Some("Email"), Some("Electorate"), &["Minister", "Position"],"Party")
}
fn parse_vic_lc(file : File) -> anyhow::Result<Vec<MP>> {
    parse_csv(file, Chamber::Vic_Legislative_Council, "LastName", &["PreferredName"], Some("Email"), Some("Electorate"), &["Minister", "Position"],"Party")
}


/// Parse a CSV file of contacts, given the headings
fn parse_csv(file : File,chamber:Chamber,surname_heading:&str,first_name_heading:&[&str],email_heading:Option<&str>,electorate_heading:Option<&str>,role_heading:&[&str],party_heading:&str) -> anyhow::Result<Vec<MP>> {
    parse_csv_getting_extra(file,chamber,surname_heading,first_name_heading,email_heading,electorate_heading,role_heading,party_heading,None).map(|(mps,_)|mps)
}

/// Parse a CSV file of MPs, given the headings, extracting them, and optionally an extra column specified by the `extra_heading` parameter.
fn parse_csv_getting_extra(file : File,chamber:Chamber,surname_heading:&str,first_name_heading:&[&str],email_heading:Option<&str>,electorate_heading:Option<&str>,role_heading:&[&str],party_heading:&str,extra_heading:Option<&str>) -> anyhow::Result<(Vec<MP>,Vec<String>)> {
    let mut reader = csv::Reader::from_reader(file);
    let mut mps = Vec::new();
    let mut extra_vec = Vec::new();
    let headings = reader.headers()?;
    let find_heading = |name:&str|{headings.iter().position(|e|e==name)}.ok_or_else(||anyhow!("No column header {} for surname for {}",surname_heading,chamber));
    let col_surname = find_heading(surname_heading)?;
    let col_party = find_heading(party_heading)?;
    let cols_firstname : Vec<usize> = first_name_heading.into_iter().map(|&s|find_heading(s)).collect::<anyhow::Result<Vec<usize>>>()?;
    let cols_role : Vec<usize> = role_heading.into_iter().map(|&s|find_heading(s)).collect::<anyhow::Result<Vec<usize>>>()?;
    let col_electorate : Option<usize> = electorate_heading.map(find_heading).transpose()?;
    let col_email : Option<usize> = email_heading.map(find_heading).transpose()?;
    let col_extra : Option<usize> = extra_heading.map(find_heading).transpose()?;
    for record in reader.records() {
        let record = record?;
        let mp = MP {
            first_name: cols_firstname.iter().map(|&c|&record[c]).find(|s|!s.is_empty()).unwrap_or("").to_string(),
            surname: record[col_surname].to_string(),
            electorate: Electorate { chamber, region: col_electorate.map(|c|record[c].to_string()) },
            email: col_email.map(|c|&record[c]).unwrap_or("").to_string(),
            role: cols_role.iter().map(|&c|&record[c]).fold(String::new(),|s,r|if r.is_empty() {s} else {(if s.is_empty() {s} else {s+"; "})+r}),
            party: record[col_party].to_string(),
        };
        // println!("{}",mp);
        mps.push(mp);
        if let Some(col_extra) = col_extra {
            extra_vec.push(record[col_extra].to_string())
        }
    }
    Ok((mps,extra_vec))
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

/// Parse ACT legislative assembly
fn parse_act_la(path:&Path) -> anyhow::Result<Vec<MP>> {
    let mut mps = Vec::new();
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let table = html.select(&Selector::parse("table > tbody").unwrap()).next().ok_or_else(||anyhow!("Could not find table in ACT html file"))?;
    let select_td = Selector::parse("td").unwrap();
    for tr in table.select(&Selector::parse("tr").unwrap()) {
        let tds : Vec<_> = tr.select(&select_td).collect();
        if tds.len()!=6 { return Err(anyhow!("Unexpected number of columns in ACT table"))}
        let name = tds[0].text().next().ok_or_else(||anyhow!("Could not find name in ACT html file"))?.trim();
        let role = tds[1].text().map(|t|t.trim()).join("; ");
        let electorate = tds[2].text().next().ok_or_else(||anyhow!("Could not find electorate in ACT html file"))?.trim();
        let party = tds[3].text().next().ok_or_else(||anyhow!("Could not find party in ACT html file"))?.trim();
        let email = tds[4].text().find(|t|t.trim().ends_with("act.gov.au"));
        if email.is_none() { // This genuinely occurs once as of June 30, 2022 for Ed Cocks.
            println!("Warning - could not find email in ACT html file for {}", name);
        }
        let email = email.unwrap_or("");
        if let Some((surname,first_name)) = name.split_once(',') {
            // println!("name : {} electorate {} email {} role {}",name,electorate,email,role);
            let mp = MP{
                first_name: first_name.trim().to_string(),
                surname: surname.trim().to_string(),
                electorate: Electorate { chamber: Chamber::ACT_Legislative_Assembly, region: Some(electorate.to_string()) },
                email: email.to_string(),
                role,
                party : party.to_string(),
            };
            mps.push(mp);
        } else { return Err(anyhow!("Name {} does not contain a comma in ACT table",name))}
    }
    Ok(mps)
}


/// Parse WA both houses
fn parse_wa(path:&Path,chamber:Chamber) -> anyhow::Result<Vec<MP>> {
    let mut mps = Vec::new();
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let table = html.select(&Selector::parse("table > tbody").unwrap()).next().ok_or_else(||anyhow!("Could not find table in WA html file"))?;
    let select_td = Selector::parse("td").unwrap();
    for tr in table.select(&Selector::parse("tr").unwrap()) {
        let tds : Vec<_> = tr.select(&select_td).collect();
        if tds.len()!=3 { return Err(anyhow!("Unexpected number of columns in WA table"))}
        let mut member = tds[0].text();
        let first_name = member.next().ok_or_else(||anyhow!("Could not find first name in WA html file"))?.trim().trim_start_matches("Hon. ").trim_start_matches("Mr ").trim_start_matches("Ms ").trim_start_matches("Dr ").trim_start_matches("Ms ").to_string();
        let surname = member.next().ok_or_else(||anyhow!("Could not find surname in WA html file"))?.trim().to_string();
        let mut party : Option<String> = None;
        let mut roles : Vec<String> = Vec::new();
        for s in member {
            let s = s.trim();
            if s.is_empty() || s=="MLA" || s=="MLC" {}
            else if s.starts_with("Party: ") { party=Some(s.trim_start_matches("Party: ").trim().to_string())}
            else { roles.push(s.to_string()); }
        }
        let electorate = tds[1].text().next().ok_or_else(||anyhow!("Could not find electorate in WA html file"))?.trim();
        let email = tds[2].text().find(|t|t.trim().trim_end_matches(".").ends_with("@mp.wa.gov.au")).ok_or_else(||anyhow!("Could not find email in WA html file"))?.trim().trim_end_matches(".").to_string(); // Jodie Hanns has an extra period at the end of her email address.
        let mp = MP{
            first_name,
            surname,
            electorate: Electorate { chamber, region: Some(electorate.to_string()) },
            email,
            role : roles.join("; "),
            party : party.ok_or_else(||anyhow!("Could not find party in WA html file"))?,
        };
        // println!("{}",mp);
        mps.push(mp);
    }
    Ok(mps)
}


/// Parse the list of which districts are in which electorate in Victoria.
fn parse_vic_district_list(path:&Path) -> anyhow::Result<Vec<RegionContainingOtherRegions>> {
    let mut electorates = Vec::new();
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let table = html.select(&Selector::parse("table > tbody").unwrap()).next().ok_or_else(||anyhow!("Could not find table in Vic district list html file"))?;
    for tr in table.select(&Selector::parse("tr > td div.list").unwrap()) {
        let super_region = tr.select(&Selector::parse("dl dd").unwrap()).next().ok_or_else(||anyhow!("Could not find electorate in Vic district list html file"))?.text().next().ok_or_else(||anyhow!("Could not find electorate"))?.to_string();
        let regions = tr.select(&Selector::parse("div.district a").unwrap()).map(|e|e.text().next().expect("Expecting a region").to_string()).collect::<Vec<_>>();
        if !regions.is_empty() {
            //println!("Electorate {} districts {:?}",super_region,regions);
            electorates.push(RegionContainingOtherRegions{ super_region, regions });
        }
    }
    Ok(electorates)
}


/// parse NT legislative assembly.
fn parse_nt_la_pdf(path:&Path) -> anyhow::Result<Vec<MP>> {
    let mut mps = Vec::new();
    let strings = parse_pdf_to_strings_with_same_font(path)?;
    let mut history : Vec<String> = Vec::new();
    // for s in strings { println!("** {:?}",s);}
    let surname_firstname = Regex::new(r"^\d+\.\s*([^,]+),\s*\S+\s+([^,]+),\s*MLA\s*$").unwrap(); // extract a surname and firstname
    let mut found_name : Option<(String,String)> = None;
    let mut roles : Vec<String> = vec![];
    let mut electorate : Option<String> = None;
    let mut party : Option<String> = None;
    for s in strings {
        // println!("** {}",s);
        if let Some(cap) = surname_firstname.captures(&s) {
            found_name=Some((cap[1].to_string(),cap[2].to_string()));
        } else if found_name.is_some() {
            let emails = s.split_whitespace().filter(|w|w.contains("@nt.gov.au")).map(|s|s.to_string()).collect::<Vec<_>>();
            history.push(s);
            for email in emails {
                //println!("Email {}",email);
                let lower_case_email = email.to_lowercase();
                if lower_case_email.starts_with("electorate.") {
                    let lower_case_electorate = lower_case_email.trim().trim_start_matches("electorate.").trim_end_matches("@nt.gov.au"); // used to be lower case, no longer is.
                    //println!("Looking for electorate {}",lower_case_electorate);
                    for h in &history {
                        let h = h.trim();
                        let mut lower_case_and_without_whitespace = h.to_lowercase();
                        lower_case_and_without_whitespace.retain(|c| !c.is_whitespace());
                        if lower_case_and_without_whitespace.starts_with(lower_case_electorate) {
                            let mut togo = lower_case_electorate.len();
                            electorate = Some(h.chars().take_while(|c|togo>0 && (c.is_whitespace()||{ togo-=1; true})).collect());
                            let h=h[electorate.as_ref().unwrap().len()..].trim_start();
                            party = Some(if let Some(party_pos) = h.find("Party") {
                                h[..party_pos+5].to_string()
                            } else {
                                h.chars().take_while(|c|!c.is_whitespace()).collect() // probably Independent, but maybe something else...
                            });
                            break;
                        } else if h.len()>0 { roles.push(h.to_string() )}
                    }
                } else {
                    let (surname,first_name) = found_name.take().unwrap();
                    let mp = MP{
                        first_name,
                        surname,
                        electorate: Electorate { chamber: Chamber::NT_Legislative_Assembly, region: Some(electorate.take().ok_or_else(||anyhow!("No NT electorate found"))?) },
                        email: email.to_string(),
                        role: roles.join("; "),
                        party: party.take().ok_or_else(||anyhow!("No NT party found"))?,
                    };
                    println!("{}",mp);
                    mps.push(mp);
                    history.clear();
                    roles.clear();
                }
            }
        }
    }
    Ok(mps)
}

fn parse_qld_parliament(path: &Path)  -> anyhow::Result<Vec<MP>> {
    let mut mps = Vec::new();
    let mut doc : Xls<_> = open_workbook(path)?;
    for (_,sheet) in &doc.worksheets() {
        let mut iter = sheet.rows();
        if let Some(headings) = iter.next() {
            let hcol = |title:&str| headings.iter().position(|v|title==&v.to_string()).ok_or_else(||anyhow!("Could not find QLD column heading {}",title));
            let col_first = hcol("first")?;
            let col_last = hcol("last")?;
            let col_electorate = hcol("electorate")?;
            let col_role = hcol("portfolio")?;
            let col_email = hcol("Email address")?;
            let col_party = hcol("party")?;
            for row in iter {
                let cell = |col:usize| row.get(col).ok_or_else(||anyhow!("Missing data in column {} for QLD",col)).map(|v|v.to_string());
                let mp = MP{
                    first_name: cell(col_first)?,
                    surname: cell(col_last)?.trim_end_matches(" MP").to_string(),
                    electorate: Electorate { chamber: Chamber::Qld_Legislative_Assembly, region: Some(cell(col_electorate)?.trim_start_matches("Member for ").to_string()) },
                    email: cell(col_email)?,
                    role: cell(col_role)?,
                    party: cell(col_party)?,
                };
                // println!("{}",mp);
                mps.push(mp);
            }
        }
    }
    Ok(mps)
}

fn parse_sa(file:File,chamber:Chamber) -> anyhow::Result<Vec<MP>> {
    let mut mps = Vec::new();
    let raw : serde_json::Value = serde_json::from_reader(file)?;
    let raw = raw.get("memberContacts").and_then(|v|v.as_array()).ok_or_else(||anyhow!("Missing array field memberContacts for SA Json file"))?;
    for entry in raw {
        let field = |name:&str| entry.get(name).ok_or_else(||anyhow!("Missing field {} for SA Json file",name));
        let string_field = |name:&str| field(name).and_then(|v|v.as_str().map(|s|s.to_string()).ok_or_else(||anyhow!("Field {} is present but not a string for SA Json file",name)));
        let email = if chamber==Chamber::SA_Legislative_Council { field("email")?.as_str().unwrap_or("") } else { field("electorateContactDetails")?.as_array().and_then(|a|a.iter().find(|v|v.get("contactType").and_then(|s|s.as_str())==Some("Email"))).and_then(|v|v.get("detail")).and_then(|v|v.as_str()).ok_or_else(||anyhow!("Could not find email for SA Json file"))?};
        let mp = MP{
            first_name: string_field("firstName")?,
            surname: string_field("lastName")?,
            electorate: Electorate { chamber, region: if chamber==Chamber::SA_Legislative_Council {None} else {Some(string_field("electorateName")?)} },
            email: email.to_string(),  // NB Heidi Girolamo does not have an email on this list.
            role: field("positions")?.as_array().ok_or_else(||anyhow!("SA Json file position field not array")).and_then(|v|v.iter().map(|e|e.as_str().map(|s|s.to_string()).ok_or_else(||anyhow!("SA Json file position entry not string"))).collect::<anyhow::Result<Vec<String>>>())?.join("; "),
            party: string_field("politicalPartyName")?
        };
        //println!("{}",mp);
        mps.push(mp);
    }
    Ok(mps)
}

fn parse_tas(path:&Path,chamber:Chamber) -> anyhow::Result<Vec<MP>> {
    let mut mps : Vec<MP> = Vec::new();
    let mut doc : Xlsx<_> = open_workbook(path)?;
    for (_,sheet) in &doc.worksheets() {
        let mut iter = sheet.rows();
        if let Some(headings) = iter.next() {
            let hcol = |title:&str| headings.iter().position(|v|title==&v.to_string().to_lowercase()).ok_or_else(||anyhow!("Could not find TAS column heading {}",title));
            let col_first = hcol("first")?;
            let col_last = hcol("last")?;
            let col_electorate = hcol("electorate")?;
            let col_role = hcol("portfolio")?;
            let col_email = hcol("email address")?;
            let col_party = hcol("party")?;
            for row in iter {
                let cell = |col:usize| row.get(col).ok_or_else(||anyhow!("Missing data in column {} for TAS",col)).map(|v|v.to_string());
                let electorate = cell(col_electorate)?.trim().trim_start_matches("Member for ").to_string();
                let empty_electorate = electorate.is_empty();
                let mp = MP{
                    first_name: cell(col_first)?,
                    surname: cell(col_last)?.trim_end_matches(" MP").trim_end_matches(" MLC").to_string(),
                    electorate: Electorate { chamber, region: Some(electorate) },
                    email: cell(col_email)?,
                    role: cell(col_role)?,
                    party: cell(col_party)?,
                };
                if empty_electorate {
                    if let Some(last) = mps.last_mut() {
                        if last.surname==mp.surname && last.first_name==mp.first_name {// just additional role
                            last.role=if last.role.is_empty() { mp.role } else { last.role.to_string()+"; "+&mp.role};
                        } else { return Err(anyhow!("Empty electorate for TAS with different prior person.")); }
                    } else { return Err(anyhow!("Empty electorate for TAS as first entry.")); }
                } else {
                    //println!("{}",mp);
                    mps.push(mp);
                }
            }
        }
    }
    Ok(mps)
}

fn extract_electorates(mps : &[MP]) -> anyhow::Result<HashSet<String>> {
    mps.iter().map(|mp|mp.electorate.region.as_ref().map(|s|s.to_string()).ok_or_else(||anyhow!("Missing electorate"))).collect()
}



/// Download, check, and if valid replace the downloaded files with MP lists. First of the two stages for generating MPs.json
pub async fn update_mp_list_of_files() -> anyhow::Result<()> {
    std::fs::create_dir_all(MP_SOURCE)?;
    let dir = PathBuf::from_str(MP_SOURCE)?;

    // NT
    let nt_members = download_to_file("https://parliament.nt.gov.au/__data/assets/pdf_file/0004/932971/MASTER-List-of-Members-Fourteenth-Assembly-as-at-August-2022.pdf").await?;
    parse_nt_la_pdf(nt_members.path())?;
    nt_members.persist(dir.join(Chamber::NT_Legislative_Assembly.to_string()+".pdf"))?;

    // Vic list of districts in each region
    let district_list = download_to_file("https://www.parliament.vic.gov.au/component/fabrik/list/26").await?;
    parse_vic_district_list(district_list.path())?;
    district_list.persist(dir.join("VicDistrictList.html"))?;

    // WA
    let la = download_to_file("https://www.parliament.wa.gov.au/parliament/memblist.nsf/WebCurrentMembLA?OpenView").await?;
    parse_wa(la.path(),Chamber::WA_Legislative_Assembly)?;
    la.persist(dir.join(Chamber::WA_Legislative_Assembly.to_string()+".html"))?;
    let lc = download_to_file("https://www.parliament.wa.gov.au/parliament/memblist.nsf/WebCurrentMembLC?OpenView").await?;
    parse_wa(lc.path(),Chamber::WA_Legislative_Council)?;
    lc.persist(dir.join(Chamber::WA_Legislative_Council.to_string()+".html"))?;

    // VIC
    let la = download_to_file("https://www.parliament.vic.gov.au/images/members/assemblymembers.csv").await?;
    parse_vic_la(la.reopen()?)?;
    la.persist(dir.join(Chamber::Vic_Legislative_Assembly.to_string()+".csv"))?;
    let lc = download_to_file("https://www.parliament.vic.gov.au/images/members/councilmembers.csv").await?;
    parse_vic_lc(lc.reopen()?)?;
    lc.persist(dir.join(Chamber::Vic_Legislative_Council.to_string()+".csv"))?;

    // TAS
    let ha = download_to_file("https://www.parliament.tas.gov.au/Members/HAMembers.xlsx").await?;
    parse_tas(ha.path(),Chamber::Tas_House_Of_Assembly)?;
    ha.persist(dir.join(Chamber::Tas_House_Of_Assembly.to_string()+".xlsx"))?;
    let lc = download_to_file("https://www.parliament.tas.gov.au/members/lcMembers.xlsx").await?;
    parse_tas(lc.path(),Chamber::Tas_Legislative_Council)?;
    lc.persist(dir.join(Chamber::Tas_Legislative_Council.to_string()+".xlsx"))?;

    // SA
    let ha = download_to_file("https://contact-details-api.parliament.sa.gov.au/api/HAMembersDetails").await?;
    parse_sa(ha.reopen()?,Chamber::SA_House_Of_Assembly)?;
    ha.persist(dir.join(Chamber::SA_House_Of_Assembly.to_string()+".json"))?;
    let lc = download_to_file("https://contact-details-api.parliament.sa.gov.au/api/LCMembersDetails").await?;
    parse_sa(lc.reopen()?,Chamber::SA_Legislative_Council)?;
    lc.persist(dir.join(Chamber::SA_Legislative_Council.to_string()+".json"))?;

    // QLD
    let qld_members = download_to_file("https://documents.parliament.qld.gov.au/Members/mailingLists/MEMMERGEEXCEL.xls").await?;
    parse_qld_parliament(qld_members.path())?;
    qld_members.persist(dir.join(Chamber::Qld_Legislative_Assembly.to_string()+".xls"))?;

    // Federal CSVs.
    let house_reps = download_to_file("https://www.aph.gov.au/-/media/03_Senators_and_Members/Address_Labels_and_CSV_files/FamilynameRepsCSV.csv").await?;
    let (australian_house_reps_res,_federal_electorates_by_state) = parse_australian_house_reps(house_reps.reopen()?)?;
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



/// Create "data/MP_source/MPs.json" from the source files downloaded by update_mp_list_of_files(). Second of the two stages for generating MPs.json
pub fn create_mp_list() -> anyhow::Result<()> {
    let dir = PathBuf::from_str(MP_SOURCE)?;
    let mut mps = Vec::new();
    let federal_electorates_by_state = { // deal with Federal (Senate and House of Reps).
        println!("Processing federal");
        let (mut reps_from_csvs,federal_electorates_by_state) = parse_australian_house_reps(File::open(dir.join(Chamber::Australian_House_Of_Representatives.to_string()+".csv"))?)?;
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
        federal_electorates_by_state
    };
    { // Deal with Assembly of the ACT
        println!("Processing ACT");
        mps.extend(parse_act_la(&dir.join(Chamber::ACT_Legislative_Assembly.to_string()+".html"))?);
    }
    { // Deal with NSW
        println!("Processing NSW");
        mps.extend(parse_nsw_la(File::open(dir.join(Chamber::NSW_Legislative_Assembly.to_string()+".csv"))?)?);
        mps.extend(parse_nsw_lc(File::open(dir.join(Chamber::NSW_Legislative_Council.to_string()+".csv"))?)?);
    }
    { // Deal with NT
        println!("Processing NT");
        mps.extend(parse_nt_la_pdf(&dir.join(Chamber::NT_Legislative_Assembly.to_string()+".pdf"))?);
    }
    { // Deal with QLD
        println!("Processing Qld");
        mps.extend(parse_qld_parliament(&dir.join(Chamber::Qld_Legislative_Assembly.to_string()+".xls"))?);
    }
    { // Deal with SA
        println!("Processing SA");
        mps.extend(parse_sa(File::open(dir.join(Chamber::SA_Legislative_Council.to_string()+".json"))?,Chamber::SA_Legislative_Council)?);
        mps.extend(parse_sa(File::open(dir.join(Chamber::SA_House_Of_Assembly.to_string()+".json"))?, Chamber::SA_House_Of_Assembly)?);
    }
    { // Deal with TAS
        println!("Processing Tas");
        mps.extend(parse_tas(&dir.join(Chamber::Tas_House_Of_Assembly.to_string()+".xlsx"),Chamber::Tas_House_Of_Assembly)?);
        mps.extend(parse_tas(&dir.join(Chamber::Tas_Legislative_Council.to_string()+".xlsx"),Chamber::Tas_Legislative_Council)?);
    }
    { // Deal with VIC
        println!("Processing Vic");
        mps.extend(parse_vic_la(File::open(dir.join(Chamber::Vic_Legislative_Assembly.to_string()+".csv"))?)?);
        mps.extend(parse_vic_lc(File::open(dir.join(Chamber::Vic_Legislative_Council.to_string()+".csv"))?)?);
    }
    { // Deal with WA
        println!("Processing WA");
        mps.extend(parse_wa(&dir.join(Chamber::WA_Legislative_Assembly.to_string()+".html"),Chamber::WA_Legislative_Assembly)?);
        mps.extend(parse_wa(&dir.join(Chamber::WA_Legislative_Council.to_string()+".html"),Chamber::WA_Legislative_Council)?);
    }
    // Vic list of districts in each region
    println!("Processing Vic districts");
    let vic_districts = parse_vic_district_list(&dir.join("VicDistrictList.html"))?;
    let spec = MPSpec { mps, federal_electorates_by_state, vic_districts };
    serde_json::to_writer(File::create(dir.join("MPs.json"))?,&spec)?;
    Ok(())
}