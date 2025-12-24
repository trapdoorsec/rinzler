use crate::error::{Result, ScanError};
use crate::result::CrawlResult;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
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
    #[allow(dead_code)]
    timeout_secs: u64,
}

impl Crawler {
    pub fn new() -> Self {
        Self::with_timeout(10)
    }

    pub fn with_timeout(timeout_secs: u64) -> Self {
        let client = Client::builder()
            .user_agent("Rinzler/0.1 (https://github.com/trapdoorsec/rinzler)")
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .connect_timeout(std::time::Duration::from_secs(timeout_secs / 2))
            .pool_max_idle_per_host(50) // Connection pooling
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .http2_adaptive_window(true) // Enable HTTP/2 with adaptive flow control
            .tcp_keepalive(std::time::Duration::from_secs(60))
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
            timeout_secs,
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

        // Create worker-owned queues with work stealing
        // Each worker has its own queue: VecDeque<(url, depth)>
        let worker_queues: Arc<Vec<Mutex<VecDeque<(String, usize)>>>> =
            Arc::new((0..workers).map(|_| Mutex::new(VecDeque::new())).collect());

        // Initialize worker 0's queue with the starting URL
        {
            let mut queue = worker_queues[0].lock().await;
            queue.push_back((start_url.to_string(), 0));
        }

        // Spawn worker tasks
        let mut worker_handles = Vec::new();

        for worker_id in 0..workers {
            let client = self.client.clone();
            let base_domain = base_domain.clone();
            let progress_cb = self.progress_callback.clone();
            let cross_domain_cb = self.cross_domain_callback.clone();
            let auto_follow = self.auto_follow;
            let max_depth = self.max_depth;
            let visited = self.visited.clone();
            let results = self.results.clone();
            let worker_queues_clone = worker_queues.clone();

            let handle = tokio::spawn(async move {
                debug!("Worker {} started", worker_id);
                let mut empty_iterations = 0;
                const MAX_EMPTY_ITERATIONS: usize = 10;  // Retry 10 times before giving up

                loop {
                    // Get work from own queue (no stealing in crawl mode)
                    let work_item = {
                        let mut queue = worker_queues_clone[worker_id].lock().await;
                        queue.pop_front()
                    };

                    let (url, depth) = if let Some(item) = work_item {
                        // Reset empty counter since we found work
                        empty_iterations = 0;
                        item
                    } else {
                        // Own queue is empty - check if all workers are done
                        if Self::all_queues_empty(&worker_queues_clone).await {
                            empty_iterations += 1;
                            debug!("Worker {} found all queues empty ({}/{})", worker_id, empty_iterations, MAX_EMPTY_ITERATIONS);
                            if empty_iterations >= MAX_EMPTY_ITERATIONS {
                                debug!("Worker {} exiting", worker_id);
                                break;
                            }
                        } else {
                            empty_iterations = 0;  // Reset counter
                        }

                        // Sleep and retry
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                        continue;
                    };

                    // Check depth limit
                    if depth >= max_depth {
                        continue;
                    }

                    // Report progress
                    if let Some(ref callback) = progress_cb {
                        callback(worker_id, url.clone());
                    }

                    // Fetch and parse the URL
                    match Self::fetch_and_parse_static(
                        &client,
                        &url,
                        &base_domain,
                        &cross_domain_cb,
                        auto_follow,
                    )
                    .await
                    {
                        Ok((crawl_result, new_urls)) => {
                            // Store the result
                            {
                                let mut results_lock = results.lock().await;
                                results_lock.push(crawl_result);
                            }

                            // Distribute new URLs across ALL worker queues (round-robin)
                            let num_workers = worker_queues_clone.len();
                            let num_new_urls = new_urls.len();
                            debug!("[Worker {}] Distributing {} URLs across {} workers", worker_id, num_new_urls, num_workers);
                            let mut target_worker = 0;
                            for new_url in new_urls {
                                // Check and mark as visited
                                let should_queue = {
                                    let mut visited_lock = visited.lock().await;
                                    if !visited_lock.contains(&new_url) {
                                        visited_lock.insert(new_url.clone());
                                        true
                                    } else {
                                        false
                                    }
                                };

                                if should_queue {
                                    // Add to target worker's queue
                                    debug!("[Worker {}] Queuing {} to worker {}", worker_id, new_url, target_worker);
                                    let mut queue = worker_queues_clone[target_worker].lock().await;
                                    queue.push_back((new_url.clone(), depth + 1));
                                    drop(queue); // Release lock immediately

                                    // Round-robin to next worker
                                    target_worker = (target_worker + 1) % worker_queues_clone.len();
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Crawl error for {}: {}", url, e);
                        }
                    }
                }

                debug!("Worker {} finished", worker_id);
            });

            worker_handles.push(handle);
        }

        // Wait for all workers to complete
        for handle in worker_handles {
            handle
                .await
                .map_err(|e| ScanError::Other(format!("Worker task failed: {}", e)))?;
        }

        let results = self.results.lock().await;
        info!("Crawl complete. Visited {} pages", results.len());
        Ok(results.clone())
    }


    /// Check if all worker queues are empty
    async fn all_queues_empty(worker_queues: &Arc<Vec<Mutex<VecDeque<(String, usize)>>>>) -> bool {
        for queue in worker_queues.iter() {
            if !queue.lock().await.is_empty() {
                return false;
            }
        }
        true
    }

    /// Static version of fetch_and_parse for use in spawned tasks
    async fn fetch_and_parse_static(
        client: &Client,
        url: &str,
        base_domain: &str,
        cross_domain_callback: &Option<CrossDomainCallback>,
        auto_follow: bool,
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
            let (links, forms, scripts) = Self::extract_elements_static(
                &body,
                url,
                base_domain,
                cross_domain_callback,
                auto_follow,
            )?;
            result.links_found = links.clone();
            result.forms_found = forms;
            result.scripts_found = scripts;
            new_urls = links;
        }

        Ok((result, new_urls))
    }

    /// Static version of extract_elements for use in spawned tasks
    fn extract_elements_static(
        html: &str,
        current_url: &str,
        base_domain: &str,
        cross_domain_callback: &Option<CrossDomainCallback>,
        auto_follow: bool,
    ) -> Result<(Vec<String>, usize, usize)> {
        let document = Html::parse_document(html);

        // Extract links
        let link_selector = Selector::parse("a[href]").unwrap();
        let mut links = Vec::new();

        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href")
                && let Some(absolute_url) = Self::resolve_url_static(current_url, href)
            {
                debug!("Found link: {} (base_domain: {})", absolute_url, base_domain);
                if Self::is_same_domain_static(&absolute_url, base_domain) {
                    debug!("  -> Same domain, adding to queue");
                    links.push(absolute_url);
                } else if auto_follow {
                    // Cross-domain link and auto_follow is enabled
                    debug!("  -> Cross-domain but auto_follow enabled, adding to queue");
                    links.push(absolute_url);
                } else if !auto_follow {
                    // Cross-domain link found and auto_follow is false
                    debug!("  -> Cross-domain, checking callback");
                    if let Some(callback) = cross_domain_callback
                        && callback(absolute_url.clone(), base_domain.to_string())
                    {
                        debug!("  -> Callback approved, adding to queue");
                        links.push(absolute_url);
                    } else {
                        debug!("  -> No callback or declined, skipping");
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

    fn resolve_url_static(base: &str, href: &str) -> Option<String> {
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

    fn is_same_domain_static(url: &str, base_domain: &str) -> bool {
        if let Ok(parsed) = Url::parse(url)
            && let Some(host) = parsed.host_str()
        {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::sync::Mutex as TokioMutex;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    /// Test basic link discovery
    #[tokio::test]
    async fn test_link_discovery() {
        let mock_server = MockServer::start().await;

        let root_html = format!(
            r#"<html><body>
                <a href="{}/page1">Page 1</a>
                <a href="{}/page2">Page 2</a>
            </body></html>"#,
            mock_server.uri(),
            mock_server.uri()
        );

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/html")
                    .set_body_bytes(root_html.as_bytes()),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/page1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/html")
                    .set_body_bytes(b"<html><body>P1</body></html>"),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/page2"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/html")
                    .set_body_bytes(b"<html><body>P2</body></html>"),
            )
            .mount(&mock_server)
            .await;

        let crawler = Crawler::new().with_max_depth(2);

        let results = crawler.crawl(&mock_server.uri(), 1).await.unwrap();

        println!("\n=== Link Discovery Test ===");
        println!("Total pages crawled: {}", results.len());
        println!("URLs crawled:");
        for result in &results {
            println!("  - {} (status: {})", result.url, result.status_code);
            println!("    Content-Type: {:?}", result.content_type);
            println!("    Links found: {:?}", result.links_found);
        }
        println!("Visited count: {}", crawler.get_visited_count().await);

        // Should have crawled root + 2 pages = 3 total
        assert!(
            results.len() >= 3,
            "Expected at least 3 pages crawled (root + 2 links), but got {}",
            results.len()
        );
    }

    /// Test that multiple workers are actually used during crawling
    #[tokio::test]
    async fn test_multiple_workers_are_used() {
        // Track which workers process URLs
        let worker_activity: Arc<TokioMutex<HashMap<usize, Vec<String>>>> =
            Arc::new(TokioMutex::new(HashMap::new()));
        let worker_activity_clone = worker_activity.clone();

        // Set up mock server with pages that link to each other
        let mock_server = MockServer::start().await;

        // Root page with 10 links
        let mut root_html = String::from("<html><body>");
        for i in 1..=10 {
            root_html.push_str(&format!(
                r#"<a href="{}/page{}">Page {}</a>"#,
                mock_server.uri(),
                i,
                i
            ));
        }
        root_html.push_str("</body></html>");

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/html")
                    .set_body_bytes(root_html.as_bytes()),
            )
            .mount(&mock_server)
            .await;

        // Individual pages with no links (to avoid infinite crawling)
        for i in 1..=10 {
            Mock::given(method("GET"))
                .and(path(format!("/page{}", i)))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/html")
                        .set_body_bytes(b"<html><body>Page</body></html>"),
                )
                .mount(&mock_server)
                .await;
        }

        // Create crawler with progress callback to track worker activity
        let crawler = Crawler::new()
            .with_max_depth(2)
            .with_progress_callback(Arc::new(move |worker_id, url| {
                let worker_activity = worker_activity_clone.clone();
                tokio::spawn(async move {
                    let mut activity = worker_activity.lock().await;
                    activity.entry(worker_id).or_insert_with(Vec::new).push(url);
                });
            }));

        // Crawl with 4 workers
        let num_workers = 4;
        let results = crawler.crawl(&mock_server.uri(), num_workers).await.unwrap();

        // Give progress callbacks time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify we got results
        assert!(!results.is_empty(), "Should have crawled some pages");

        // Check worker activity
        let activity = worker_activity.lock().await;
        let workers_used = activity.keys().count();

        println!("Worker activity distribution:");
        for (worker_id, urls) in activity.iter() {
            println!("  Worker {}: {} URLs", worker_id, urls.len());
        }

        // Assert that more than one worker was used
        assert!(
            workers_used > 1,
            "Expected multiple workers to be used, but only {} worker(s) processed URLs. Distribution: {:?}",
            workers_used,
            activity.iter().map(|(k, v)| (k, v.len())).collect::<Vec<_>>()
        );

        // Ideally, all workers should have done some work
        // (though this might not always be true for small workloads)
        println!("Total workers used: {} out of {}", workers_used, num_workers);
    }

    /// Test that URLs are distributed via round-robin
    #[tokio::test]
    async fn test_work_distribution_round_robin() {
        // Track which worker processes which URL
        let worker_urls: Arc<TokioMutex<HashMap<usize, Vec<String>>>> =
            Arc::new(TokioMutex::new(HashMap::new()));
        let worker_urls_clone = worker_urls.clone();

        let mock_server = MockServer::start().await;

        // Root page with exactly 12 links (divisible by 3 workers)
        let mut root_html = String::from("<html><body>");
        for i in 1..=12 {
            root_html.push_str(&format!(
                r#"<a href="{}/page{}">Page {}</a>"#,
                mock_server.uri(),
                i,
                i
            ));
        }
        root_html.push_str("</body></html>");

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/html")
                    .set_body_bytes(root_html.as_bytes()),
            )
            .mount(&mock_server)
            .await;

        // Individual pages
        for i in 1..=12 {
            Mock::given(method("GET"))
                .and(path(format!("/page{}", i)))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/html")
                        .set_body_bytes(b"<html><body>Content</body></html>"),
                )
                .mount(&mock_server)
                .await;
        }

        let crawler = Crawler::new()
            .with_max_depth(2)
            .with_progress_callback(Arc::new(move |worker_id, url| {
                let worker_urls = worker_urls_clone.clone();
                tokio::spawn(async move {
                    let mut urls = worker_urls.lock().await;
                    urls.entry(worker_id).or_insert_with(Vec::new).push(url);
                });
            }));

        let num_workers = 3;
        crawler.crawl(&mock_server.uri(), num_workers).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let urls = worker_urls.lock().await;

        println!("\nRound-robin distribution test:");
        for (worker_id, worker_urls) in urls.iter() {
            println!("  Worker {}: {} URLs", worker_id, worker_urls.len());
        }

        // Each worker should have processed some URLs
        for worker_id in 0..num_workers {
            let count = urls.get(&worker_id).map(|v| v.len()).unwrap_or(0);
            assert!(
                count > 0,
                "Worker {} did not process any URLs. Distribution: {:?}",
                worker_id,
                urls.iter().map(|(k, v)| (k, v.len())).collect::<Vec<_>>()
            );
        }
    }

    /// Test work stealing mechanism
    #[tokio::test]
    async fn test_work_stealing() {
        let mock_server = MockServer::start().await;

        // Create a page with many links to ensure work stealing can occur
        let mut root_html = String::from("<html><body>");
        for i in 1..=20 {
            root_html.push_str(&format!(
                r#"<a href="{}/page{}">Page {}</a>"#,
                mock_server.uri(),
                i,
                i
            ));
        }
        root_html.push_str("</body></html>");

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/html")
                    .set_body_bytes(root_html.as_bytes()),
            )
            .mount(&mock_server)
            .await;

        for i in 1..=20 {
            // Add delay to simulate work and increase chance of stealing
            let html = format!(
                "<html><body><a href='{}/subpage{}'>Subpage</a></body></html>",
                mock_server.uri(),
                i
            );
            Mock::given(method("GET"))
                .and(path(format!("/page{}", i)))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/html")
                        .set_body_bytes(html.as_bytes())
                        .set_delay(tokio::time::Duration::from_millis(10)),
                )
                .mount(&mock_server)
                .await;

            Mock::given(method("GET"))
                .and(path(format!("/subpage{}", i)))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/html")
                        .set_body_bytes(b"<html><body>End</body></html>"),
                )
                .mount(&mock_server)
                .await;
        }

        // Track when workers are idle (would trigger work stealing)
        let worker_idle_count: Arc<TokioMutex<HashMap<usize, usize>>> =
            Arc::new(TokioMutex::new(HashMap::new()));
        let worker_idle_clone = worker_idle_count.clone();

        let crawler = Crawler::new()
            .with_max_depth(3)
            .with_progress_callback(Arc::new(move |worker_id, _url| {
                let idle_count = worker_idle_clone.clone();
                tokio::spawn(async move {
                    let mut counts = idle_count.lock().await;
                    *counts.entry(worker_id).or_insert(0) += 1;
                });
            }));

        let num_workers = 5;
        let results = crawler.crawl(&mock_server.uri(), num_workers).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        println!("\nWork stealing test:");
        println!("Total pages crawled: {}", results.len());

        let idle_counts = worker_idle_count.lock().await;
        println!("Worker activity:");
        for (worker_id, count) in idle_counts.iter() {
            println!("  Worker {}: {} URLs processed", worker_id, count);
        }

        // Verify multiple workers were active (indication that work was distributed/stolen)
        let active_workers = idle_counts.keys().count();
        assert!(
            active_workers > 1,
            "Expected multiple workers to be active (work stealing), but only {} were active",
            active_workers
        );
    }
}
