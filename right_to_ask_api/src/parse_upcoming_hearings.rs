//! Parse information about upcoming hearings from https://www.aph.gov.au/Parliamentary_Business/Committees/Upcoming_Public_Hearings.
//!



use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use anyhow::anyhow;
use scraper::Selector;
use serde::{Serialize,Deserialize};
use crate::parse_util::{download_to_file, relative_url};

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


/// Download, check, and if valid replace the downloaded files with MP lists. First of the two stages for generating MPs.json
pub async fn update_hearings_list_of_files() -> anyhow::Result<()> {
    std::fs::create_dir_all(HEARINGS_SOURCE)?;
    let dir = PathBuf::from_str(HEARINGS_SOURCE)?;

    // main
    let hearings_html = download_to_file("https://www.aph.gov.au/Parliamentary_Business/Committees/Upcoming_Public_Hearings").await?;
    parse_hearings_main_html_file(hearings_html.path())?;
    hearings_html.persist(dir.join("Upcoming_Public_Hearings.html"))?;
    Ok(())
}

pub async fn create_hearings_list()  -> anyhow::Result<()> {
    let dir = PathBuf::from_str(HEARINGS_SOURCE)?;
    let hearings = parse_hearings_main_html_file(&dir.join("Upcoming_Public_Hearings.html"))?;
    serde_json::to_writer(File::create(dir.join("hearings.json"))?,&hearings)?;
    Ok(())
}