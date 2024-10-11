use super::structs::NewsResponse;
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
use tokio::io::AsyncRead;

#[derive(Clone, Debug)]
pub struct NewsApiClient {
	client: reqwest::Client,
	api_token: String,
	query_type: String,
	query: Option<String>,
	source_id: String,
	language: Option<String>,
	sort_by: Option<String>,
	from: Option<DateTime<Utc>>,
	to: Option<DateTime<Utc>>,
	page_size: Option<u32>,
	sources: Option<String>,
	domains: Option<String>,
	exclude_domains: Option<String>,
	search_in: Option<String>,
	country: Option<String>,
	category: Option<String>,
}

impl NewsApiClient {
	pub async fn new(config: NewsCollectorConfig) -> anyhow::Result<Self> {
		println!("Dates-----{:?}", config.to_date);
		println!("Dates-----{:?}", config.from_date);
		let to_date = Self::string_to_datetime(config.to_date.clone());
		let from_date = Self::string_to_datetime(config.from_date.clone());
		println!("Dates-----{:?}", to_date);
		println!("Dates-----{:?}", from_date);
		Ok(NewsApiClient {
			client: reqwest::Client::new(),
			api_token: config.api_key.to_string(),
			query_type: config.query_type().as_str_name().to_string(),
			query: config.query.clone(),
			sources: config.sources.clone(),
			source_id: config.id.to_string(),
			sort_by: Some(config.sort_by().as_str_name().to_string()),
			domains: config.domains,
			to: to_date,
			from: from_date,
			language: config.language.clone(),
			page_size: config.page_size.map(|ps| ps as u32),
			exclude_domains: config.exclude_domains.clone(),
			search_in: config.search_in.clone(),
			country: config.country.clone(),
			category: config.category.clone(),
		})
	}

	pub async fn fetch_news(&self, page: u32) -> Result<NewsResponse, SourceError> {
		let url = self.create_query(page).await;
		println!("This is the url :m {:?}", url);
		let response = self
			.client
			.get(&url)
			.header(reqwest::header::USER_AGENT, "Querent/1.0")
			.send()
			.await
			.map_err(|err| {
				SourceError::new(
					SourceErrorKind::Io,
					anyhow::anyhow!("Error making the API request: {:?}", err).into(),
				)
			})?;
		let news_response: NewsResponse = response.json::<NewsResponse>().await.map_err(|err| {
			SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!("Failed to parse JSON response: {:?}", err).into(),
			)
		})?;
		if news_response.status != "ok" {
			return Err(SourceError::new(
				SourceErrorKind::Io,
				anyhow::anyhow!(news_response
					.message
					.unwrap_or_else(|| "Failed to fetch news".to_string()))
				.into(),
			));
		}

		Ok(news_response)
	}

	pub async fn create_query(&self, page: u32) -> String {	
		let mut url = "".to_string();

		if self.query_type.to_lowercase() == "everything" {
			url =
			format!("https://newsapi.org/v2/{}?apiKey={}", "everything", self.api_token);
			
			if let Some(search_in) = &self.search_in {
				url.push_str(&format!("&searchIn={}", search_in));
			}

			if let Some(domains) = &self.domains {
				url.push_str(&format!("&domains={}", domains));
			}

			if let Some(exclude_domains) = &self.exclude_domains {
				url.push_str(&format!("&excludeDomains={}", exclude_domains));
			}

			if let Some(from) = &self.from {
				url.push_str(&format!("&from={}", from.format("%Y-%m-%dT%H:%M:%SZ")));
				eprintln!("This is the url33 :m {:?}", url);
			}

			if let Some(to) = &self.to {
				url.push_str(&format!("&to={}", to.format("%Y-%m-%dT%H:%M:%SZ")));
				eprintln!("This is the url44 :m {:?}", url);
			}

			if let Some(sort_by) = &self.sort_by {
				url.push_str(&format!("&sortBy={}", sort_by));
			}
			println!("This is the url11 :m {:?}", url);
		}
		if self.query_type == "topheadlines" {
			url =
			format!("https://newsapi.org/v2/{}?apiKey={}", "top-headlines", self.api_token);
				if let Some(country) = &self.country {
					url.push_str(&format!("&country={}", country));
				}

				if let Some(category) = &self.category {
					url.push_str(&format!("&category={}", category));
				}

				if let Some(sources) = &self.sources {
					url.push_str(&format!("&sources={}", sources));
				}
				if self.sources.is_some() && (self.country.is_some() || self.category.is_some()) {
					eprintln!("Warning: 'sources' cannot be used with 'country' or 'category' in 'top-headlines'. Ignoring 'country' and 'category'.");
				}
			}

		if let Some(language) = &self.language {
			url.push_str(&format!("&language={}", language));
		}
		if let Some(query) = &self.query {
			if !query.is_empty() {
				url.push_str(&format!("&q={}", query));
			}
		}
		url.push_str(&format!("&page={}", page));
		if let Some(page_size) = self.page_size {
			url.push_str(&format!("&pageSize={}", page_size));
		}
		url
	}

	fn string_to_datetime(s: Option<String>) -> Option<DateTime<Utc>> {
		s.and_then(|date_string| {
			DateTime::parse_from_rfc3339(&date_string)
				.map(|dt| dt.with_timezone(&Utc))
				.or_else(|_| {
					NaiveDate::parse_from_str(&date_string, "%Y-%m-%d")
						.map(|nd| {
							DateTime::<Utc>::from_naive_utc_and_offset(
								nd.and_hms_opt(0, 0, 0).unwrap(),
								Utc,
							)
						})
				})
				.ok()
		})
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

		let news_response: NewsResponse = response.json::<NewsResponse>().await?;

		if news_response.status != "ok" {
			return Err(anyhow::anyhow!(news_response.message.unwrap_or_else(|| {
				"Failed to set the connection for the News Source".to_string()
			})));
		}

		Ok(())
	}

	async fn get_slice(&self, _path: &Path, _range: Range<usize>) -> SourceResult<Vec<u8>> {
		Ok(vec![])
	}

	async fn get_slice_stream(
		&self,
		_path: &Path,
		_range: Range<usize>,
	) -> SourceResult<Box<dyn AsyncRead + Send + Unpin>> {
		Ok(Box::new(string_to_async_read("".to_string())))
	}

	async fn get_all(&self, _path: &Path) -> SourceResult<Vec<u8>> {
		Ok(vec![])
	}

	async fn file_num_bytes(&self, _path: &Path) -> SourceResult<u64> {
		Ok(0)
	}

	async fn copy_to(&self, _path: &Path, _output: &mut dyn SendableAsync) -> SourceResult<()> {
		Ok(())
	}

	async fn poll_data(
		&self,
	) -> SourceResult<Pin<Box<dyn Stream<Item = SourceResult<CollectedBytes>> + Send + 'static>>> {
		let source_id = self.source_id.clone();
		let page_size = self.page_size.unwrap_or(20);
		let mut page = 1;
		let self_cloned = self.clone();
		let stream = stream! {
			loop {
				match self_cloned.fetch_news(page).await {
					Ok(news) => {
						if news.status == "ok" {
							if let Some(articles) = news.articles {
								if articles.is_empty() {
									break;
								}

								for individual_news in articles {
									let file_name = individual_news.title
										.clone()
										.map_or(".news".to_string(), |title| format!("{}.news", title));
									let file_name_path = Some(PathBuf::from(file_name));

									let data_str = serde_json::to_string_pretty(&individual_news)
										.unwrap_or_else(|_| "".to_string());
									let data_len = data_str.len();

									let doc_source = individual_news.source.name.clone();

									let collected_bytes = CollectedBytes::new(
										file_name_path,
										Some(Box::pin(string_to_async_read(data_str))),
										true,
										doc_source,
										Some(data_len),
										source_id.clone(),
										None,
									);
									yield Ok(collected_bytes);
								}

								let total_results = news.total_results.unwrap_or(0);
								let total_pages = (total_results as f64 / page_size as f64).ceil() as u32;
								if page < total_pages {
									page += 1;
								} else {
									break;
								}
							} else {
								break;
							}
						} else {
							yield Err(SourceError::new(
								SourceErrorKind::Io,
								anyhow::anyhow!(
									news
										.message
										.unwrap_or_else(|| "Failed to fetch news".to_string())
								).into(),
							));
							break;
						}
					},
					Err(err) => {
						yield Err(err);
						break;
					}
				}
			}
		};

		Ok(Box::pin(stream))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use dotenv::dotenv;
	use futures::StreamExt;
	use std::{collections::HashSet, env};

	// Negative tests
	#[tokio::test]
	async fn test_invalid_api_key() {
		dotenv().ok();
		let news_config = NewsCollectorConfig {
			api_key: "invalid_key".to_string(),
			query: Some("Tesla".to_string()),
			query_type: 0,
			id: "Some-id".to_string(),
			sources: None,
			from_date: None,
			to_date: None,
			language: None,
			sort_by: None,
			exclude_domains: None,
			search_in: None,
			page_size: Some(100),
			domains: None,
			country: None,
			category: None,
		};

		let news_api_client = NewsApiClient::new(news_config).await.unwrap();
		let result = news_api_client.poll_data().await;

		match result {
			Ok(mut stream) => {
				let mut found_error = false;
				while let Some(item) = stream.next().await {
					if item.is_err() {
						println!("This is the error ----{:?}", item.err());
						found_error = true;
						break;
					}
				}
				assert!(
					found_error,
					"Expected at least one error in the stream with an invalid API key"
				);
			},
			Err(_) => {
				assert!(false, "Expected a stream but encountered an error during stream creation")
			},
		}
	}

	#[tokio::test]
	async fn test_empty_query() {
		dotenv().ok();
		let news_config = NewsCollectorConfig {
			api_key: env::var("NEWS_API_KEY").unwrap_or("".to_string()),
			query: Some("".to_string()),
			query_type: 0,
			id: "Some-id".to_string(),
			sources: None,
			from_date: None,
			to_date: None,
			language: None,
			sort_by: None,
			exclude_domains: None,
			search_in: None,
			page_size: Some(100),
			domains: None,
			country: None,
			category: None,
		};

		let news_api_client = NewsApiClient::new(news_config).await.unwrap();
		let result = news_api_client.poll_data().await;

		match result {
			Ok(mut stream) => {
				let mut found_error = false;
				while let Some(item) = stream.next().await {
					if item.is_err() {
						found_error = true;
						break;
					}
				}
				assert!(
					found_error,
					"Expected at least one error in the stream with an empty query"
				);
			},
			Err(_) => {
				assert!(false, "Expected a stream but encountered an error during stream creation")
			},
		}
	}

	// Special case tests
	#[tokio::test]
	async fn test_query_with_special_characters() {
		dotenv().ok();
		let news_config = NewsCollectorConfig {
			api_key: env::var("NEWS_API_KEY").unwrap_or("".to_string()),
			query: Some("@!#$%^&*".to_string()),
			query_type: 0,
			id: "Some-id".to_string(),
			sources: None,
			from_date: None,
			to_date: None,
			language: None,
			sort_by: None,
			exclude_domains: None,
			search_in: None,
			page_size: Some(100),
			domains: None,
			country: None,
			category: None,
		};

		let news_api_client = NewsApiClient::new(news_config).await.unwrap();
		let result = news_api_client.poll_data().await;

		match result {
			Ok(mut stream) => {
				let mut found_error = false;
				let mut found_data = false;
				while let Some(item) = stream.next().await {
					match item {
						Ok(collected_bytes) => {
							found_data = true;
							println!("Collected bytes: {:?}", collected_bytes.file);
						},
						Err(err) => {
							found_error = true;
							println!("Found an error: {:?}", err);
							break;
						},
					}
				}

				assert!(
                found_error || !found_data,
                "Expected an error or no data for a query with special characters, but got data"
            );
			},
			Err(_) => {
				assert!(false, "Expected a stream but encountered an error during stream creation");
			},
		}
	}

	#[tokio::test]
	async fn test_query_with_future_dates() {
		dotenv().ok();
		let news_config = NewsCollectorConfig {
			api_key: env::var("NEWS_API_KEY").unwrap_or("".to_string()),
			query: Some("Tesla".to_string()),
			query_type: 0,
			id: "Some-id".to_string(),
			from_date: Some("2100-01-01".to_string()),
			to_date: Some("2100-12-31".to_string()),
			sources: None,
			language: None,
			sort_by: None,
			exclude_domains: None,
			search_in: None,
			page_size: Some(100),
			domains: None,
			country: None,
			category: None,
		};

		let news_api_client = NewsApiClient::new(news_config).await.unwrap();
		let result = news_api_client.poll_data().await;

		match result {
			Ok(mut stream) => {
				let mut found_error = false;
				while let Some(item) = stream.next().await {
					if item.is_err() {
						found_error = true;
						println!("Erroe {:?}", item.err());
						break;
					}
				}
				assert!(found_error, "Expected at least one error in the stream with future dates");
			},
			Err(_) => {
				assert!(false, "Expected a stream but encountered an error during stream creation")
			},
		}
	}

	// Happy path test
	#[tokio::test]
	async fn test_successful_news_fetch() {
		dotenv().ok();
		let news_config = NewsCollectorConfig {
			api_key: env::var("NEWS_API_KEY").unwrap_or("".to_string()),
			query: Some("Technology".to_string()),
			query_type: 0,
			id: "Some-id".to_string(),
			sources: None,
			from_date: None,
			to_date: None,
			language: Some("en".to_string()),
			sort_by: Some(0),
			exclude_domains: None,
			search_in: None,
			page_size: Some(100),
			domains: None,
			country: None,
			category: None,
		};

		let news_api_client = NewsApiClient::new(news_config).await.unwrap();
		let result = news_api_client.poll_data().await;

		match result {
			Ok(mut stream) => {
				let mut found_data = false;
				while let Some(item) = stream.next().await {
					if item.is_ok() {
						found_data = true;
						break;
					}
					else if item.is_err() {
						println!("Erroe {:?}", item.err());
						break;
					}
				}
				assert!(found_data, "Expected at least one successful data item in the stream");
			},
			Err(_) => {
				assert!(false, "Expected a stream but encountered an error during stream creation")
			},
		}
	}

	// Multiple files test
	#[tokio::test]
	async fn test_multiple_news_pages() {
		dotenv().ok();
		let news_config = NewsCollectorConfig {
			api_key: env::var("NEWS_API_KEY").unwrap_or("".to_string()),
			query: Some("Technology".to_string()),
			query_type: 0,
			id: "Some-id".to_string(),
			sources: None,
			from_date: None,
			to_date: None,
			language: Some("en".to_string()),
			sort_by: Some(0),
			exclude_domains: None,
			search_in: None,
			page_size: Some(5),
			domains: None,
			country: None,
			category: None,
		};

		let news_api_client = NewsApiClient::new(news_config).await.unwrap();
		let result = news_api_client.poll_data().await;

		match result {
			Ok(mut stream) => {
				let mut count = 0;
				let mut file_names: HashSet<String> = HashSet::new();
				while let Some(item) = stream.next().await {
					if let Ok(collected_bytes) = item {
						if let Some(file_path) = collected_bytes.file.clone() {
							if let Some(path_str) = file_path.to_str() {
								file_names.insert(path_str.to_string());
							}
						}
						count += 1;
					}
				}

				assert!(count > 0, "Expected multiple pages but got none");
				assert!(!file_names.is_empty(), "Expected file names but found none");
			},
			Err(e) => {
				println!("Error as {:?}", e.source);
				assert!(false, "Expected a stream but encountered an error during stream creation")
			},
		}
	}
}
