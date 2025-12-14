use crate::error::{Result, ScanError};
use crate::result::CrawlResult;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use url::Url;

pub type ProgressCallback = Arc<dyn Fn(usize, String) + Send + Sync>;
pub type CrossDomainCallback = Arc<dyn Fn(String, String) -> bool + Send + Sync>;

pub struct Crawler {
    client: Client,
    visited: Arc<Mutex<HashSet<String>>>,
    results: Arc<Mutex<Vec<CrawlResult>>>,
    max_depth: usize,
    base_domain: Option<String>,
    progress_callback: Option<ProgressCallback>,
    cross_domain_callback: Option<CrossDomainCallback>,
    auto_follow: bool,
}

impl Crawler {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Rinzler/0.1 (https://github.com/trapdoorsec/rinzler)")
            .timeout(std::time::Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            visited: Arc::new(Mutex::new(HashSet::new())),
            results: Arc::new(Mutex::new(Vec::new())),
            max_depth: 3,
            base_domain: None,
            progress_callback: None,
            cross_domain_callback: None,
            auto_follow: false,
        }
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    pub fn with_base_domain(mut self, domain: String) -> Self {
        self.base_domain = Some(domain);
        self
    }

    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    pub fn with_cross_domain_callback(mut self, callback: CrossDomainCallback) -> Self {
        self.cross_domain_callback = Some(callback);
        self
    }

    pub fn with_auto_follow(mut self, auto_follow: bool) -> Self {
        self.auto_follow = auto_follow;
        self
    }

    pub async fn crawl(&self, start_url: &str, workers: usize) -> Result<Vec<CrawlResult>> {
        info!("Starting crawl of {} with {} workers", start_url, workers);

        let parsed_url = Url::parse(start_url)
            .map_err(|e| ScanError::InvalidUrl(format!("Invalid URL: {}", e)))?;

        let base_domain = self
            .base_domain
            .clone()
            .unwrap_or_else(|| parsed_url.host_str().unwrap_or("unknown").to_string());

        // Mark initial URL as visited
        {
            let mut visited = self.visited.lock().await;
            visited.insert(start_url.to_string());
        }

        // Crawl the initial URL
        let mut to_crawl = vec![start_url.to_string()];
        let mut depth = 0;

        while depth < self.max_depth && !to_crawl.is_empty() {
            info!(
                "Crawling depth {}/{}, {} URLs to process",
                depth + 1,
                self.max_depth,
                to_crawl.len()
            );

            // Process URLs in parallel using stream
            let urls_to_process: Vec<_> = std::mem::take(&mut to_crawl);

            let results: Vec<_> = stream::iter(urls_to_process.into_iter().enumerate())
                .map(|(worker_id, url)| {
                    let client = self.client.clone();
                    let base_domain = base_domain.clone();
                    let progress_cb = self.progress_callback.clone();

                    async move {
                        // Report progress
                        if let Some(ref callback) = progress_cb {
                            callback(worker_id, url.clone());
                        }

                        self.fetch_and_parse(&client, &url, &base_domain).await
                    }
                })
                .buffer_unordered(workers)
                .collect()
                .await;

            // Process results and collect new URLs
            for result in results {
                match result {
                    Ok((crawl_result, new_urls)) => {
                        // Store the result
                        {
                            let mut results = self.results.lock().await;
                            results.push(crawl_result);
                        }

                        // Add new URLs to the queue if they haven't been visited
                        for new_url in new_urls {
                            let mut visited = self.visited.lock().await;
                            if !visited.contains(&new_url) {
                                visited.insert(new_url.clone());
                                to_crawl.push(new_url);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Crawl error: {}", e);
                    }
                }
            }

            depth += 1;
        }

        let results = self.results.lock().await;
        info!("Crawl complete. Visited {} pages", results.len());
        Ok(results.clone())
    }

    async fn fetch_and_parse(
        &self,
        client: &Client,
        url: &str,
        base_domain: &str,
    ) -> Result<(CrawlResult, Vec<String>)> {
        debug!("Fetching {}", url);

        let start = Instant::now();
        let response = client.get(url).send().await?;
        let response_time = start.elapsed();

        let status_code = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let content_length = response.content_length();

        let body = response.text().await?;

        let mut result = CrawlResult::new(url.to_string());
        result.status_code = status_code;
        result.content_type = content_type.clone();
        result.content_length = content_length;
        result.response_time = response_time;

        // Only parse HTML content
        let is_html = content_type
            .as_ref()
            .map(|ct| ct.contains("text/html"))
            .unwrap_or(false);

        let mut new_urls = Vec::new();

        if is_html {
            let (links, forms, scripts) = self.extract_elements(&body, url, base_domain)?;
            result.links_found = links.clone();
            result.forms_found = forms;
            result.scripts_found = scripts;
            new_urls = links;
        }

        Ok((result, new_urls))
    }

    fn extract_elements(
        &self,
        html: &str,
        current_url: &str,
        base_domain: &str,
    ) -> Result<(Vec<String>, usize, usize)> {
        let document = Html::parse_document(html);

        // Extract links
        let link_selector = Selector::parse("a[href]").unwrap();
        let mut links = Vec::new();

        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href")
                && let Some(absolute_url) = self.resolve_url(current_url, href) {
                    if self.is_same_domain(&absolute_url, base_domain) {
                        links.push(absolute_url);
                    } else if !self.auto_follow {
                        // Cross-domain link found and auto_follow is false
                        if let Some(ref callback) = self.cross_domain_callback
                            && callback(absolute_url.clone(), base_domain.to_string()) {
                                links.push(absolute_url);
                            }
                    }
                }
        }

        // Count forms
        let form_selector = Selector::parse("form").unwrap();
        let forms_count = document.select(&form_selector).count();

        // Count scripts
        let script_selector = Selector::parse("script[src]").unwrap();
        let scripts_count = document.select(&script_selector).count();

        Ok((links, forms_count, scripts_count))
    }

    fn resolve_url(&self, base: &str, href: &str) -> Option<String> {
        // Skip empty, javascript:, mailto:, tel:, etc.
        if href.is_empty()
            || href.starts_with("javascript:")
            || href.starts_with("mailto:")
            || href.starts_with("tel:")
            || href.starts_with('#')
        {
            return None;
        }

        let base_url = Url::parse(base).ok()?;
        let resolved = base_url.join(href).ok()?;

        // Remove fragment
        let mut url = resolved.clone();
        url.set_fragment(None);

        Some(url.to_string())
    }

    fn is_same_domain(&self, url: &str, base_domain: &str) -> bool {
        if let Ok(parsed) = Url::parse(url)
            && let Some(host) = parsed.host_str() {
                return host == base_domain || host.ends_with(&format!(".{}", base_domain));
            }
        false
    }

    pub async fn get_results(&self) -> Vec<CrawlResult> {
        self.results.lock().await.clone()
    }

    pub async fn get_visited_count(&self) -> usize {
        self.visited.lock().await.len()
    }
}

impl Default for Crawler {
    fn default() -> Self {
        Self::new()
    }
}
