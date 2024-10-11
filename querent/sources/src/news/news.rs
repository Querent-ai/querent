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
use tokio::io::{AsyncRead, BufReader};

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
	sources: Option<String>,
	domains: Option<String>,
	exclude_domains: Option<String>,
	search_in: Option<String>,
}

impl NewsApiClient {
	pub async fn new(config: NewsCollectorConfig) -> anyhow::Result<Self> {
		let to_date = Self::string_to_datetime(config.to_date.clone());
		let from_date = Self::string_to_datetime(config.from_date.clone());
		Ok(NewsApiClient {
			client: reqwest::Client::new(),
			api_token: config.api_key.to_string(),
			query_type: config.query_type().as_str_name().to_string(),
			query: config.query.to_string(),
			sources: config.sources.clone(),
			source_id: config.id.to_string(), //Internal collector id
			sort_by: Some(config.sort_by().as_str_name().to_string()),
			domains: config.domains,
			to: to_date,
			from: from_date,
			language: config.language.clone(),
			page_size: config.page_size.map(|ps| ps as u32),
			exclude_domains: config.exclude_domains.clone(),
			search_in: config.search_in.clone(),
		})
	}

	pub async fn fetch_news(&self, page: u32) -> Result<NewsResponse, SourceError> {
		let url = self.create_query(page).await;

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
		let mut url =
			format!("https://newsapi.org/v2/{}?&apiKey={}", self.query_type, self.api_token);
		if !self.query.is_empty() {
			url.push_str(&format!("&q={}", self.query));
		}

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

		url.push_str(&format!("&page={}", page));

		if let Some(from) = &self.from {
			url.push_str(&format!("&from={}", from.format("%Y-%m-%dT%H:%M:%SZ")));
		}

		if let Some(to) = &self.to {
			url.push_str(&format!("&to={}", to.format("%Y-%m-%dT%H:%M:%SZ")));
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
		if let Some(exclude_domains) = &self.exclude_domains {
			if !exclude_domains.is_empty() {
				url.push_str(&format!("&excludeDomains={}", exclude_domains));
			}
		}

		if let Some(search_in) = &self.search_in {
			if !search_in.is_empty() {
				url.push_str(&format!("&searchIn={}", search_in));
			}
		}

		url
	}

	fn string_to_datetime(s: Option<String>) -> Option<DateTime<Utc>> {
		s.and_then(|date_string| {
			DateTime::parse_from_rfc3339(&date_string)
				.map(|dt| dt.with_timezone(&Utc))
				.or_else(|_| {
					Err(NaiveDate::parse_from_str(&date_string, "%Y-%m-%d").ok().map(|nd| {
						DateTime::<Utc>::from_naive_utc_and_offset(
							nd.and_hms_opt(0, 0, 0).unwrap(),
							Utc,
						)
					}))
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
		let news = self.fetch_news(1).await.map_err(|err| {
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
		let news = self.fetch_news(1).await.map_err(|err| {
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
		let news = self.fetch_news(1).await.map_err(|err| {
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
		let news = self.fetch_news(1).await.map_err(|err| {
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
		let news = self.fetch_news(1).await.map_err(|err| {
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
		let page_size = self.page_size.unwrap_or(20); //Need to ask gpt
		let mut page = 1;
		// let news = self.fetch_news().await.map_err(|err| {
		// 	SourceError::new(
		// 		SourceErrorKind::Io,
		// 		anyhow::anyhow!("Error while fetching news: {:?}", err).into(),
		// 	)
		// })?;
		let stream = stream! {
			loop {
				match self.fetch_news(page).await {
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
			exclude_domains: None,
			search_in: None,
			page_size: Some(10),
			domains: None,
		};

		let news_api_client = NewsApiClient::new(news_config).await.unwrap();

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
						Ok(collected_bytes) => {
							if let Some(pathbuf) = collected_bytes.file {
								if let Some(str_path) = pathbuf.to_str() {
									count_files.insert(str_path.to_string());
								}
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
