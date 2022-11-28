//! Parse information about upcoming hearings from https://www.aph.gov.au/Parliamentary_Business/Committees/Upcoming_Public_Hearings.
//!



use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use anyhow::{anyhow, Context};
use scraper::{ElementRef, Html, Selector};
use serde::{Serialize,Deserialize};
use crate::committee::CommitteeInfo;
use crate::parse_util::{download_to_file, relative_url};
use crate::regions::Jurisdiction;

pub const HEARINGS_SOURCE : &'static str = "data/upcoming_hearings";

#[derive(Serialize,Deserialize,Debug)]
pub struct UpcomingHearing {
    date_short : String,
    date_long : String,
    inquiry : String,
    committee : String,
    committee_url : Option<String>,
    chamber : String,
    location : String,
    program_url : Option<String>,
}

/// Parse hearings html file
/// A typical row will look like
/// ```text
///      <tr id="main_0_content_1_lvCommittees_trRow1_0" class="toggle-hearing-info" data-child-information="&lt;strong>Time: &lt;/strong>9:00 AM - 11:00 PM&lt;br />&lt;strong>Location: &lt;/strong>Committee Room 2S3, Parliament House&lt;br />&lt;strong>Contact: &lt;/strong>Committee Secretary, Phone: &lt;a href=&#39;tel:+61(02) 6277 3526&#39;>(02) 6277 3526&lt;/a>, Email: &lt;a href=&#39;mailto:ec.sen@aph.gov.au&#39;>ec.sen@aph.gov.au&lt;/a>">
// 			<td class="details-control" tabindex="0"></td>
// 			<td class="details-control"><span class='hidden'>20220404</span>Mon, 04 Apr 2022 - Tue, 05 Apr 2022</td>
// 			<td class="details-control">Environment and Communications Legislation Committee Budget Estimates 2022-23 hearings March and April 2022<a id="main_0_content_1_lvCommittees_hlInquiryTitle_0" target="_blank"></a></td>
// 			<td class="details-control"><a id="main_0_content_1_lvCommittees_hlCommitteeTitle_0" rel="noopener noreferrer" href="/Parliamentary_Business/Committees/Senate/Environment_and_Communications" target="_blank">Environment and Communications Legislation Committee</a></td>
// 			<td class="details-control">Senate</td>
// 			<td class="details-control">CANBERRA, ACT</td>
// 			<td class="details-control"><a href="/-/media/Estimates/ec/bud2223/ec.pdf?la=en" alt="1"><img title="PDF Format" alt="1" src="/-/media/Images/pdf.png" /></a></td>
// 		</tr>
/// ```
fn parse_hearings_main_html_file(path:&Path,base_url:&str) -> anyhow::Result<Vec<UpcomingHearing>> {
    let mut hearings = Vec::new();
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    if let Some(table) = html.select(&Selector::parse("table#allCommitteeHearingsTable > tbody").unwrap()).next() { // there will be no table if there are no hearings.
        let select_td = Selector::parse("td").unwrap();
        let select_a = Selector::parse("a").unwrap();
        for tr in table.select(&Selector::parse("tr").unwrap()) {
            let data_child = tr.value().attr("data-child-information").ok_or_else(||anyhow!("Could not find data-child-information in main hearings html file"))?;
            println!("{}",data_child);
            let tds : Vec<_> = tr.select(&select_td).collect();
            if tds.len()!=7 { return Err(anyhow!("Unexpected number of columns in main hearings html file"))}
            let mut date_col = tds[1].text();
            let date_short = date_col.next().unwrap_or("").trim().to_string();
            let date_long = date_col.next().unwrap_or("").trim().to_string();
            let inquiry = tds[2].text().next().unwrap_or("").trim().to_string();
            let committee_a = tds[3].select(&select_a).next().ok_or_else(||anyhow!("Could not find a in committee column in main hearings html file"))?;
            let committee = committee_a.text().next().unwrap_or("").trim().to_string();
            let committee_url = rel_url_from_a(base_url,&committee_a)?;
            let chamber = tds[4].text().next().unwrap_or("").trim().to_string();
            let location = tds[5].text().next().unwrap_or("").trim().to_string();
            let program_a = tds[6].select(&select_a).next();
            let program_url = if let Some(a) = program_a { rel_url_from_a(base_url,&a)? } else { None };
            let hearing = UpcomingHearing{
                date_short,
                date_long,
                inquiry,
                committee,
                committee_url,
                chamber,
                location,
                program_url
            };
            println!("{:#?}",hearing);
            hearings.push(hearing);
        }
    }
    Ok(hearings)
}

/// Given a base url for a page and an `a` element (probably) containing a href, return (probably) a resolved absolute URL.
fn rel_url_from_a(base:&str,a:&ElementRef) -> anyhow::Result<Option<String>> {
    if let Some(rel_url) = a.value().attr("href") {
        Ok(Some(relative_url(base,rel_url.trim())?))
    } else {
        Ok(None)
    }
}

/// parse a committee that can be selected as a list of "a" html elements.
/// Non-a elements contain types of committees, which are canonicalized by passing through committee_type_canonicalizer.
fn parse_simple_a_committee(jurisdiction:Jurisdiction,selector:&str,html:&Html,base_url:&str,committee_type_canonicalizer : Option<&HashMap<String,String>>) -> anyhow::Result<Vec<CommitteeInfo>> {
    let mut res = Vec::new();
    let selector = Selector::parse(selector).map_err(|e|anyhow!("Could not parse selector `{}` error {:?}",selector,e))?;
    let mut committee_type : Option<String> = None;
    for a in html.select(&selector) {
        let name = a.text().map(|s|s.trim()).filter(|s|!s.is_empty()).next().unwrap_or("").trim().to_string(); // Get first non-trivial block of text.
        if a.value().name()=="a" {
            let url = rel_url_from_a(base_url,&a)?;
            let committee = CommitteeInfo{ jurisdiction, name,url, committee_type: committee_type.clone()}; // TODO these URLs are often a 304 link to a prettier link.
            // println!("{:?}",committee);
            res.push(committee);
        } else if let Some(canonicalizer) = committee_type_canonicalizer {
            // is a committee type
            committee_type = canonicalizer.get(&name).cloned();

        } else {
            return Err(anyhow!("Found unexpected type {} in parse_simple_a_committee",name));
        }
    }
    Ok(res)
}

fn parse_federal_committees_html_file(path:&Path,base_url:&str) -> anyhow::Result<Vec<CommitteeInfo>> { // TODO the URLs are ugly links that 304 to nicer links.
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let parse = |jurisdiction:Jurisdiction,desc:&'static str| {
        let selector = "li#".to_string()+desc+" li a";
        parse_simple_a_committee(jurisdiction,&selector,&html,base_url,None).context(desc)
    };
    let senate = parse(Jurisdiction::Australian_Senate,"senate")?;
    let house = parse(Jurisdiction::Australian_House_Of_Representatives,"house")?;
    let joint = parse(Jurisdiction::Federal,"joint")?;
    Ok(vec![senate,house,joint].into_iter().flatten().collect())
}

/// parse json records like
/// ```json
/// [
///   {"committeeId":1,"name":"Parliamentary Procedures and Practices","typeCode":"SELECT","typeName":"Select Committees","houseCode":"HA","houseName":"House of Assembly","parliamentId":49},
///   {"committeeId":4,"name":"Joint Parliamentary Services Committee","typeCode":"ADMIN","typeName":"Administrative Committees","houseCode":"JO","houseName":"Joint","parliamentId":49},
///   {"committeeId":13,"name":"Internet and Interactive Home Gambling and Gambling by Other Means of Telecommunication Committee","typeCode":"SELECT","typeName":"Select Committees","houseCode":"LC","houseName":"Legislative Council","parliamentId":49}
/// ]
/// ```
///
/// Only use the records with the largest value of parliamentId
fn parse_sa_committees_json_file(path:&Path,_base_url:&str) -> anyhow::Result<Vec<CommitteeInfo>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let records : Vec<serde_json::Value> = serde_json::from_reader(reader)?;
    let parliament_id = |r:&serde_json::Value| { r["parliamentId"].as_i64().unwrap_or(0)};
    let max_parliament_id = records.iter().map(parliament_id).max().unwrap_or(0);
    let mut res : Vec<CommitteeInfo> = vec![];
    for record in records {
        if parliament_id(&record)==max_parliament_id { // a current committee
            if let Some(name) = record["name"].as_str() {
                let committee_type = record["typeCode"].as_str().map(|s|s.to_string());
                if let Some(house) = record["houseCode"].as_str() {
                    let jurisdiction = match house {
                        "HA" => Jurisdiction::SA_House_Of_Assembly,
                        "LC" => Jurisdiction::SA_Legislative_Council,
                        "JO" => Jurisdiction::SA,
                        _ => return Err(anyhow!("Unknown houseCode value {}",house))
                    };
                    res.push(CommitteeInfo{
                        jurisdiction,
                        name: name.to_string(),
                        url: None,
                        committee_type
                    });
                }
            }
        }
    }
    Ok(res)
}


fn parse_act_committees_html_file(path:&Path,base_url:&str) -> anyhow::Result<Vec<CommitteeInfo>> {
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let mut mapper = HashMap::new();
    mapper.insert("Dissolved committees".to_string(),"dissolved".to_string());
    let mut res = parse_simple_a_committee(Jurisdiction::ACT,"div#main div.spf-article-title a , div#main hr~h2",&html,base_url,Some(&mapper))?;
    res.retain(|c|c.committee_type!=Some("dissolved".to_string())); // I am assuming we remove dissolved committees. Remove this line if we want to keep them.
    Ok(res)
}

fn parse_nsw_committees_html_file(path:&Path,base_url:&str) -> anyhow::Result<Vec<CommitteeInfo>> {  // TODO check that we only want ones without an end date.
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let table = html.select(&Selector::parse("table#tblListView > tbody").unwrap()).next().ok_or_else(||anyhow!("Could not find table in NSW committee file"))?;
    let select_td = Selector::parse("td").unwrap();
    let select_a = Selector::parse("a").unwrap();
    let mut res = Vec::new();
    for tr in table.select(&Selector::parse("tr").unwrap()) {
        let tds : Vec<_> = tr.select(&select_td).collect();
        if tds.len()!=6 { return Err(anyhow!("Unexpected number of columns in NSW committee html file"))}
        let a = tds[0].select(&select_a).next().ok_or_else(||anyhow!("Could not find a in first td in NSW committee file"))?;
        let name = a.text().next().unwrap_or("").trim().to_string();
        let url = rel_url_from_a(base_url,&a)?;
        let jurisdiction = match tds[1].text().next() {
            Some("Legislative Council") => Jurisdiction::NSW_Legislative_Council,
            Some("Legislative Assembly") => Jurisdiction::NSW_Legislative_Assembly,
            Some("Joint") => Jurisdiction::NSW,
            Some(s) => return Err(anyhow!("Unknown house {}",s)),
            None => return Err(anyhow!("Missing house")),
        };
        let committee_type = tds[2].text().next().map(|s|s.trim().to_string()); // Select or Standing or Statutory
        //let start_date = tds[4].text().next().unwrap_or("").trim().to_string();
        let end_date = tds[5].text().next().unwrap_or("").trim().to_string();
        if end_date.is_empty() {
            let committee = CommitteeInfo{ jurisdiction, name,url,committee_type};
            println!("{:?}",committee);
            res.push(committee);
        }
    }
    Ok(res)
}

fn parse_nt_committees_html_file(path:&Path,base_url:&str) -> anyhow::Result<Vec<CommitteeInfo>> {
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    parse_simple_a_committee(Jurisdiction::NT,"div.content-body table tbody tr td a",&html,base_url,None)
}

fn parse_qld_committees_html_file(path:&Path,base_url:&str) -> anyhow::Result<Vec<CommitteeInfo>> {
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    parse_simple_a_committee(Jurisdiction::QLD,"div.committee__listing h4 a",&html,base_url,None)
}

fn parse_tas_committees_html_files(juristiction:Jurisdiction,path:&Path,base_url:&str) -> anyhow::Result<Vec<CommitteeInfo>> {
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    // Fir comittees with a heading containing an a, we don't want enquiries containing an a.
    let mut mapper = HashMap::new();
    mapper.insert("Select Committees".to_string(),"select".to_string());
    mapper.insert("Standing Committees".to_string(),"standing".to_string());
    mapper.insert("Sessional Committees".to_string(),"sessional".to_string());
    let mut nonadmin = parse_simple_a_committee(juristiction,"body > div tbody a, body > div h3",&html,base_url,Some(&mapper))?;
    let administration = parse_simple_a_committee(juristiction,"body > div thead a, body > div h3",&html,base_url,Some(&mapper))?;
    nonadmin.retain(|e|e.committee_type.is_some()); // get rid of inquiries when there is a committee.
    Ok(vec![nonadmin,administration].into_iter().flatten().collect())
}
fn parse_tas_lc_committees_html_file(path:&Path,base_url:&str) -> anyhow::Result<Vec<CommitteeInfo>> { parse_tas_committees_html_files(Jurisdiction::Tas_Legislative_Council,path,base_url) }
fn parse_tas_ha_committees_html_file(path:&Path,base_url:&str) -> anyhow::Result<Vec<CommitteeInfo>> { parse_tas_committees_html_files(Jurisdiction::Tas_House_Of_Assembly,path,base_url) }
fn parse_tas_joint_committees_html_file(path:&Path,base_url:&str) -> anyhow::Result<Vec<CommitteeInfo>> { parse_tas_committees_html_files(Jurisdiction::TAS,path,base_url) }


fn parse_vic_committees_html_file(path:&Path,base_url:&str) -> anyhow::Result<Vec<CommitteeInfo>> {
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let parse = |jurisdiction:Jurisdiction,div_id:&str| {
        let selector = format!("div#{} a",div_id);
        parse_simple_a_committee(jurisdiction,&selector,&html,base_url,None).context(selector)
    };
    let joint = parse(Jurisdiction::VIC,"panel-joint-committees")?;
    let council = parse(Jurisdiction::Vic_Legislative_Council,"panel-lc-committees")?;
    let assembly = parse(Jurisdiction::Vic_Legislative_Assembly,"panel-la-committees")?;
    Ok(vec![joint,council,assembly].into_iter().flatten().collect())
}

fn parse_wa_committees_html_file(path:&Path,base_url:&str) -> anyhow::Result<Vec<CommitteeInfo>> {
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let parse = |jurisdiction:Jurisdiction,assembly:&str| {
        let selector = format!("div#main article.{} h3 a",assembly);
        parse_simple_a_committee(jurisdiction,&selector,&html,base_url,None).context(selector)
    };
    let la = parse(Jurisdiction::WA_Legislative_Assembly,"la")?;
    let lc = parse(Jurisdiction::WA_Legislative_Council,"lc")?;
    Ok(vec![la,lc].into_iter().flatten().collect())
}





/// A file that should be downloaded from `url` and stored in `filename`.
struct DownloadableFile<'a> {
    url : &'a str,
    filename : &'a str,
}

const FEDERAL_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.aph.gov.au/Parliamentary_Business/Committees", filename: "Federal_Committees.html"};
const FEDERAL_HEARINGS_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.aph.gov.au/Parliamentary_Business/Committees/Upcoming_Public_Hearings", filename: "Federal_Hearings.html"};

const ACT_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.parliament.act.gov.au/parliamentary-business/in-committees/committees", filename: "ACT_Committees.html"};
const NSW_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.parliament.nsw.gov.au/committees/listofcommittees/pages/committees.aspx", filename: "NSW_Committees.html"};
const NT_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://parliament.nt.gov.au/committees/list", filename: "NT_Committees.html"};
const QLD_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.parliament.qld.gov.au/Work-of-Committees/Committees", filename: "QLD_Committees.html"};
// The SA html file https://www.parliament.sa.gov.au/en/Committees/Committees-Detail is computed based on a json file which contains the parliamentId in it.
// Will need to change the 54, alternatively leave the whole filter off and just get the ones with the largest ids.
const SA_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://committees-api.parliament.sa.gov.au/api/Committees", filename: "SA_Committees.json"}; // removed the filter ?$filter=parliamentId%20eq%2054 from the end of the URL.
const TAS_LC_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.parliament.tas.gov.au/ctee/council/LCCommittees.html", filename: "TAS_LC_Committees.html"};
const TAS_HA_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.parliament.tas.gov.au/ctee/assembly/HACommittees.html", filename: "TAS_HA_Committees.html"};
const TAS_JOINT_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.parliament.tas.gov.au/ctee/joint/JointCommittees.html", filename: "TAS_Joint_Committees.html"};
const VIC_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.parliament.vic.gov.au/committees/list-of-committees", filename: "VIC_Committees.html"};
const WA_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.parliament.wa.gov.au/parliament/commit.nsf/WCurrentCommitteesByName", filename: "WA_Committees.html"};

impl DownloadableFile<'static> {
    /// Download the file, run the test_function on it, and if it is OK keep the file and return the result of the test.
    async fn download_and_check<R>(&self,dir:&PathBuf,test_function: impl Fn(&Path,&str)->anyhow::Result<R>) -> anyhow::Result<R> {
        let temp_file = download_to_file(self.url).await.context(self.url)?;
        let res = test_function(temp_file.path(),self.url).context(self.url)?;
        temp_file.persist(dir.join(self.filename)).context(self.url)?;
        Ok(res)
    }

    /// For a file already tested by [download_and_check], collect all the items found into an accumulator.
    async fn accumulate<R>(&self,accumulator:&mut Vec<R>,dir:&PathBuf,test_function: impl Fn(&Path,&str)->anyhow::Result<Vec<R>>) -> anyhow::Result<()> {
        let path = dir.join(self.filename);
        let mut res = test_function(&path,self.url).context(self.url)?;
        accumulator.extend(res.drain(..));
        Ok(())
    }
}

/// Download, check, and if valid replace the downloaded files with MP lists. First of the two stages for generating MPs.json
pub async fn update_hearings_list_of_files() -> anyhow::Result<()> {
    std::fs::create_dir_all(HEARINGS_SOURCE)?;
    let dir = PathBuf::from_str(HEARINGS_SOURCE)?;

    SA_COMMITTEE_FILE.download_and_check(&dir,parse_sa_committees_json_file).await?;


    ACT_COMMITTEE_FILE.download_and_check(&dir,parse_act_committees_html_file).await?;
    NSW_COMMITTEE_FILE.download_and_check(&dir,parse_nsw_committees_html_file).await?;
    NT_COMMITTEE_FILE.download_and_check(&dir,parse_nt_committees_html_file).await?;
    QLD_COMMITTEE_FILE.download_and_check(&dir,parse_qld_committees_html_file).await?;
    TAS_LC_COMMITTEE_FILE.download_and_check(&dir,parse_tas_lc_committees_html_file).await?;
    TAS_HA_COMMITTEE_FILE.download_and_check(&dir,parse_tas_ha_committees_html_file).await?;
    TAS_JOINT_COMMITTEE_FILE.download_and_check(&dir,parse_tas_joint_committees_html_file).await?;
    VIC_COMMITTEE_FILE.download_and_check(&dir,parse_vic_committees_html_file).await?;
    WA_COMMITTEE_FILE.download_and_check(&dir,parse_wa_committees_html_file).await?;
    // federal
    FEDERAL_COMMITTEE_FILE.download_and_check(&dir,parse_federal_committees_html_file).await?;
    FEDERAL_HEARINGS_FILE.download_and_check(&dir,parse_hearings_main_html_file).await?;
    Ok(())
}

pub async fn create_hearings_list()  -> anyhow::Result<()> {
    let dir = PathBuf::from_str(HEARINGS_SOURCE)?;
    let mut committees : Vec<CommitteeInfo> = vec![];
    SA_COMMITTEE_FILE.accumulate(&mut committees,&dir,parse_sa_committees_json_file).await?;
    ACT_COMMITTEE_FILE.accumulate(&mut committees,&dir,parse_act_committees_html_file).await?;
    NSW_COMMITTEE_FILE.accumulate(&mut committees,&dir,parse_nsw_committees_html_file).await?;
    NT_COMMITTEE_FILE.accumulate(&mut committees,&dir,parse_nt_committees_html_file).await?;
    QLD_COMMITTEE_FILE.accumulate(&mut committees,&dir,parse_qld_committees_html_file).await?;
    TAS_LC_COMMITTEE_FILE.accumulate(&mut committees,&dir,parse_tas_lc_committees_html_file).await?;
    TAS_HA_COMMITTEE_FILE.accumulate(&mut committees,&dir,parse_tas_ha_committees_html_file).await?;
    TAS_JOINT_COMMITTEE_FILE.accumulate(&mut committees,&dir,parse_tas_joint_committees_html_file).await?;
    VIC_COMMITTEE_FILE.accumulate(&mut committees,&dir,parse_vic_committees_html_file).await?;
    WA_COMMITTEE_FILE.accumulate(&mut committees,&dir,parse_wa_committees_html_file).await?;
    FEDERAL_COMMITTEE_FILE.accumulate(&mut committees,&dir,parse_federal_committees_html_file).await?;
    serde_json::to_writer(File::create(dir.join("committees.json"))?,&committees)?;
    let mut hearings: Vec<UpcomingHearing> = vec![];
    FEDERAL_HEARINGS_FILE.accumulate(&mut hearings,&dir,parse_hearings_main_html_file).await?;
    serde_json::to_writer(File::create(dir.join("hearings.json"))?,&hearings)?;
    Ok(())
}