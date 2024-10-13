use std::{
	collections::HashMap,
	io::{Cursor, Read},
};
use xml::reader::{EventReader, XmlEvent};
use zip::ZipArchive;

use crate::IngestorError;

pub fn extract_text_and_images_from_pptx(
	bytes: &[u8],
) -> Result<(Vec<String>, HashMap<String, Vec<u8>>), IngestorError> {
	let cursor = Cursor::new(bytes);
	let mut archive = ZipArchive::new(cursor)?;

	let mut slide_texts = Vec::new();
	let mut slide_images = HashMap::new();

	for i in 0..archive.len() {
		let mut file = archive.by_index(i)?;
		let file_name = file.name().to_string();

		// Extract text from slide XML files
		if file_name.starts_with("ppt/slides/slide") && file_name.ends_with(".xml") {
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

		// Extract images from media files
		if file_name.starts_with("ppt/media/") {
			let mut img_data = Vec::new();
			file.read_to_end(&mut img_data)?;
			slide_images.insert(file_name, img_data);
		}
	}

	Ok((slide_texts, slide_images))
}
