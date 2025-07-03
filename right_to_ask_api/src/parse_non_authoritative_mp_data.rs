//! Parse various files from non-authoritative sources such as Wikipedia, to add to information
//! derived in parse_mp-lists.
//!
use crate::mp_non_authoritative::{ImageInfo, MPNonAuthoritative};
use crate::parse_util::{
    download_wiki_data_to_file, download_wikipedia_file, get_nested_json, parse_wiki_data,
    strip_quotes,
};
use crate::regions::{Chamber, Electorate};
use anyhow::anyhow;
use itertools::assert_equal;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use tempfile::NamedTempFile;
use url::Url;
use url::form_urlencoded::byte_serialize;

pub const MP_SOURCE: &'static str = "data/MP_source";
pub const NON_AUTHORITATIVE_DIR: &'static str = "non_authoritative_data";
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

/// A temporary file that known where it should be persisted to.
/// Use this for when one creates a temporary file that one will probably want to
/// persist, but may not if it is corrupt.
struct PersistableTempFile {
    temp_file: NamedTempFile,
    place_to_persist: String,
}
impl PersistableTempFile {
    pub fn persist(self) -> anyhow::Result<()> {
        self.temp_file.persist(self.place_to_persist)?;
        Ok(())
    }
}
/// A file that can be accessed. It may be a temporary file that will be persisted if need be, or it may be
/// a permanent file that can be just accessed.
enum FileThatIsSomewhere {
    Temporary(PersistableTempFile),
    Permanent(String),
}

impl FileThatIsSomewhere {
    /// if given a client, download it to a temporary file from the url, making capable of saving to the permanent_address
    /// Otherwise assume it is at the permanent address and disregard the url.
    async fn get(
        url: &str,
        client: Option<&reqwest::Client>,
        permanent_address: String,
    ) -> anyhow::Result<FileThatIsSomewhere> {
        if let Some(client) = client {
            // download it to a temp file
            let temp_file = download_wikipedia_file(url, client).await?;
            Ok(FileThatIsSomewhere::Temporary(PersistableTempFile {
                temp_file,
                place_to_persist: permanent_address,
            }))
        } else {
            Ok(FileThatIsSomewhere::Permanent(permanent_address))
        }
    }
    fn persist_if_needed(self) -> anyhow::Result<()> {
        match self {
            FileThatIsSomewhere::Temporary(f) => f.persist(),
            _ => Ok(()),
        }
    }
    fn as_json(&self) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::from_reader(match self {
            FileThatIsSomewhere::Temporary(f) => File::open(f.temp_file.path())?,
            FileThatIsSomewhere::Permanent(s) => File::open(s)?,
        })?)
    }
}
/// Download all the non-authoritative data.
/// If the client is None, it does no downloading; if the client is present, it is used for downloads.
pub async fn get_photos_and_summaries(
    json_file: &str,
    opt_client: Option<&reqwest::Client>,
) -> anyhow::Result<HashMap<Electorate, MPNonAuthoritative>> {
    println!("Getting photos and summaries - got json file {}", json_file);
    let found: Vec<(String, String, String)> = parse_wiki_data(File::open(json_file)?).await?;
    let mut results: HashMap<Electorate, MPNonAuthoritative> = HashMap::new();

    for (name, electorate_name, id) in found {
        // Make a directory labelled with the electorate for data that will be used to find the picture, but not used after creating MPs.json.
        let non_authoritative_path = format!(
            "{}/{}/{}/{}/{}/",
            MP_SOURCE.to_string(),
            NON_AUTHORITATIVE_DIR.to_string(),
            PICS_DIR.to_string(),
            Chamber::Australian_House_Of_Representatives,
            &electorate_name
        );
        std::fs::create_dir_all(&non_authoritative_path)?;

        // Make a directory labelled with the electorate, for storing image info
        // intended for server upload. That is, it will be used in addition to MPs.json.
        let uploadable_path = format!(
            "{}/{}/{}/{}/",
            MP_SOURCE,
            PICS_DIR,
            Chamber::Australian_House_Of_Representatives,
            &electorate_name
        );
        std::fs::create_dir_all(&uploadable_path)?;

        // Make the MP data structure into which all this info will be stored.
        let mut mp: MPNonAuthoritative = MPNonAuthoritative {
            name: name.clone(),
            electorate_name: electorate_name.clone(),
            path: uploadable_path.clone(),
            ..Default::default()
        };

        // Get the person's wikipedia title from their ID (this is usually their name but may have disambiguating
        // extra characters for common names)
        // TODO Actually we should be able to pipe the IDs, e.g.
        // https://www.wikidata.org/w/api.php?action=wbgetentities&props=sitelinks/urls&ids=Q134309102|Q112131017&sitefilter=enwiki&format=json
        // and hence make far fewer queries. I _think_ a max of 50 might apply.
        // But just doing one for now.
        let url = format!(
            "{}{}{}",
            WIKIPEDIA_API_URL, WIKIPEDIA_SITE_LINKS_REQUEST, &id
        );
        println!("Processing {}", &name);

        let entity_file = FileThatIsSomewhere::get(
            &url,
            opt_client,
            format!("{non_authoritative_path}/entity.json"),
        )
        .await?;
        let wikipedia_entity_data: serde_json::Value = entity_file.as_json()?;

        // Parse the wikipedia entity data
        // TODO can delete
        let opt_title: Option<&str> = wikipedia_entity_data
            .get("entities")
            .and_then(|q| q.get(&id))
            .and_then(|i| i.get("sitelinks"))
            .and_then(|s| s.get("enwiki"))
            .and_then(|s| s.get("title"))
            .and_then(|i| i.as_str());
        let opt_title_new: Option<&str> = get_nested_json(
            &wikipedia_entity_data,
            &["entities", &id, "sitelinks", "enwiki", "title"],
        );
        // assert_equal(opt_title_new, opt_title); // TODO should be able to just use opt_title_new
        println!(
            "found title {} for url {}",
            opt_title_new.unwrap_or("NONE"),
            url
        );

        if let Some(title) = opt_title_new {
            // Now get their summary & image info using their title.
            // Again, we could pipe the titles.
            // "https://en.wikipedia.org/w/api.php?action=query&prop=extracts|pageimages&exintro=&exsentences=2&explaintext=&redirects=&format=json&titles=Ali%20France";
            let encoded_title: String = byte_serialize(title.as_bytes()).collect();
            // FIXME I do not understand why I need to do this.
            let percent_encoded_title = encoded_title.replace("+", "%20");
            // mp.wikipedia_title = Some(title.to_string());
            let summary_url: String = format!(
                "{}{}{}",
                EN_WIKIPEDIA_API_URL,
                WIKIPEDIA_EXTRACT_AND_IMAGES_REQUEST,
                // percent_encoded_title
                encoded_title
            );

            let summary_file = FileThatIsSomewhere::get(
                &summary_url,
                opt_client,
                format!("{non_authoritative_path}/summary.json"),
            )
            .await?;
            let response = summary_file.as_json()?;
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
                    mp.links.insert(
                        String::from("wikipedia"),
                        format!("{}{}", WIKIPEDIA_PAGE_FROM_ID, page_id),
                    );

                    // Add the wikipedia summary.
                    mp.wikipedia_summary = page_data
                        .get("extract")
                        .and_then(|s| s.as_str())
                        .map(|s| strip_quotes(s));
                    let image_name = page_data
                        .get("pageimage")
                        .map(|s| s.to_string().replace("\"", ""));
                    if !image_name.is_none() {
                        println!("found image name {:?} for {}", image_name.as_ref(), title);
                    }

                    if let Some(filename_with_quotes) = image_name {
                        let filename = strip_quotes(&filename_with_quotes);
                        let image_metadata_url: String = format!(
                            "{EN_WIKIPEDIA_API_URL}{WIKIPEDIA_IMAGE_INFO_REQUEST}{}",
                            // Get rid of "
                            filename.replace("\"", "")
                        );
                        let image_metadata_file = FileThatIsSomewhere::get(
                            &image_metadata_url,
                            opt_client,
                            format!("{non_authoritative_path}/image_metadata.json"),
                        )
                        .await?;

                        // First get the image metadata
                        if let Some(img_data) =
                            parse_image_info(title, image_metadata_file.as_json()?)
                        {
                            // Store the attribution in the appropriate directory, as a text file.
                            store_attr_txt(&img_data, &uploadable_path).await?;

                            // Then download the actual file
                            let image_file = FileThatIsSomewhere::get(
                                &img_data.source_url.as_ref().unwrap(),
                                opt_client,
                                format!("{uploadable_path}/{}", img_data.filename),
                            )
                            .await?;
                            image_file.persist_if_needed()?;

                            mp.img_data = Some(img_data);
                            image_metadata_file.persist_if_needed()?;
                        }
                    }
                }
            }
            summary_file.persist_if_needed()?;
        }

        entity_file.persist_if_needed()?;

        println!("Found MP {mp:?}");
        results.insert(
            Electorate {
                chamber: Chamber::Australian_House_Of_Representatives,
                region: Some(electorate_name),
            },
            mp,
        );
    }
    Ok(results)
}

/// Store a pretty-printed text file with the attribution info, into the directory in which the
/// image will be posted.
async fn store_attr_txt(img_data: &ImageInfo, path: &String) -> anyhow::Result<File> {
    std::fs::create_dir_all(crate::parse_util::TEMP_DIR)?;
    let mut attribution_file = NamedTempFile::new_in(crate::parse_util::TEMP_DIR)?;
    const UNKNOWN: &str = "Unknown";
    let short_name: String = match &img_data.attribution_short_name {
        Some(name) => name.to_string(),
        None => UNKNOWN.to_string(),
    };
    let artist: String = match &img_data.artist {
        Some(name) => name.to_string(),
        None => UNKNOWN.to_string(),
    };
    let attr = format!(
        "Artist: {}. License: {} via Wikimedia Commons.\n",
        artist,
        if let Some(attribution_url) = &img_data.attribution_url {
            format!("{} {}", short_name, attribution_url)
        } else {
            short_name
        }
    );
    attribution_file.write_all(attr.as_bytes())?;
    attribution_file.flush()?;
    let filepath = format!("{}/{}.{}", path, "attr", "txt");
    Ok(attribution_file.persist(&filepath)?)
}

/// parse image metadata
fn parse_image_info(title: &str, json: serde_json::Value) -> Option<ImageInfo> {
    let opt_pages = json
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

            if let Some(url) = image_info
                .get("url")
                .and_then(|u| u.as_str())
                .map(|s| strip_quotes(s)) {

                if let Some(ext_pos) = url.rfind('.') {
                    let filename = format!("{}{}", title, &url[ext_pos..]);

                    let info: ImageInfo = ImageInfo {
                        description,
                        filename,
                        artist,
                        source_url: Some(url),
                        attribution_short_name: license_short,
                        attribution_url: license_url,
                    };
                    return Some(info);
                }
            }
        }
    }
    None
}
