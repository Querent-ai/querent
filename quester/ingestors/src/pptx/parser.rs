use std::io::{Cursor, Read};
use xml::reader::{EventReader, XmlEvent};
use zip::ZipArchive;

use crate::IngestorError;

pub fn extract_text_from_pptx(bytes: &[u8]) -> Result<String, IngestorError> {
	let cursor = Cursor::new(bytes);
	let mut archive = ZipArchive::new(cursor)?;

	let mut slide_texts = Vec::new();

	for i in 0..archive.len() {
		let mut file = archive.by_index(i)?;
		if file.name().starts_with("ppt/slides/slide") && file.name().ends_with(".xml") {
			let mut content = String::new();
			file.read_to_string(&mut content)?;
			let parser = EventReader::from_str(&content);

			let mut current_text = String::new();
			let mut inside_text_tag = false;

			for e in parser {
				match e? {
					XmlEvent::StartElement { name, .. } =>
						if name.local_name == "t" {
							inside_text_tag = true;
						},
					XmlEvent::Characters(s) =>
						if inside_text_tag {
							current_text.push_str(&s);
						},
					XmlEvent::EndElement { name } =>
						if name.local_name == "t" {
							inside_text_tag = false;
							current_text.push(' ');
						},
					_ => {},
				}
			}
			slide_texts.push(current_text.trim().to_string());
		}
	}

	Ok(slide_texts.join("\n\n"))
}
