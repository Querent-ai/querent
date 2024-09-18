use crate::{SendableAsync, Source, SourceError, SourceErrorKind, SourceResult};
use anyhow::Result;
use async_stream::stream;
use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use common::CollectedBytes;
use futures::Stream;
use google_drive3::chrono::DateTime;
use proto::semantics::NewsCollectorConfig;
use std::{
	io::Cursor,
	ops::Range,
	path::{Path, PathBuf},
	pin::Pin,
};
use tokio::io::{AsyncRead, BufReader};

use super::structs::NewsResponse;

#[derive(Clone, Debug)]
pub struct NewsApiClient {
	client: reqwest::Client,
	api_token: String,
	query_type: String,
	query: String,
	source_id: String,
	language: Option<String>,
	sort_by: Option<String>,
	from: Option<DateTime<Utc>>,
	to: Option<DateTime<Utc>>,
	page_size: Option<u32>,
	page: Option<u32>,
	sources: Option<String>,
	domains: Option<String>,
}

impl NewsApiClient {
	pub fn new(config: NewsCollectorConfig) -> Self {
		let to_date = Self::string_to_datetime(config.to_date.clone());
		let from_date = Self::string_to_datetime(config.from_date.clone());
		NewsApiClient {
			client: reqwest::Client::new(),
			api_token: config.api_key.to_string(),
			query_type: config.query_type().as_str_name().to_string(),
			query: config.query.to_string(),
			source_id: config.id.to_string(),
			language: config.language.clone(),
			sort_by: Some(config.sort_by().as_str_name().to_string()),
			to: to_date,
			from: from_date,
			page_size: Self::i32_to_u32(config.page_size),
			page: Self::i32_to_u32(config.page),
			sources: config.sources,
			domains: config.domains,
		}
	}

	pub async fn fetch_news(&self) -> Result<NewsResponse, SourceError> {
		let url = self.create_query().await;

		let response = self
			.client
			.get(&url)
			.header(reqwest::header::USER_AGENT, "Querent/1.0")
			.send()
			.await
			.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error while making the API request: {:?}", err).into(),
				)
			});

		let news_response: NewsResponse = response?.json::<NewsResponse>().await?;

		Ok(news_response)
	}

	pub async fn create_query(&self) -> String {
		let mut url = format!(
			"https://newsapi.org/v2/{}?q={}&apiKey={}",
			self.query_type, self.query, self.api_token
		);

		if let Some(language) = &self.language {
			if !language.is_empty() {
				url.push_str(&format!("&language={}", language));
			}
		}

		if let Some(sort_by) = &self.sort_by {
			if !sort_by.is_empty() {
				url.push_str(&format!("&sortBy={}", sort_by));
			}
		}

		if let Some(page_size) = self.page_size {
			if page_size > 0 {
				url.push_str(&format!("&pageSize={}", page_size));
			}
		}

		if let Some(page) = self.page {
			if page > 0 {
				url.push_str(&format!("&page={}", page));
			}
		}

		if let Some(from) = self.from {
			if !from.format("%Y-%m-%d").to_string().is_empty() {
				url.push_str(&format!("&from={}", from.format("%Y-%m-%d")));
			}
		}

		if let Some(to) = self.to {
			if !to.format("%Y-%m-%d").to_string().is_empty() {
				url.push_str(&format!("&to={}", to.format("%Y-%m-%d")));
			}
		}

		if let Some(sources) = &self.sources {
			if !sources.is_empty() {
				url.push_str(&format!("&sources={}", sources));
			}
		}
		if let Some(domains) = &self.domains {
			if !domains.is_empty() {
				url.push_str(&format!("&domains={}", domains));
			}
		}

		url
	}

	fn string_to_datetime(s: Option<String>) -> Option<DateTime<Utc>> {
		s.and_then(|date_string| {
			NaiveDate::parse_from_str(&date_string, "%d-%m-%Y")
				.ok()
				.map(|nd| nd.and_hms_opt(0, 0, 0).unwrap_or_default())
				.map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
		})
	}

	fn i32_to_u32(value: Option<i32>) -> Option<u32> {
		value.and_then(|v| if v >= 0 { Some(v as u32) } else { None })
	}
}

fn string_to_async_read(description: String) -> impl AsyncRead + Send + Unpin {
	Cursor::new(description.into_bytes())
}

#[async_trait]
impl Source for NewsApiClient {
	async fn check_connectivity(&self) -> anyhow::Result<()> {
		let url = format!(
            "https://newsapi.org/v2/top-headlines?country=us&category=business&pagesize=1&apiKey={}",
            self.api_token
        );

		let response = self
			.client
			.get(&url)
			.header(reqwest::header::USER_AGENT, "Querent/1.0")
			.send()
			.await
			.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error while making the API request: {:?}", err).into(),
				)
			})?;

		let body = response.text().await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while reading response text: {:?}", err).into(),
			)
		})?;

		let news_response: NewsResponse = serde_json::from_str(&body).map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Failed to parse JSON response: {:?}", err).into(),
			)
		})?;

		if news_response.status != "ok" {
			return Err(anyhow::anyhow!(news_response
				.message
				.unwrap_or("Failed to set the connection for the News Source".to_string())));
		}

		Ok(())
	}

	async fn get_slice(&self, _path: &Path, _range: Range<usize>) -> SourceResult<Vec<u8>> {
		let news = self.fetch_news().await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while fetching news: {:?}", err).into(),
			)
		})?;
		let mut description: String = "".to_string();

		if news.status == "ok" {
			if let Some(articles) = news.articles {
				for individual_news in articles {
					if let Some(desc) = individual_news.description {
						description.push_str(&desc);
					}
				}
			}
		}

		Ok(description.into())
	}

	async fn get_slice_stream(
		&self,
		_path: &Path,
		_range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		let news = self.fetch_news().await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while fetching news: {:?}", err).into(),
			)
		})?;
		let mut description: String = "".to_string();

		if news.status == "ok" {
			if let Some(articles) = news.articles {
				for individual_news in articles {
					if let Some(desc) = individual_news.description {
						description.push_str(&desc);
					}
				}
			}
		}
		Ok(Box::new(string_to_async_read(description)))
	}

	async fn get_all(&self, _path: &Path) -> SourceResult<Vec<u8>> {
		let news = self.fetch_news().await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while fetching news: {:?}", err).into(),
			)
		})?;
		let mut description: String = "".to_string();

		if news.status == "ok" {
			if let Some(articles) = news.articles {
				for individual_news in articles {
					if let Some(desc) = individual_news.description {
						description.push_str(&desc);
					}
				}
			}
		}

		Ok(description.into())
	}

	async fn file_num_bytes(&self, _path: &Path) -> SourceResult<u64> {
		let news = self.fetch_news().await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while fetching news: {:?}", err).into(),
			)
		})?;
		let mut description: String = "".to_string();

		if news.status == "ok" {
			if let Some(articles) = news.articles {
				for individual_news in articles {
					if let Some(desc) = individual_news.description {
						description.push_str(&desc);
					}
				}
			}
		}
		Ok(description.len() as u64)
	}

	async fn copy_to(&self, _path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
		let news = self.fetch_news().await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while fetching news: {:?}", err).into(),
			)
		})?;
		let mut description: String = "".to_string();

		if news.status == "ok" {
			if let Some(articles) = news.articles {
				for individual_news in articles {
					if let Some(desc) = individual_news.description {
						description.push_str(&desc);
					}
				}
			}
		}
		let mut body_stream_reader = BufReader::new(description.as_bytes());
		tokio::io::copy_buf(&mut body_stream_reader, output).await?;
		Ok(())
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		let source_id = self.source_id.clone();

		let news = self.fetch_news().await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Error while fetching news: {:?}", err).into(),
			)
		})?;
		let stream = stream! {
			if news.status == "ok" {
				if let Some(articles) = news.articles {
					for individual_news in articles {
						let mut file_name = ".news".to_string();
						if let Some(ref title) = individual_news.title {
							file_name = format!("{}.news", title);
						};
						let file_name_path = Some(PathBuf::from(file_name));

						let data = individual_news.clone();
						let data_str = serde_json::to_string_pretty(&data.clone()).unwrap_or("".to_string());
						let data_len = data_str.len();

						let doc_source = Some(individual_news.source.name.unwrap_or_else(|| String::from("")));

						let collected_bytes = CollectedBytes::new(
							file_name_path,
							Some(Box::pin(string_to_async_read(data_str))),
							true,
							doc_source,
							Some(data_len),
							source_id.clone(),
							None
						);
						yield Ok(collected_bytes)

					}
				}
			}
		};

		Ok(Box::pin(stream))
	}
}

#[cfg(test)]
mod tests {

	use std::{collections::HashSet, env};

	use dotenv::dotenv;
	use futures::StreamExt;

	use super::*;

	#[tokio::test]
	async fn test_news_collector() {
		dotenv().ok();
		let news_config = NewsCollectorConfig {
			api_key: env::var("NEWS_API_KEY").map_err(|e| e.to_string()).unwrap_or("".to_string()),
			query: "Tesla".to_string(),
			query_type: 0,
			id: "Some-id".to_string(),
			sources: None,
			from_date: None,
			to_date: None,
			language: None,
			sort_by: None,
			page: None,
			page_size: Some(10),
			domains: None,
		};

		let news_api_client = NewsApiClient::new(news_config);

		let connectivity = news_api_client.check_connectivity().await;
		assert!(
			connectivity.is_ok(),
			"Failed to connect to news API {:?}",
			connectivity.err().unwrap()
		);

		let result = news_api_client.poll_data().await;

		match result {
			Ok(mut stream) => {
				let mut count_files: HashSet<String> = HashSet::new();
				while let Some(item) = stream.next().await {
					match item {
						Ok(collected_bytes) =>
							if let Some(pathbuf) = collected_bytes.file {
								if let Some(str_path) = pathbuf.to_str() {
									count_files.insert(str_path.to_string());
								}
							},
						Err(_) => panic!("Expected successful data collection"),
					}
				}
				println!("Files are --- {:?}", count_files);
			},
			Err(e) => {
				eprintln!("Failed to get stream: {:?}", e);
			},
		}
	}
}
