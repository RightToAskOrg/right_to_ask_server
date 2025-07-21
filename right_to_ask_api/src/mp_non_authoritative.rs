
//! Extra data about MPs derived from not-necessarily-authoritative sources, e.g. Wikipedia.

use std::collections::HashMap;
use serde::{Serialize,Deserialize};

/// Information about a MP (or other human elected representative, e.g. senator).
/// Not all fields are known perfectly for each person.
/// This is Information about current MPs, rather than a definition of an MP at some point in time.
/// Defaults to None Options, empty strings and empty maps.
#[derive(Serialize,Deserialize,Debug,Clone,Default)]
pub struct MPNonAuthoritative {
    pub wikipedia_summary : Option<String>,
    pub name: String,
    pub img_data : Option<ImageInfo>, // path, filename, attribution
    pub electorate_name : Option<String>,
    pub links : HashMap<String, String>,  // meant to be, e.g. ``Wikipedia, {wikipedia page}''
    pub path: String,  // The path in our system (e.g. /chamber/electorate-name/)
}

#[derive(Serialize,Deserialize,Debug,Clone,Default)]
pub struct ImageInfo {
    pub filename: String, // The filename for our stored version (e.g. person-name.[jpg/png])
    pub description: Option<String>, // The description (to accompany the photo) - usually just the name.
    pub artist: Option<String>, // Artist name, from Wikipedia. This is often html.
    pub source_url: Option<String>, // The url we got the image from
    pub attribution_short_name: Option<String>,
    pub attribution_url: Option<String>, 
}

impl MPNonAuthoritative {
    // Just a silly function to see if we can get functions to compile.
    pub fn has_image(&self) -> bool {
        self.img_data.is_some()
    }
    
    pub fn has_image2(&self) -> bool {
        if let Some(_) = self.img_data { true } else { false }
    }
}
