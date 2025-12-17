// Fuzzing module for forced browsing / directory enumeration

use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use url::Url;

/// Result of a fuzz attempt
#[derive(Debug, Clone)]
pub struct FuzzResult {
    pub url: String,
    pub status_code: u16,
    pub content_length: Option<u64>,
    pub content_type: Option<String>,
    pub source: FuzzSource,
}

/// Source of the fuzz target
#[derive(Debug, Clone, PartialEq)]
pub enum FuzzSource {
    Initial,    // From command line
    Database,   // From previous crawl
    Discovered, // Found during fuzzing
}

/// Options for configuring a fuzz operation
pub struct FuzzOptions {
    pub base_urls: Vec<String>,
    pub wordlist: Vec<String>,
    pub threads: usize,
    pub show_progress_bars: bool,
    pub use_head_requests: bool,
    pub timeout_secs: u64,
    pub db_path: Option<std::path::PathBuf>,
}

/// Execute fuzzing with given options
pub async fn execute_fuzz(options: FuzzOptions) -> Result<Vec<FuzzResult>, String> {
    let FuzzOptions {
        base_urls,
        wordlist,
        threads,
        show_progress_bars,
        use_head_requests,
        timeout_secs,
        db_path,
    } = options;

    if base_urls.is_empty() {
        return Err("No base URLs provided".to_string());
    }

    if wordlist.is_empty() {
        return Err("Wordlist is empty".to_string());
    }

    // Query database for known endpoints from previous crawls
    let mut db_endpoints = Vec::new();
    if let Some(ref db_path) = db_path {
        if let Ok(db_urls) = query_database_endpoints(db_path, &base_urls) {
            db_endpoints = db_urls;
            if !db_endpoints.is_empty() {
                println!(
                    "✓ Found {} endpoints from previous crawls in database",
                    db_endpoints.len()
                );
            }
        }
    }

    // Build initial base URLs with sources
    // Note: We add database URLs first, then command-line URLs
    // Since workers use pop() which takes from the end, this ensures
    // root routes (command-line) are tested BEFORE database endpoints
    let mut base_urls_with_source = Vec::new();

    // Add database URLs first (will be tested last)
    for url in &db_endpoints {
        base_urls_with_source.push((url.clone(), FuzzSource::Database));
    }

    // Add command-line URLs last (will be tested first due to pop())
    for url in &base_urls {
        base_urls_with_source.push((url.clone(), FuzzSource::Initial));
    }

    // Build full URLs to test
    let mut urls_to_test = Vec::new();
    for (base_url, source) in &base_urls_with_source {
        for word in &wordlist {
            let test_url = build_test_url(base_url, word)?;
            urls_to_test.push((test_url, source.clone()));
        }
    }

    let initial_count = urls_to_test.len();
    println!(
        "Testing {} initial URLs with {} workers",
        initial_count, threads
    );
    if !db_endpoints.is_empty() {
        println!(
            "  {} from command line, {} from database",
            base_urls.len() * wordlist.len(),
            db_endpoints.len() * wordlist.len()
        );
    }
    println!();

    // Set up multi-progress for worker tracking
    let m = if show_progress_bars {
        Some(Arc::new(MultiProgress::new()))
    } else {
        None
    };

    // Create shared results vector and hits display
    let results: Arc<Mutex<Vec<FuzzResult>>> = Arc::new(Mutex::new(Vec::new()));
    let hits_display: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // Create worker-owned queues with work stealing
    // Each worker has its own queue: VecDeque<(url, source)>
    let worker_queues: Arc<Vec<Mutex<VecDeque<(String, FuzzSource)>>>> =
        Arc::new((0..threads).map(|_| Mutex::new(VecDeque::new())).collect());

    // Distribute initial URLs evenly across workers
    for (idx, (url, source)) in urls_to_test.into_iter().enumerate() {
        let worker_id = idx % threads;
        worker_queues[worker_id]
            .try_lock()
            .unwrap()
            .push_back((url, source));
    }

    let tested_urls: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let wordlist_arc = Arc::new(wordlist);

    // Create hits display progress bar (sticky at top)
    let hits_pb = if show_progress_bars && let Some(ref multi_progress) = m {
        let pb = multi_progress.add(ProgressBar::new(0));
        pb.set_style(ProgressStyle::default_bar().template("{msg}").unwrap());
        pb.set_message("Hits: 0".to_string());
        Some(Arc::new(pb))
    } else {
        None
    };

    // Create optimized HTTP client with HTTP/2 and connection pooling
    let client = Arc::new(
        Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .connect_timeout(Duration::from_secs(timeout_secs / 2))
            .pool_max_idle_per_host(threads) // Connection pooling
            .pool_idle_timeout(Duration::from_secs(90))
            .http2_adaptive_window(true) // Enable HTTP/2 with adaptive flow control
            .tcp_keepalive(Duration::from_secs(60))
            .redirect(reqwest::redirect::Policy::limited(3))
            .user_agent("Rinzler/0.1 (https://github.com/trapdoorsec/rinzler)")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?,
    );

    // Spawn workers with work stealing
    let mut worker_tasks = Vec::new();

    for worker_id in 0..threads {
        // Create progress bar for this worker
        let pb = if show_progress_bars && let Some(ref multi_progress) = m {
            let progress_bar = multi_progress.add(ProgressBar::new(initial_count as u64));
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template("[{bar:40.cyan/blue}] Worker {msg}")
                    .unwrap()
                    .progress_chars("=>-"),
            );
            progress_bar.set_message(format!("{} idle", worker_id));
            Some(progress_bar)
        } else {
            None
        };

        let client_clone = client.clone();
        let results_clone = results.clone();
        let hits_display_clone = hits_display.clone();
        let hits_pb_clone = hits_pb.clone();
        let worker_queues_clone = worker_queues.clone();
        let tested_urls_clone = tested_urls.clone();
        let wordlist_clone = wordlist_arc.clone();

        let task = tokio::spawn(async move {
            let mut processed = 0;

            loop {
                // Try to get work from own queue
                let work_item = {
                    let mut queue = worker_queues_clone[worker_id].lock().await;
                    queue.pop_front()
                };

                let (url, source) = if let Some(item) = work_item {
                    item
                } else {
                    // Own queue is empty - try to steal from other workers
                    let stolen = try_steal_fuzz_work(worker_id, &worker_queues_clone).await;
                    if let Some(item) = stolen {
                        item
                    } else {
                        // No work available anywhere - check if all queues are truly empty
                        if all_fuzz_queues_empty(&worker_queues_clone).await {
                            break; // All done
                        }
                        // Queues might have new work, try again
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        continue;
                    }
                };

                // Extract path for display
                let url_path = extract_path(&url);

                // Update progress bar with current URL in orange
                if let Some(ref pb) = pb {
                    processed += 1;
                    let msg = format!(
                        "{} {} {}",
                        worker_id,
                        processed,
                        format!("[{}]", url_path).truecolor(255, 165, 0) // Orange
                    );
                    pb.set_message(msg);
                }

                // Make request
                if let Ok(mut result) =
                    make_fuzz_request(&client_clone, &url, use_head_requests).await
                {
                    result.source = source.clone();

                    // Save all responses < 500 to results for final report
                    if result.status_code < 500 {
                        results_clone.lock().await.push(result.clone());
                    }

                    // If we found a new endpoint (200-399), add it to this worker's queue
                    if (200..400).contains(&result.status_code) {
                        // Display the hit
                        let hit_display = format_hit(&result);
                        hits_display_clone.lock().await.push(hit_display);

                        // Update hits display area
                        if let Some(ref hits_pb) = hits_pb_clone {
                            let hits = hits_display_clone.lock().await;
                            let formatted = format_hits_display(&hits);
                            hits_pb.set_message(formatted);
                        }

                        // Extract base path for this discovered endpoint
                        if let Ok(base_url) = extract_base_url(&result.url) {
                            let mut tested = tested_urls_clone.lock().await;

                            // Only add if we haven't tested this base yet
                            if !tested.contains(&base_url) {
                                tested.insert(base_url.clone());

                                // Generate new fuzz targets and add to this worker's queue (route affinity)
                                let mut queue = worker_queues_clone[worker_id].lock().await;
                                for word in wordlist_clone.iter() {
                                    if let Ok(new_url) = build_test_url(&base_url, word) {
                                        queue.push_back((new_url, FuzzSource::Discovered));
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(ref pb) = pb {
                    pb.inc(1);
                }
            }

            if let Some(ref pb) = pb {
                pb.finish_with_message(format!("{}: done", worker_id));
            }
        });

        worker_tasks.push(task);
    }

    // Wait for all workers to complete
    for task in worker_tasks {
        task.await
            .map_err(|e| format!("Worker task failed: {}", e))?;
    }

    // Finalize hits display
    if let Some(ref hits_pb) = hits_pb {
        hits_pb.finish();
    }

    // Extract results
    let final_results = results.lock().await.clone();

    Ok(final_results)
}

/// Try to steal work from other workers
async fn try_steal_fuzz_work(
    worker_id: usize,
    worker_queues: &Arc<Vec<Mutex<VecDeque<(String, FuzzSource)>>>>,
) -> Option<(String, FuzzSource)> {
    // Try to steal from each other worker
    for target_id in 0..worker_queues.len() {
        if target_id == worker_id {
            continue; // Don't steal from self
        }

        let mut target_queue = worker_queues[target_id].lock().await;
        if let Some(item) = target_queue.pop_back() {
            return Some(item);
        }
    }

    None
}

/// Check if all worker queues are empty
async fn all_fuzz_queues_empty(
    worker_queues: &Arc<Vec<Mutex<VecDeque<(String, FuzzSource)>>>>,
) -> bool {
    for queue in worker_queues.iter() {
        if !queue.lock().await.is_empty() {
            return false;
        }
    }
    true
}

/// Extract path component from URL for display
fn extract_path(url: &str) -> String {
    if let Ok(parsed) = Url::parse(url) {
        let path = parsed.path();
        if path.len() > 40 {
            format!("...{}", &path[path.len() - 37..])
        } else {
            path.to_string()
        }
    } else {
        url.to_string()
    }
}

/// Format a single hit for display
fn format_hit(result: &FuzzResult) -> String {
    let path = extract_path(&result.url);
    let status_colored = colorize_status_code(result.status_code);
    format!("{} {}", path, status_colored)
}

/// Colorize status code based on value
fn colorize_status_code(status: u16) -> String {
    let status_str = format!("[{}]", status);
    match status {
        200..=299 => status_str.green().to_string(),
        300..=399 => status_str.yellow().to_string(),
        400..=499 => status_str.red().to_string(),
        _ => status_str.white().to_string(),
    }
}

/// Format hits display with multi-column layout (max 10 per column)
fn format_hits_display(hits: &[String]) -> String {
    if hits.is_empty() {
        return "Hits: 0".to_string();
    }

    let total_hits = hits.len();
    let max_per_column = 10;

    if total_hits <= max_per_column {
        // Single column
        let mut display = format!("Hits: {}\n", total_hits);
        for hit in hits {
            display.push_str(&format!("  {}\n", hit));
        }
        display
    } else {
        // Multiple columns
        let num_columns = (total_hits + max_per_column - 1) / max_per_column;
        let mut display = format!(
            "Hits: {} (showing in {} columns)\n",
            total_hits, num_columns
        );

        // Build column layout
        let mut columns: Vec<Vec<&str>> = vec![Vec::new(); num_columns];
        for (idx, hit) in hits.iter().enumerate() {
            let col = idx / max_per_column;
            if col < num_columns {
                columns[col].push(hit);
            }
        }

        // Display columns side by side
        let max_rows = columns.iter().map(|c| c.len()).max().unwrap_or(0);
        for row in 0..max_rows {
            for (col_idx, column) in columns.iter().enumerate() {
                if let Some(hit) = column.get(row) {
                    display.push_str(&format!("  {:50}", hit));
                } else {
                    display.push_str(&format!("{:52}", ""));
                }
                if col_idx < columns.len() - 1 {
                    display.push_str("  ");
                }
            }
            display.push('\n');
        }

        display
    }
}

/// Make a single fuzz request
async fn make_fuzz_request(
    client: &Client,
    url: &str,
    use_head: bool,
) -> Result<FuzzResult, String> {
    let response = if use_head {
        // Use HEAD request to skip body download
        client
            .head(url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
    } else {
        // Use GET request
        client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
    };

    let status_code = response.status().as_u16();
    let content_length = response.content_length();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    Ok(FuzzResult {
        url: url.to_string(),
        status_code,
        content_length,
        content_type,
        source: FuzzSource::Initial, // Will be overwritten by caller
    })
}

/// Extract base URL from a full URL (removes query params and fragments)
pub fn extract_base_url(url: &str) -> Result<String, String> {
    let parsed = Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
    let mut base = parsed.clone();
    base.set_query(None);
    base.set_fragment(None);
    Ok(base.to_string())
}

/// Query database for known endpoints from previous crawls
fn query_database_endpoints(
    db_path: &std::path::Path,
    target_urls: &[String],
) -> Result<Vec<String>, String> {
    use crate::data::Database;

    // Open database
    let db = Database::new(db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    let mut endpoints = Vec::new();

    // Extract domains from target URLs
    let mut target_domains = Vec::new();
    for url in target_urls {
        if let Ok(parsed) = Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                target_domains.push(host.to_string());
            }
        }
    }

    // Query database for nodes matching these domains
    for domain in &target_domains {
        // Simple query - get all nodes for this domain
        let query = "SELECT url FROM nodes WHERE domain = ?";

        if let Ok(mut stmt) = db.get_connection().prepare(query) {
            if let Ok(rows) = stmt.query_map([domain], |row| row.get::<_, String>(0)) {
                for url_result in rows.flatten() {
                    // Only include if it's a valid URL for the target
                    if let Ok(parsed) = Url::parse(&url_result) {
                        if let Some(host) = parsed.host_str() {
                            if host == domain || host.ends_with(&format!(".{}", domain)) {
                                endpoints.push(url_result);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(endpoints)
}

/// Build a test URL from base URL and wordlist entry
pub fn build_test_url(base_url: &str, word: &str) -> Result<String, String> {
    let mut url =
        Url::parse(base_url).map_err(|e| format!("Invalid base URL '{}': {}", base_url, e))?;

    // Get current path
    let current_path = url.path().to_string();

    // Ensure path ends with /
    let path_base = if current_path.ends_with('/') {
        current_path
    } else {
        format!("{}/", current_path)
    };

    // Build new path with word
    let new_path = format!("{}{}", path_base, word.trim_start_matches('/'));

    url.set_path(&new_path);

    Ok(url.to_string())
}

/// Load wordlist from file
pub fn load_wordlist(path: &Path) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read wordlist {}: {}", path.display(), e))?;

    let words: Vec<String> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !line.trim().starts_with('#'))
        .map(|line| line.trim().to_string())
        .collect();

    if words.is_empty() {
        return Err(format!(
            "Wordlist {} is empty or contains only comments",
            path.display()
        ));
    }

    Ok(words)
}

/// Generate a simple fuzz report
pub fn generate_fuzz_report(results: &[FuzzResult]) -> String {
    let mut report = String::new();

    // Count by source
    let initial_count = results
        .iter()
        .filter(|r| r.source == FuzzSource::Initial)
        .count();
    let db_count = results
        .iter()
        .filter(|r| r.source == FuzzSource::Database)
        .count();
    let discovered_count = results
        .iter()
        .filter(|r| r.source == FuzzSource::Discovered)
        .count();

    // Group by status code
    let mut by_status: HashMap<u16, Vec<&FuzzResult>> = HashMap::new();
    for result in results {
        by_status
            .entry(result.status_code)
            .or_default()
            .push(result);
    }

    // Sort status codes
    let mut status_codes: Vec<u16> = by_status.keys().copied().collect();
    status_codes.sort();

    report.push_str(
        "\n═══════════════════════════════════════════════════════════════════════════════\n",
    );
    report.push_str("                            FUZZ RESULTS\n");
    report.push_str(
        "═══════════════════════════════════════════════════════════════════════════════\n\n",
    );

    report.push_str(&format!("Total findings: {}\n", results.len()));
    if db_count > 0 || discovered_count > 0 {
        report.push_str(&format!("  {} from initial targets\n", initial_count));
        if db_count > 0 {
            report.push_str(&format!("  {} from database endpoints\n", db_count));
        }
        if discovered_count > 0 {
            report.push_str(&format!(
                "  {} from discovered endpoints\n",
                discovered_count
            ));
        }
    }
    report.push('\n');

    for status_code in status_codes {
        if let Some(status_results) = by_status.get(&status_code) {
            let status_label = match status_code {
                200..=299 => format!("[{}] Success", status_code),
                300..=399 => format!("[{}] Redirect", status_code),
                400..=499 => format!("[{}] Client Error", status_code),
                _ => format!("[{}]", status_code),
            };

            report.push_str(&format!(
                "{} ({} findings)\n",
                status_label,
                status_results.len()
            ));
            report.push_str(
                "───────────────────────────────────────────────────────────────────────────────\n",
            );

            for result in status_results {
                // Add source indicator
                let source_marker = match result.source {
                    FuzzSource::Initial => "",
                    FuzzSource::Database => " [DB]",
                    FuzzSource::Discovered => " [DISC]",
                };

                report.push_str(&format!("  {}{}", result.url, source_marker));
                if let Some(length) = result.content_length {
                    report.push_str(&format!(" ({} bytes)", length));
                }
                if let Some(ref ct) = result.content_type {
                    let short_ct = ct.split(';').next().unwrap_or(ct);
                    report.push_str(&format!(" [{}]", short_ct));
                }
                report.push('\n');
            }
            report.push('\n');
        }
    }

    report.push_str(
        "═══════════════════════════════════════════════════════════════════════════════\n",
    );
    if discovered_count > 0 {
        report.push_str("  [DB] = From database  [DISC] = Discovered during fuzzing\n");
        report.push_str(
            "═══════════════════════════════════════════════════════════════════════════════\n",
        );
    }
    report.push_str("                            End of Report\n");
    report.push_str(
        "═══════════════════════════════════════════════════════════════════════════════\n",
    );

    report
}
