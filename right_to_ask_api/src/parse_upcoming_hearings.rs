//! Parse information about upcoming hearings from https://www.aph.gov.au/Parliamentary_Business/Committees/Upcoming_Public_Hearings.
//!



use std::fs::File;
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
struct UpcomingHearing {
    date_short : String,
    date_long : String,
    inquiry : String,
    committee : String,
    committee_url : String,
    chamber : String,
    location : String,
    program_url : String,
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
fn parse_hearings_main_html_file(path:&Path) -> anyhow::Result<Vec<UpcomingHearing>> {
    let mut hearings = Vec::new();
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let table = html.select(&Selector::parse("table#allCommitteeHearingsTable > tbody").unwrap()).next().ok_or_else(||anyhow!("Could not find table in main hearings html file"))?;
    let select_td = Selector::parse("td").unwrap();
    let select_a = Selector::parse("a").unwrap();
    fn rel_url(url:Option<&str>) -> anyhow::Result<String> {
        if let Some(url) = url {
            relative_url("https://www.aph.gov.au/Parliamentary_Business/Committees/Upcoming_Public_Hearings",url.trim())
        } else {
            Err(anyhow!("No URL provided in main hearings html file"))
        }
    }
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
        let committee_url = rel_url(committee_a.value().attr("href"))?;
        let chamber = tds[4].text().next().unwrap_or("").trim().to_string();
        let location = tds[5].text().next().unwrap_or("").trim().to_string();
        let program_url = rel_url(tds[6].select(&select_a).next().ok_or_else(||anyhow!("Could not find a in program column in main hearings html file"))?.value().attr("href"))?;
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
fn parse_simple_a_committee(jurisdiction:Jurisdiction,selector:&str,html:&Html,base_url:&str) -> anyhow::Result<Vec<CommitteeInfo>> {
    let mut res = Vec::new();
    let selector = Selector::parse(selector).map_err(|e|anyhow!("Could not parse selector `{}` error {:?}",selector,e))?;
    for a in html.select(&selector) {
        let name = a.text().next().unwrap_or("").trim().to_string();
        let url = rel_url_from_a(base_url,&a)?;
        let committee = CommitteeInfo{ jurisdiction, name,url}; // TODO these URLs are often a 304 link to a prettier link.
        println!("{:?}",committee);
        res.push(committee);
    }
    Ok(res)
}

fn parse_federal_committees_html_file(path:&Path) -> anyhow::Result<Vec<CommitteeInfo>> { // TODO the URLs are ugly links that 304 to nicer links.
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let parse = |jurisdiction:Jurisdiction,desc:&'static str| {
        let selector = "li#".to_string()+desc+" li a";
        parse_simple_a_committee(jurisdiction,&selector,&html,"https://www.aph.gov.au/Parliamentary_Business/Committees").context(desc)
    };
    let senate = parse(Jurisdiction::Australian_Senate,"senate")?;
    let house = parse(Jurisdiction::Australian_House_Of_Representatives,"house")?;
    let joint = parse(Jurisdiction::Federal,"joint")?;
    Ok(vec![senate,house,joint].into_iter().flatten().collect())
}

fn parse_act_committees_html_file(path:&Path) -> anyhow::Result<Vec<CommitteeInfo>> { // TODO remove dissolved committees?
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    parse_simple_a_committee(Jurisdiction::ACT,"div#main div.spf-article-title a",&html,"https://www.parliament.act.gov.au/parliamentary-business/in-committees/committees")
}

fn parse_nsw_committees_html_file(path:&Path) -> anyhow::Result<Vec<CommitteeInfo>> {  // TODO check that we only want ones without an end date.
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let table = html.select(&Selector::parse("table#tblListView > tbody").unwrap()).next().ok_or_else(||anyhow!("Could not find table in NSW committee file"))?;
    let select_td = Selector::parse("td").unwrap();
    let select_a = Selector::parse("a").unwrap();
    let base_url = "https://www.parliament.nsw.gov.au/committees/listofcommittees/pages/committees.aspx";
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
        let committee_type = tds[2].text().next().unwrap_or("").trim().to_string(); // Select or Standing or Statutory
        let start_date = tds[4].text().next().unwrap_or("").trim().to_string();
        let end_date = tds[5].text().next().unwrap_or("").trim().to_string();
        if end_date.is_empty() {
            let committee = CommitteeInfo{ jurisdiction, name,url};
            println!("{:?}",committee);
            res.push(committee);
        }
    }
    Ok(res)
}

fn parse_nt_committees_html_file(path:&Path) -> anyhow::Result<Vec<CommitteeInfo>> {
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    parse_simple_a_committee(Jurisdiction::NT,"div.content-body table tbody tr td a",&html,"https://parliament.nt.gov.au/committees/list")
}

fn parse_qld_committees_html_file(path:&Path) -> anyhow::Result<Vec<CommitteeInfo>> {
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    parse_simple_a_committee(Jurisdiction::QLD,"div.committee__listing h4 a",&html,"https://www.parliament.qld.gov.au/Work-of-Committees/Committees")
}

fn parse_tas_lc_committees_html_file(path:&Path) -> anyhow::Result<Vec<CommitteeInfo>> { // TODO it would be nice to extract the "select"/"standing"/"Government Administration A"/etc.
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    parse_simple_a_committee(Jurisdiction::Tas_Legislative_Council,"body > div tbody a",&html,"https://www.parliament.tas.gov.au/ctee/council/LCCommittees.html") // TODO replacing tbody by table enables the Government Administration A Committee. I don't know if we want this.
}

fn parse_vic_committees_html_file(path:&Path) -> anyhow::Result<Vec<CommitteeInfo>> {
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let parse = |jurisdiction:Jurisdiction,n:usize| {
        let selector = format!("div#middle article ul:nth-child({}) > li > a:last-child",n);
        parse_simple_a_committee(jurisdiction,&selector,&html,"https://www.parliament.vic.gov.au/committees/list-of-committees").context(selector)
    };
    let joint = parse(Jurisdiction::VIC,3)?;
    let council = parse(Jurisdiction::Vic_Legislative_Council,5)?;
    let assembly = parse(Jurisdiction::Vic_Legislative_Assembly,8)?; // Ugh! These numbers are very brittle.
    Ok(vec![joint,council,assembly].into_iter().flatten().collect())
}

fn parse_wa_committees_html_file(path:&Path) -> anyhow::Result<Vec<CommitteeInfo>> {
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    let parse = |jurisdiction:Jurisdiction,assembly:&str| {
        let selector = format!("div#main article.{} h3 a",assembly);
        parse_simple_a_committee(jurisdiction,&selector,&html,"https://www.parliament.wa.gov.au/parliament/commit.nsf/WCurrentCommitteesByName").context(selector)
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
const SA_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://committees-api.parliament.sa.gov.au/api/Committees?$filter=parliamentId%20eq%2054", filename: "SA_Committees.json"}; // TODO (also TAS)
const TAS_LC_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.parliament.tas.gov.au/ctee/council/LCCommittees.html", filename: "TAS_LC_Committees.html"};
const TAS_HA_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.parliament.tas.gov.au/ctee/assembly/HACommittees.html", filename: "TAS_HA_Committees.html"};
const TAS_JOINT_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.parliament.tas.gov.au/ctee/joint/JointCommittees.html", filename: "TAS_Joint_Committees.html"};
const VIC_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.parliament.vic.gov.au/committees/list-of-committees", filename: "VIC_Committees.html"};
const WA_COMMITTEE_FILE : DownloadableFile<'static> = DownloadableFile{ url: "https://www.parliament.wa.gov.au/parliament/commit.nsf/WCurrentCommitteesByName", filename: "WA_Committees.html"};

impl DownloadableFile<'static> {
    /// Download the file, run the test_function on it, and if it is OK keep the file and return the result of the test.
    async fn download_and_check<R>(&self,dir:&PathBuf,test_function: impl Fn(&Path)->anyhow::Result<R>) -> anyhow::Result<R> {
        let temp_file = download_to_file(self.url).await.context(self.url)?;
        let res = test_function(temp_file.path()).context(self.url)?;
        temp_file.persist(dir.join(self.filename)).context(self.url)?;
        Ok(res)
    }
}

/// Download, check, and if valid replace the downloaded files with MP lists. First of the two stages for generating MPs.json
pub async fn update_hearings_list_of_files() -> anyhow::Result<()> {
    std::fs::create_dir_all(HEARINGS_SOURCE)?;
    let dir = PathBuf::from_str(HEARINGS_SOURCE)?;


    // ACT_COMMITTEE_FILE.download_and_check(&dir,parse_act_committees_html_file).await?;
    // NSW_COMMITTEE_FILE.download_and_check(&dir,parse_nsw_committees_html_file).await?;
    // NT_COMMITTEE_FILE.download_and_check(&dir,parse_nt_committees_html_file).await?;
    // QLD_COMMITTEE_FILE.download_and_check(&dir,parse_qld_committees_html_file).await?;
    TAS_LC_COMMITTEE_FILE.download_and_check(&dir,parse_tas_lc_committees_html_file).await?;
    // VIC_COMMITTEE_FILE.download_and_check(&dir,parse_vic_committees_html_file).await?;
    // WA_COMMITTEE_FILE.download_and_check(&dir,parse_wa_committees_html_file).await?;
/*
    // federal
    FEDERAL_COMMITTEE_FILE.download_and_check(&dir,parse_federal_committees_html_file).await?;
    FEDERAL_HEARINGS_FILE.download_and_check(&dir,parse_hearings_main_html_file).await?;
*/
    Ok(())
}

pub async fn create_hearings_list()  -> anyhow::Result<()> {
    let dir = PathBuf::from_str(HEARINGS_SOURCE)?;
    /*
    let hearings = parse_hearings_main_html_file(&dir.join("Upcoming_Public_Hearings.html"))?;
    serde_json::to_writer(File::create(dir.join("hearings.json"))?,&hearings)?;*/
    Ok(())
}