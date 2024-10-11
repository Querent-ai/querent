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
		let to_date = Self::string_to_datetime(config.to_date.clone());
		let from_date = Self::string_to_datetime(config.from_date.clone());
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
			format!("https://newsapi.org/v2/{}?apiKey={}", self.query_type, self.api_token);

			if let Some(query) = &self.query {
				if !query.is_empty() {
					url.push_str(&format!("&q={}", query));
				}
			}			
		url.push_str(&format!("&page={}", page));
		if let Some(page_size) = self.page_size {
			url.push_str(&format!("&pageSize={}", page_size));
		}
		if self.query_type == "everything" {
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
			}

			if let Some(to) = &self.to {
				url.push_str(&format!("&to={}", to.format("%Y-%m-%dT%H:%M:%SZ")));
			}

			if let Some(sort_by) = &self.sort_by {
				url.push_str(&format!("&sortBy={}", sort_by));
			}
		}
		if self.query_type == "top-headlines" {
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

	async fn copy_to(&self, _path: &Path, output: &mut dyn SendableAsync) -> SourceResult<()> {
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

