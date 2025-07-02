//! Parse various files from non-authoritative sources such as Wikipedia, to add to information
//! derived in parse_mp-lists.
//!
use crate::mp::{MP, MPSpec};
use crate::parse_pdf_util::{extract_string, parse_pdf_to_strings_with_same_font};
use crate::parse_util::{download_to_file, download_wiki_data_to_file, download_wikipedia_data, download_wikipedia_file, parse_wiki_data, strip_quotes};
use crate::regions::{Chamber, Electorate, RegionContainingOtherRegions, State};
use anyhow::anyhow;
use calamine::{Reader, Xls, Xlsx, open_workbook};
use encoding_rs_io::DecodeReaderBytesBuilder;
use futures::TryFutureExt;
use itertools::Itertools;
use regex::Regex;
use scraper::Selector;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::fmt::Display;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tempfile::NamedTempFile;
use toml::to_string;
use url::form_urlencoded::byte_serialize;
use crate::mp_non_authoritative::{ImageInfo, MPNonAuthoritative};

pub const MP_SOURCE: &'static str = "data/MP_source";
pub const TEMP_DIR: &'static str = "non_authoritative_data";
pub const PICS_DIR: &'static str = "pics";

const WIKIPEDIA_API_URL: &str = "https://www.wikidata.org/w/api.php?";
const EN_WIKIPEDIA_API_URL: &str = "https://en.wikipedia.org/w/api.php?";
const WIKIPEDIA_SITE_LINKS_REQUEST: &str =
    "action=wbgetentities&props=sitelinks/urls&sitefilter=enwiki&format=json&ids=";
const WIKIPEDIA_EXTRACT_AND_IMAGES_REQUEST: &str = "action=query&prop=extracts|pageimages&exintro=&exsentences=2&explaintext=&redirects=&format=json&titles=";
const WIKIPEDIA_IMAGE_INFO_REQUEST: &str =
    "action=query&prop=imageinfo&iiprop=extmetadata|url&format=json&titles=File:";
// How to get a wikipedia page link from a pageID.
const WIKIPEDIA_PAGE_FROM_ID: &str = "https://en.wikipedia.org/?curid=";

/// Get wikidata download for all the house of reps MPs.
/// TODO: Think about how this should be structured for multiple chambers. Possibly we just want one
/// function and one big .json file with all the data for each chamber, or possibly we want to pass
/// the chamber into the function.
/// Also we don't use these images, so we can delete that bit.
pub async fn get_house_reps_json(client: &reqwest::Client) -> anyhow::Result<NamedTempFile> {
    let query_string = concat!(
        // "#Current members of the Australian House of Representatives with electorate, party, picture and date they assumed office\n" ,
        "SELECT ?mp ?mpLabel ?districtLabel ?partyLabel ?assumedOffice (sample(?image) as ?image) where {\n",
        "  # Get all mps\n",
        "  ?mp p:P39 ?posheld; # With position held\n",
        "           p:P102 ?partystatement. # And with a certain party\n",
        "\n",
        "  # Get the party\n",
        "  ?partystatement ps:P102 ?party.\n",
        "  MINUS { ?partystatement pq:P582 ?partyEnd. } # but minus the ones the mp is no longer a member of\n",
        "  MINUS { ?party wdt:P361 ?partOf. } # and the 'Minnesota Democratic–Farmer–Labor Party' and such\n",
        "\n",
        "  # Check on the position \n",
        "  ?posheld ps:P39 wd:Q18912794; # Position held is in the Australian house of reps\n",
        "           pq:P768 ?district;\n",
        "           pq:P580 ?assumedOffice. # And should have a starttime\n",
        "\n",
        "  MINUS { ?posheld pq:P582 ?endTime. } # But not an endtime\n",
        "\n",
        "  # Add an image\n",
        "  OPTIONAL { ?mp wdt:P18 ?image. }\n",
        "\n",
        "  SERVICE wikibase:label { bd:serviceParam wikibase:language \"[AUTO_LANGUAGE],mul,en\". }\n",
        "} GROUP BY ?mp ?mpLabel ?districtLabel ?partyLabel ?assumedOffice ORDER BY ?mpLabel",
        // " &format=json"
    );
    let file: NamedTempFile = download_wiki_data_to_file(&*query_string, &client).await?;
    // let raw_data : serde_json::Value = serde_json::from_reader(&file)?;
    Ok(file)
}

/// Returns name, district, summary and optional (path,filename) for downloaded picture,
/// as a map from electorate name to the non-authoritative data about the MP.
pub async fn process_non_authoritative_mp_data()
 -> anyhow::Result<HashMap<String, MPNonAuthoritative>> {

    // Make a directory labelled with the electorate.
    /*
    let path = format!(
        "{}/pics/{}/{}/",
        MP_SOURCE.to_string(),
        Chamber::Australian_House_Of_Representatives,
        &electorate_name
    );
    std::fs::create_dir_all(&path)?;
    */
    todo!()
}

/// Download all the non-authoritative data.
pub async fn get_photos_and_summaries(
    json_file: &str,
    client: &reqwest::Client,
) -> anyhow::Result<HashMap<String, MPNonAuthoritative>> {
    println!("Getting photos and summaries - got json file {}", json_file);
    let found: Vec<(String, String, String)> = parse_wiki_data(File::open(json_file)?).await?;
    let mut results: HashMap<String, MPNonAuthoritative> = HashMap::new();

    for (name, electorate_name, id) in found {

        // Make a directory labelled with the electorate.
        let path = format!(
            "{}/{}/{}/{}/{}/",
            MP_SOURCE.to_string(),
            TEMP_DIR.to_string(),
            PICS_DIR.to_string(),
            Chamber::Australian_House_Of_Representatives,
            &electorate_name
        );
        std::fs::create_dir_all(&path)?;

        // Make the MP data structure into which all this info will be stored.
        let mut mp: MPNonAuthoritative
            = MPNonAuthoritative { name: name.clone(), electorate_name: electorate_name.clone(),
                 path: path.clone(), ..Default::default() };

        // Get the person's wikipedia title from their ID (this is usually their name but may have disambiguating
        // extra characters for common names)
        // TODO Actually we should be able to pipe the IDs, e.g.
        // https://www.wikidata.org/w/api.php?action=wbgetentities&props=sitelinks/urls&ids=Q134309102|Q112131017&sitefilter=enwiki&format=json
        // and hence make far fewer queries. I _think_ a max of 50 might apply.
        // But just doing one for now.
        let url = format!(
            "{}{}{}",
            WIKIPEDIA_API_URL.to_string(),
            WIKIPEDIA_SITE_LINKS_REQUEST,
            &id
        );
        println!("Processing {}", &name);
        let response = download_wikipedia_data(url.as_str(), client).await?;
        let opt_title: Option<&str> = response
            .get("entities")
            .and_then(|q| q.get(&id))
            .and_then(|i| i.get("sitelinks"))
            .and_then(|s| s.get("enwiki"))
            .and_then(|s| s.get("title"))
            .and_then(|i| i.as_str());
        println!(
            "found title {} for url {}",
            opt_title.unwrap_or("NONE"),
            url
        );

        if let Some(title) = opt_title {
            // Now get their summary & image info using their title.
            // Again, we could pipe the titles.
            // "https://en.wikipedia.org/w/api.php?action=query&prop=extracts|pageimages&exintro=&exsentences=2&explaintext=&redirects=&format=json&titles=Ali%20France";
            let encoded_title: String = byte_serialize(title.as_bytes()).collect();
            // FIXME I do not understand why I need to do this.
            let percent_encoded_title = encoded_title.replace("+", "%20");
            // mp.wikipedia_title = Some(title.to_string());
            let summary_url: String = format!(
                "{}{}{}",
                EN_WIKIPEDIA_API_URL.to_string(),
                WIKIPEDIA_EXTRACT_AND_IMAGES_REQUEST.to_string(),
                percent_encoded_title
            );
            let response = download_wikipedia_data(summary_url.as_str(), client).await?;
            // let mut image_name: Option<&Value> = None;
            // There's actually only one page number per page (I think), but since we don't know what they are,
            // the easiest way to get them is to iterate over them.
            let opt_pages = response
                .get("query")
                .and_then(|q| q.get("pages"))
                .and_then(|p| p.as_object());
            // There's only ever 1 page, so just get the first one (but if there happened to be more we would miss them).
            if let Some(pages) = opt_pages {
                if let Some((page_id, page_data)) = pages.iter().next() {

                    // Add the wikipedia page as a link.
                    mp.links.insert(String::from("wikipedia"),
                                 format!("{}{}", WIKIPEDIA_PAGE_FROM_ID, page_id.to_string()));

                    // Add the wikipedia summary.
                    mp.wikipedia_summary = page_data
                        .get("extract")
                        .and_then(|s| s.as_str())
                        .map(|s| strip_quotes(s));
                    let image_name = page_data.get("pageimage").map(|s| s.to_string().replace("\"",""));
                    if !image_name.is_none() {
                        println!(
                            "found image name {:?} for {}",
                            image_name.as_ref(),
                            title
                        );
                    }

                    if let Some(filename_with_quotes) = image_name {

                        // First get the image metadata
                        let img_data: ImageInfo = get_image_info(strip_quotes(filename_with_quotes.as_str()).as_str(), client).await?;

                        // Store the attribution in the appropriate directory, as a text file.
                        store_attr_txt(&img_data, &path).await?;

                        // Then download the actual file
                        if let Some(img_url) = &img_data.source_url {
                            let tempfile = download_wikipedia_file(&img_url, client).await?;
                            let extn_regexp = Regex::new(r".(?<extn>\w+)$").unwrap();
                            let extn = &extn_regexp.captures(&img_url).unwrap()["extn"].to_string();
                            println!("Got image {} with extension {}", url, &extn);
                            let escaped_name = name.replace(" ", "_");
                            let filepath = format!("{}/{}.{}", path, escaped_name, extn);
                            tempfile.persist(&filepath)?;
                        }

                        mp.img_data = Some(img_data);
                    }
                }
            }
        }
        println!("Found MP {mp:?}");
        results.insert(electorate_name, mp);
    }
    Ok(results)
}

/// Store a pretty-printed text file with the attribution info, into the directory in which the
/// image will be posted.
async fn store_attr_txt(img_data: &ImageInfo, path: &String) -> anyhow::Result<File> {
    std::fs::create_dir_all(crate::parse_util::TEMP_DIR)?;
    let mut attribution_file = NamedTempFile::new_in(crate::parse_util::TEMP_DIR)?;
    const UNKNOWN: &str = "Unknown";
    let short_name : String = match &img_data.attribution_short_name {
        Some(name) => name.to_string(),
        None => UNKNOWN.to_string(),
    };
    let artist : String = match &img_data.artist {
        Some(name) => name.to_string(),
        None => UNKNOWN.to_string(),
    };
    let attr = format!(
        "Artist: {}. License: {} via Wikimedia Commons.\n",
        artist,
        if let Some(attribution_url) = &img_data.attribution_url {
            format!(
                "<A href={}>{}</A>",
                attribution_url,
                short_name
            )
        } else {
            short_name
        }
    );
    attribution_file.write_all(attr.as_bytes())?;
    attribution_file.flush()?;
    let filepath = format!("{}/{}.{}", path, "attr", "txt");
    Ok(attribution_file.persist(&filepath)?)
}

async fn get_image_info(filename: &str, client: &reqwest::Client) -> anyhow::Result<ImageInfo> {
    let metadata_url: String = format!(
        "{}{}{}",
        EN_WIKIPEDIA_API_URL.to_string(),
        WIKIPEDIA_IMAGE_INFO_REQUEST.to_string(),
        // Get rid of "
        filename.to_string().replace("\"", "")
    );
    let response = download_wikipedia_data(metadata_url.as_str(), client).await?;
    let opt_pages = response
        .get("query")
        .and_then(|q| q.get("pages"))
        .and_then(|p| p.as_object());

    // There's only ever 1 page, but if there happened to be more we would miss them.

    // .get("entities")
    //    .and_then(|q| q.get(&id))
    //    .and_then(|i| i.get("sitelinks"))

    if let Some(pages) = opt_pages {
        if let Some((_, page_data)) = pages.iter().next() {
            let image_info = &page_data.get("imageinfo").unwrap().as_array().unwrap()[0];
            let image_metadata = image_info.get("extmetadata").unwrap();
            let description = image_metadata
                .get("ImageDescription")
                .and_then(|d| d.get("value"))
                .and_then(|v| v.as_str())
                .map(|s| strip_quotes(s));
            let artist = image_metadata
                .get("Artist")
                .and_then(|a| a.get("value"))
                .and_then(|v| v.as_str())
                .map(|s| strip_quotes(s));
            // println!("found artist {} for {}", artist.unwrap_or(String::from("None")), filename);
            let license_short: Option<String> = image_metadata
                .get("LicenseShortName")
                .and_then(|l| l.get("value"))
                .and_then(|v| v.as_str())
                .map(|s| strip_quotes(s));

            // TODO We should probably check
            // what the license actually is, e.g. whether AttributionRequired is true.
            let license_url: Option<String> = image_metadata
                .get("LicenseUrl")
                .and_then(|l| l.get("value"))
                .and_then(|v| v.as_str())
                .map(|s| strip_quotes(s));

            let url = image_info
                .get("url")
                .and_then(|u| u.as_str())
                .map(|s| strip_quotes(s));

            // println!("found image url {} for {}", url, filename);


            let info: ImageInfo = ImageInfo {
                description,
                filename: filename.to_string(),
                artist,
                source_url: url,
                attribution_short_name: license_short,
                attribution_url: license_url.clone(),
            };
            Ok(info)
        } else {
            Err(anyhow!("Failed to get image info"))
        } } else {
            // TODO this is where the && for the if let... would work very nicely.
            Err(anyhow!("Failed to get image info"))
        }
    }
