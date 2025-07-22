//! Parse various files from non-authoritative sources such as Wikipedia, to add to information
//! derived in parse_mp-lists.
//!
use crate::mp_non_authoritative::{ImageInfo, MPNonAuthoritative};
use crate::parse_util::{
    download_wiki_data_to_file, download_wikipedia_file, get_nested_json, parse_wiki_data,
    strip_quotes,
};
use crate::regions::{Chamber, Electorate, State};
use std::collections::{HashMap};
use std::fs::File;
use std::io::Write;
use tempfile::NamedTempFile;
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

fn wiki_data_code(chamber: &Chamber) -> String {
    match chamber {
        Chamber::Australian_House_Of_Representatives => "Q18912794".to_string(),
        Chamber::Australian_Senate                   => "Q6814428".to_string(),
        Chamber::ACT_Legislative_Assembly            => "Q6814365".to_string(),
        Chamber::NSW_Legislative_Assembly            => "Q19202748".to_string(),
        Chamber::NSW_Legislative_Council             => "Q18810377".to_string(),
        Chamber::NT_Legislative_Assembly             => "Q26998278".to_string(),
        Chamber::Qld_Legislative_Assembly            => "Q18526194".to_string(),
        Chamber::SA_House_Of_Assembly                => "Q18220900".to_string(),
        Chamber::SA_Legislative_Council              => "Q18662245".to_string(),
        Chamber::Tas_House_Of_Assembly               => "Q19007285".to_string(),
        Chamber::Tas_Legislative_Council             => "Q19299542".to_string(),
        Chamber::Vic_Legislative_Assembly            => "Q18534408".to_string(),
        Chamber::Vic_Legislative_Council             => "Q19185341".to_string(),
        Chamber::WA_Legislative_Assembly             => "Q20165902".to_string(),
        Chamber::WA_Legislative_Council              => "Q19627913".to_string()
    }
}

/// Get wikidata download for all the MPs in the given chamber.
/// An example for pasting into Wikidata, with districts:
/* SELECT ?mp ?mpLabel ?districtLabel ?assumedOffice where {
     ?mp p:P39 ?posheld.    # Check on the position
     ?posheld ps:P39 wd:Q18912794;
              pq:P768 ?district;
              pq:P580 ?assumedOffice. # And should have a starttime
     MINUS { ?posheld pq:P582 ?endTime. } # But not an endtime
     SERVICE wikibase:label { bd:serviceParam wikibase:language "[AUTO_LANGUAGE],mul,en". }
 }
 GROUP BY ?mp ?mpLabel ?districtLabel ?assumedOffice
 ORDER BY ?mpLabel
 LIMIT 180
*/
/// The district request is omitted for chambers with no districts (some Legislative Councils).
pub async fn get_wikidata_json(client: &reqwest::Client, chamber: &Chamber) -> anyhow::Result<NamedTempFile> {
    let fields = format!("?mp ?mpLabel{} ?assumedOffice",
                         if chamber.has_regions() {" ?districtLabel"} else {""} );
    let query_string = format!("SELECT {}{}{}{}{}{}{}{}{}{}{}{}{}",
        &fields,
"       where { ?mp p:P39 ?posheld.",    // # Check on the position
"               ?posheld ps:P39 wd:", //# Position held
        wiki_data_code(&chamber) + ";",
if chamber.has_regions() {"pq:P768 ?district;"} else {""}, // Ask for district only if the chamber has them.
"             pq:P580 ?assumedOffice.", // # And should have a starttime
"    MINUS { ?posheld pq:P582 ?endTime. }", // # But not an endtime
"    SERVICE wikibase:label { bd:serviceParam wikibase:language \"[AUTO_LANGUAGE],mul,en\". }",
"}",
" GROUP BY ", &fields,
" ORDER BY ?mpLabel",
" LIMIT 180"  // Should be large enough to guarantee no Australian parliament has more members.
    );
 
    let file: NamedTempFile = download_wiki_data_to_file(&*query_string, &client).await?;
    Ok(file)
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
    json_file: &str, chamber: &Chamber,
    opt_client: Option<&reqwest::Client>,
) -> anyhow::Result<HashMap<Electorate, Vec<MPNonAuthoritative>>> {
    println!("Getting photos and summaries - got json file {}", json_file);
    let found: Vec<(String, Option<String>, String)> = parse_wiki_data(File::open(json_file)?).await?;
    let mut results: HashMap<Electorate, Vec<MPNonAuthoritative>> = HashMap::new();

    for (name, electorate_name, id) in found {
        // Make a directory labelled with the electorate for data that will be used to find the picture, but not used after creating MPs.json.

        // FIXME Do something more intelligent than just setting to None if we got an error.
        let electorate_name = electorate_name.and_then(|e| canonicalise_electorate_name(chamber, &e).unwrap_or(None));
        let directory : String = match &electorate_name {
            Some(name) => format!( "{}/{}/{}", PICS_DIR, chamber, &name),
            None =>  format!( "{}/{}", PICS_DIR, chamber)
        };

        let non_authoritative_path = format!(
            "{}/{}/{}",
            MP_SOURCE,
            NON_AUTHORITATIVE_DIR,
            directory
        );
        std::fs::create_dir_all(&non_authoritative_path)?;

        // Make a directory labelled with the electorate, for storing image info
        // intended for server upload. That is, it will be used in addition to MPs.json.
        let uploadable_path = format!(
            "{}/{}",
            MP_SOURCE,
            directory
        );
        std::fs::create_dir_all(&uploadable_path)?;

        // FIXME - clean this up and make the different names for directories cleaner.
        // Make the MP data structure into which all this info will be stored.
        // Note that not all chambers have individual electorates.
        let mut mp: MPNonAuthoritative = MPNonAuthoritative {
            name: name.clone(),
            electorate_name: electorate_name.clone(),
            path: directory,
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
            WIKIPEDIA_API_URL, WIKIPEDIA_SITE_LINKS_REQUEST, byte_serialize(id.as_bytes()).collect::<String>()
        );
        println!("Processing {}", &name);

        let entity_file = FileThatIsSomewhere::get(
            &url,
            opt_client,
            format!("{non_authoritative_path}/{}_entity.json", &id),
        ).await?;
        let wikipedia_entity_data: serde_json::Value = entity_file.as_json()?;

        // Parse the wikipedia entity data
        let opt_title_new: Option<&str> = get_nested_json(
            &wikipedia_entity_data,
            &["entities", &id, "sitelinks", "enwiki", "title"],
        );
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
            let summary_url: String = format!(
                "{}{}{}",
                EN_WIKIPEDIA_API_URL,
                WIKIPEDIA_EXTRACT_AND_IMAGES_REQUEST,
                encoded_title
            );

            let summary_file = FileThatIsSomewhere::get(
                &summary_url,
                opt_client,
                format!("{non_authoritative_path}/{}_summary.json", &id),
            ).await?;

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
                        format!("{}{}", WIKIPEDIA_PAGE_FROM_ID, byte_serialize(page_id.as_bytes()).collect::<String>()),
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
                        let filename = byte_serialize(strip_quotes(&filename_with_quotes).as_bytes()).collect::<String>();
                        let image_metadata_url: String = format!(
                            "{EN_WIKIPEDIA_API_URL}{WIKIPEDIA_IMAGE_INFO_REQUEST}{}",
                            // Get rid of "
                            filename
                        );
                        let image_metadata_file = FileThatIsSomewhere::get(
                            &image_metadata_url,
                            opt_client,
                            format!("{non_authoritative_path}/{}_image_metadata.json", &id),
                        ).await?;

                        // First get the image metadata
                        if let Some(img_data) = parse_image_info(title, image_metadata_file.as_json()?) {
                            // Store the attribution in the appropriate directory, as a text file.
                            store_attr_txt(&img_data, &uploadable_path, title).await?;

                            // Then download the actual file
                            let image_file = FileThatIsSomewhere::get(
                                &img_data.source_url.as_ref().unwrap(),
                                opt_client,
                                format!("{uploadable_path}/{}", img_data.filename),
                            ).await?;
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

        // println!("Found MP {mp:?}");

        let electorate = Electorate {
            chamber: *chamber,
            region: electorate_name
        };
        results.entry(electorate)
            .or_insert(Vec::new())
            .push(mp); 
    }
    Ok(results)
}

/// Deal with possible discrepancies between wikipedia region names and authoritative ones.
/// For the senate, change the full state/territory name to its 2-3 char short name.
/// We may at some point have a problem with capitalisation for electorate names, but for the 
/// moment we don't.
/// TODO deal appropriately with chambers that don't have a region, e.g. NSW/SA Legislative Council.
fn canonicalise_electorate_name(chamber: &Chamber, region: &str) -> anyhow::Result<Option<String>> {
    match chamber {
        Chamber::Australian_Senate => Ok(Some(State::try_from(region.to_uppercase().as_str())?.to_string())),
        _ => Ok(Some(region.to_string())),
    }
}

/// Store a pretty-printed text file with the attribution info, into the directory in which the
/// image will be posted.
async fn store_attr_txt(img_data: &ImageInfo, path: &String, wikipedia_title: &str) -> anyhow::Result<File> {
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
    let filepath = format!("{}/{}_{}.{}", path, wikipedia_title, "attr", "txt");
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
