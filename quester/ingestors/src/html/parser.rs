use std::collections::HashSet;

pub struct HtmlParser {
	body_elements: HashSet<String>,
}

impl HtmlParser {
	pub fn new() -> Self {
		HtmlParser { body_elements: HashSet::new() }
	}

	pub fn parse(&mut self, html: &str) {
		let mut in_body = false;
		let mut current_element = String::new();

		let mut chars = html.chars().peekable();

		while let Some(c) = chars.next() {
			if c == '<' {
				if !current_element.is_empty() {
					if in_body {
						self.body_elements.insert(current_element.clone());
					}
					current_element.clear();
				}

				current_element.push(c);

				while let Some(&next) = chars.peek() {
					if next == '>' {
						current_element.push(chars.next().unwrap());
						break;
					} else {
						current_element.push(chars.next().unwrap());
					}
				}

				if current_element.trim().to_lowercase() == "</body>" {
					in_body = false;
				} else if current_element.trim().to_lowercase() == "<body>" {
					in_body = true;
				}

				current_element.clear();
			} else {
				if in_body {
					current_element.push(c);
				}
			}
		}
	}

	pub fn get_body_elements(&self) -> &HashSet<String> {
		&self.body_elements
	}
}
