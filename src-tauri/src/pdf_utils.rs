use lopdf::{Document, Dictionary, Object};

pub fn add_watermark_to_pdf(doc: &mut Document, text: &str) -> Result<(), String> {
    let pages = doc.get_pages();
    let page_ids: Vec<(u32, u16)> = pages.values().cloned().collect();
    
    let font_dict = Dictionary::from_iter(vec![
        ("Type", Object::Name(b"Font".to_vec())),
        ("Subtype", Object::Name(b"Type1".to_vec())),
        ("BaseFont", Object::Name(b"Helvetica".to_vec())),
        ("Name", Object::Name(b"FWM".to_vec())),
    ]);
    let font_id = doc.add_object(Object::Dictionary(font_dict));
    
    for page_id in page_ids {
        let page_obj = doc.get_object(page_id)
            .map_err(|e| format!("Failed to get page: {}", e))?;
        
        let mut page_dict = match page_obj {
            Object::Dictionary(ref d) => d.clone(),
            _ => continue,
        };
        
        let mut _width = 612.0_f32;
        let mut height = 792.0_f32;
        
        if let Ok(Object::Array(media_box)) = page_dict.get(b"MediaBox") {
            if media_box.len() >= 4 {
                if let Object::Real(w) = media_box[2] { _width = w as f32; }
                if let Object::Real(h) = media_box[3] { height = h as f32; }
            }
        }
        
        let x = 10.0;
        let y = height - 15.0;
        
        let lines: Vec<&str> = text.split('\n').collect();
        let line_height = 10.0;
        
        let mut content = String::new();
        content.push_str("q\nBT\n/FWM 8 Tf\n");
        
        if let Some(first_line) = lines.first() {
            content.push_str(&format!("{} {} Td ({}) Tj\n", x, y, first_line));
        }
        
        let total_lines = lines.len();
        for (i, line) in lines.iter().skip(1).enumerate() {
            let is_last = (i + 2) == total_lines;
            if is_last {
                content.push_str(&format!("0 {} Td ({}) Tj\n", line_height * 50.0, line));
            } else {
                content.push_str(&format!("0 {} Td ({}) Tj\n", -line_height, line));
            }
        }
        
        content.push_str("ET\nQ");
        
        let stream = lopdf::Stream::new(Dictionary::new(), content.into_bytes());
        let stream_id = doc.add_object(Object::Stream(stream));
        
        let contents = page_dict.get(b"Contents")
            .cloned()
            .unwrap_or_else(|_| Object::Array(vec![]));
        
        let new_contents = match contents {
            Object::Array(mut arr) => {
                arr.push(Object::Reference(stream_id));
                Object::Array(arr)
            }
            _ => Object::Array(vec![Object::Reference(stream_id)]),
        };
        
        page_dict.set("Contents", new_contents);
        
        if page_dict.get(b"Resources").is_err() {
            let mut resources = Dictionary::new();
            let mut fonts = Dictionary::new();
            fonts.set("FWM", Object::Reference(font_id));
            resources.set("Font", Object::Dictionary(fonts));
            page_dict.set("Resources", Object::Dictionary(resources));
        } else if let Ok(Object::Dictionary(ref mut resources)) = page_dict.get_mut(b"Resources") {
            if resources.get(b"Font").is_err() {
                let mut fonts = Dictionary::new();
                fonts.set("FWM", Object::Reference(font_id));
                resources.set("Font", Object::Dictionary(fonts));
            }
        }
        
        doc.objects.insert(page_id, Object::Dictionary(page_dict));
    }
    
    Ok(())
}

pub fn extract_signature_info(pdf_data: &[u8]) -> Option<(String, String, String, String)> {
    let pdf_string = String::from_utf8_lossy(pdf_data);
    
    let start_idx = pdf_string.find("Digitally signed by ")?;
    let after_marker = &pdf_string[start_idx..];
    
    let clean_lines = parse_signature_lines(after_marker)?;
    
    let (signer_name, timestamp, extra, signature) = match clean_lines.len() {
        len if len >= 4 => {
            let sig = if clean_lines[2].starts_with("Hash:") {
                clean_lines[2].trim_start_matches("Hash:").trim().to_string()
            } else {
                clean_lines[3].trim_start_matches("Hash:").trim().to_string()
            };
            let ext = if clean_lines[2].starts_with("Hash:") {
                "(none)".to_string()
            } else {
                clean_lines[2].clone()
            };
            (clean_lines[0].clone(), clean_lines[1].clone(), ext, sig)
        }
        len if len >= 3 => {
            let ext = if clean_lines[2].starts_with("Hash:") {
                "(none)".to_string()
            } else {
                clean_lines[2].clone()
            };
            let sig = if clean_lines[2].starts_with("Hash:") {
                clean_lines[2].trim_start_matches("Hash:").trim().to_string()
            } else {
                "SHA256: (hash not found)".to_string()
            };
            (clean_lines[0].clone(), clean_lines[1].clone(), ext, sig)
        }
        len if len >= 2 => {
            (clean_lines[0].clone(), clean_lines.get(1).cloned().unwrap_or_default(), "(none)".to_string(), "SHA256: (hash not found)".to_string())
        }
        _ => return None,
    };
    
    Some((signer_name, timestamp, extra, signature))
}

fn parse_signature_lines(after_marker: &str) -> Option<Vec<String>> {
    let mut clean_lines: Vec<String> = Vec::new();
    
    if let Some(ds_pos) = after_marker.find("Digitally signed by ") {
        let after_ds = &after_marker[ds_pos + "Digitally signed by ".len()..];
        let mut remaining = after_ds.to_string();
        
        while clean_lines.len() < 4 {
            if let Some(td_pos) = remaining.find("0 ") {
                if let Some(td_end) = remaining[td_pos..].find(" Td (") {
                    remaining = (&remaining[td_pos + td_end + " Td (".len()..]).to_string();
                } else {
                    break;
                }
            }
            
            if let Some(open_paren) = remaining.find('(') {
                if let Some(close_paren) = remaining[open_paren..].find(") Tj") {
                    let text = &remaining[open_paren + 1..open_paren + close_paren];
                    let trimmed = text.trim().to_string();
                    if !trimmed.is_empty() {
                        clean_lines.push(trimmed);
                    }
                    remaining = (&remaining[open_paren + close_paren + 4..]).to_string();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    
    if clean_lines.len() < 2 {
        clean_lines.clear();
        if let Some(ds_pos) = after_marker.find("Digitally signed by ") {
            let after_ds = &after_marker[ds_pos + "Digitally signed by ".len()..];
            if let Some(newline_pos) = after_ds.find('\n') {
                let name = after_ds[..newline_pos].trim().to_string();
                if !name.is_empty() && name != ") Tj" {
                    clean_lines.push(name);
                }
                let rest = &after_ds[newline_pos + 1..];
                for line in rest.lines().take(4) {
                    let cleaned = line.replace(") Tj", "")
                                     .replace("0 -10 Td (", "")
                                     .trim()
                                     .to_string();
                    if !cleaned.is_empty() {
                        clean_lines.push(cleaned);
                    }
                }
            }
        }
    }
    
    let clean_lines: Vec<String> = clean_lines.into_iter()
        .map(|line| {
            line.replace(") Tj", "")
                .replace("0 -10 Td (", "")
                .replace("0 500 Td (", "")
                .replace("BT", "")
                .replace("ET", "")
                .trim()
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .collect();
    
    if clean_lines.is_empty() {
        None
    } else {
        Some(clean_lines)
    }
}
