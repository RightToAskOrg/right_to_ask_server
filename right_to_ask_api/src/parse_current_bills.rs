//! Parse information about upcoming hearings from https://www.aph.gov.au/Parliamentary_Business/Committees/Upcoming_Public_Hearings.
//!

use std::cmp;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use anyhow::{anyhow, Context};
use itertools::Itertools;
use scraper::{Selector};
use serde::{Serialize,Deserialize};
use crate::parse_util::{download_to_file};

pub const BILLS_SOURCE : &'static str = "data/current_bills";
const APH_ROOT_URL : &'static str = "https://www.aph.gov.au";
const BILLS_URL_PREFIX: &'static str = "/Parliamentary_Business/Bills_Legislation/Bills_Search_Results/Result?bId=";
// This seems to be the most it will accept.
// TODO - Check that the number of results it returns is not greater than this.
const MAX_RESULTS : u8 = 100;
// TODO - produce this more elegantly, using MAX_RESULTS.
// const BILLS_BEFORE_PARLIAMENT_URL: String = format!("https://www.aph.gov.au/Parliamentary_Business/Bills_Legislation/Bills_before_Parliament?ps={}", MAX_RESULTS);
const BILLS_BEFORE_PARLIAMENT_URL: &'static str = "https://www.aph.gov.au/Parliamentary_Business/Bills_Legislation/Bills_before_Parliament?ps=100";
const FEDERAL_BILLS_FILE : DownloadableFile<'static> = DownloadableFile{ url: BILLS_BEFORE_PARLIAMENT_URL, filename: "Federal_Bills.html"};

#[derive(Serialize,Deserialize,Debug)]
pub struct CurrentBill {
    title : String,
    id : String,
    url : String,
    summary_text : String,
    category : String,
    sponsor: String,
    // TODO status could be an enum, matching the enum in the json config.
    status : String
}

/// Parse bills html file
/// the results are inside a ul tag like: <ul class="search-filter-results">
/// A typical line will look like
/// ```text
/// <li>
//                     <div class="row">
//                         <h4 class="medium-11 small-8 columns">
//                             <a id="main_0_content_0_lvResults_hlTitle_0" href="/Parliamentary_Business/Bills_Legislation/Bills_Search_Results/Result?bId=r7344">Aged Care (Accommodation Payment Security) Levy Amendment Bill 2025</a></h4>
//                         <p class="action medium-2 small-4 columns">
//                             <a data-target="/Help/secure/my-parliament/track-item-popup?type=Bill&id=r7344&meta=" href="#" class="colorbox-popup button btn-track">Track</a>
//                             <span class=""><a href="#" onclick="$.colorbox({href:'/overlays/Message.aspx?trackingwhatsthis=1',width: '80%', maxWidth:'80%',opacity: 0}); return false;" aria-label="Information on tracking">(What's this?)</a></span>
//                         </p>
//                     </div>
//                     <div>
//                         <dl class="dl--inline text-small">
//                             <dt>Date</dt>
//                             <dd> 24 Jul 2025&nbsp;</dd>
//                             <dt>Chamber</dt>
//                             <dd> House of Representatives&nbsp;</dd>
//                             <dt>Status</dt>
//                             <dd> Before Senate&nbsp;</dd>
//                             <dt> Portfolio</dt>
//                             <dd> Health, Disability and Ageing&nbsp;</dd>
//                             <dt> Summary</dt>
//                             <dd> Introduced for some noble purpose... </dd>
//                         </dl>
//                         <p class="extra">
//                             <a id="main_0_content_0_lvResults_hlBill_0" aria-label="Bill link - Aged Care (Accommodation Payment Security) Levy Amendment Bill 2025" rel="noopener noreferrer" href="https://parlinfo.aph.gov.au/parlInfo/search/display/display.w3p;query=Id%3A%22legislation%2Fbills%2Fr7344_first-reps%2F0000%22;rec=0" target="_blank">Bill</a>
//                              |
//                             <a id="main_0_content_0_lvResults_hlExplanatoryMemorandum_0" aria-label="Explanatory Memorandum - Aged Care (Accommodation Payment Security) Levy Amendment Bill 2025 - legislation/ems/r7344_ems_c05e65fe-561d-474b-b3d1-3190156ec9e8" rel="noopener noreferrer" href="https://parlinfo.aph.gov.au/parlInfo/search/display/display.w3p;query=Id%3A%22legislation%2Fems%2Fr7344_ems_c05e65fe-561d-474b-b3d1-3190156ec9e8%22" target="_blank">Explanatory Memorandum</a>
//                         </p>
//                     </div>
//                 </li>
/// ```
/// Some have a 'Sponsor' instead of a 'Portfolio'.
fn parse_bills_main_html_file(path:&Path,base_url:&str) -> anyhow::Result<Vec<CurrentBill>> {
    let mut bills = Vec::new();
    let html = scraper::Html::parse_document(&std::fs::read_to_string(path)?);
    if let Some(list) = html.select(&Selector::parse(r#"ul[class="search-filter-results"]"#).unwrap()).next() {
        for tr in list.select(&Selector::parse("li").unwrap()) {
            // For now, let's just see if we can get the title.
            let select_div = Selector::parse("div").unwrap();
            let mut divs = tr.select(&select_div);
            let first_div = divs.next().ok_or_else(|| anyhow!("Missing first div"))?;
            let bill_headers = first_div.select(&Selector::parse("h4 > a").unwrap()).next().ok_or(anyhow!("Missing headers"))?;
            let main_page_url = bill_headers.value().attr("href").ok_or_else(||anyhow!("Could not find bill href in main bills html file"))?.to_string();
            let id = main_page_url.trim_start_matches(BILLS_URL_PREFIX).to_string();
            let title = bill_headers.text().collect::<String>();
            let second_div = divs.next().ok_or_else(|| anyhow!("Missing second div"))?;
            let list = second_div.select(&Selector::parse("dl").unwrap()).next().ok_or_else(|| anyhow!("Missing date"))?;
            let terms : Vec<_> = list.select(&Selector::parse("dt").unwrap()).collect();
            let descriptions : Vec<_> = list.select(&Selector::parse("dd").unwrap()).collect();
            let mut summary_text = String::new();
            let mut status = String::new();
            // Some bills have a 'portfolio' which is a department; others (which I think are private members' or senators' bills) have a sponsor.
            let mut category : String = String::from("private");
            let mut sponsor : String = String::new();
            let length = cmp::min(terms.len(), descriptions.len()) ;
            for i in 0..length {
                let term = terms[i].text().collect::<Vec<&str>>();
                let term = term.first().ok_or_else(|| anyhow!("Could not find term in bill id {}", &id))?.trim();
                if term.eq("Summary") {
                    summary_text = descriptions[i].text().collect::<Vec<&str>>().iter().map(|s| s.trim()).join(" ");
                }
                if term.eq("Portfolio") {
                   category = descriptions[i].text().collect::<Vec<&str>>().iter().map(|s| s.trim()).join(" ");
                }
                if term.eq("Sponsor") {
                    sponsor = descriptions[i].text().collect::<Vec<&str>>().iter().map(|s| s.trim()).join(" ");
                }
                if term.eq("Status") {
                    status = descriptions[i].text().collect::<Vec<&str>>().iter().map(|s| s.trim()).join(" ");
                }
            }
            println!("Found bill {}\n at url {}\n with id {}\n and description {}", title, main_page_url, id, &summary_text);
            // TODO Add links to bill text and explanatory memorandum.
            // Align terminology with AoR config. (v1.3?)
            let bill = CurrentBill {
                title,
                category,
                sponsor,
                url: format!("{APH_ROOT_URL}{BILLS_URL_PREFIX}{}", &id),
                id,
                summary_text,
                status
            };
            bills.push(bill);
        }
    }
    Ok(bills)
}

/// A file that should be downloaded from `url` and stored in `filename`.
// TODO this is a copy-paste of the one in parse_upcoming_hearings - use that instead, or put it in a utils folder.
struct DownloadableFile<'a> {
    url : &'a str,
    filename : &'a str,
}


impl DownloadableFile<'static> {
    /// Download the file, run the test_function on it, and if it is OK keep the file and return the result of the test.
    // TODO this is a copy-paste of the one in parse_upcoming_hearings - use that instead, or put it in a utils folder.
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
pub async fn update_bills_list_of_files() -> anyhow::Result<()> {
    std::fs::create_dir_all(BILLS_SOURCE)?;
    let dir = PathBuf::from_str(BILLS_SOURCE)?;

    // federal
    FEDERAL_BILLS_FILE.download_and_check(&dir,parse_bills_main_html_file).await?;
    Ok(())
}

pub async fn create_bills_list()  -> anyhow::Result<()> {
    let dir = PathBuf::from_str(BILLS_SOURCE)?;
    let mut bills: Vec<CurrentBill> = vec![];
    FEDERAL_BILLS_FILE.accumulate(&mut bills,&dir,parse_bills_main_html_file).await?;
    serde_json::to_writer(File::create(dir.join("bills.json"))?,&bills)?;
    Ok(())
}