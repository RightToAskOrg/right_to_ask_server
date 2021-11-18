use std::path::Path;


/// A PDF TJ operation takes a string, or rather an array of strings and other stuff. Extract just the string. Also works for Tj
pub(crate) fn extract_string(op:&pdf::content::Operation) -> String {
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

/// Take a PDF file, and extract the text in the file separated by what font it is in.
pub(crate) fn parse_pdf_to_strings_with_same_font(path:&Path) -> anyhow::Result<Vec<String>> {
    let mut res : Vec<String> = Vec::new();
    let pdf = pdf::file::File::open(path)?;
    let mut font_of_last_text : Option<String> = None; // the font of the last text.
    let mut current_font : Option<String> = None; // the font currently active
    for page in pdf.pages() {
        let page = page?;
        if let Some(content) = &page.contents {
            for op in &content.operations {
                match op.operator.to_uppercase().as_str() {
                    "BT" => {  current_font=None; }
                    "TF" if op.operands.len()==2 => {  current_font=Some(op.operands[0].as_name()?.to_string()); }
                    "TJ" => {
                        let text = extract_string(op);
                        if res.len()>0 && current_font==font_of_last_text { res.last_mut().unwrap().push_str(&text) }
                        else { res.push(text); font_of_last_text=current_font.clone(); }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(res)
}
/*
/// Take a PDF file, and extract the text in the file separated by what font it is in, still split up by where it came from
pub(crate) fn parse_pdf_to_string_sets_with_same_font(path:&Path) -> anyhow::Result<Vec<Vec<String>>> {
    let mut res : Vec<Vec<String>> = Vec::new();
    let pdf = pdf::file::File::open(path)?;
    let mut font_of_last_text : Option<String> = None; // the font of the last text.
    let mut current_font : Option<String> = None; // the font currently active
    for page in pdf.pages() {
        let page = page?;
        if let Some(content) = &page.contents {
            for op in &content.operations {
                match op.operator.to_uppercase().as_str() {
                    "BT" => {  current_font=None; }
                    "TF" if op.operands.len()==2 => {  current_font=Some(op.operands[0].as_name()?.to_string()); }
                    "TJ" => {
                        let text = extract_string(op);
                        if res.len()>0 && current_font==font_of_last_text { res.last_mut().unwrap().push(text) }
                        else { res.push(vec![text]); font_of_last_text=current_font.clone(); }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(res)
}*/