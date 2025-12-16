// Fuzzing module for forced browsing / directory enumeration

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use url::Url;

/// Result of a fuzz attempt
#[derive(Debug, Clone)]
pub struct FuzzResult {
    pub url: String,
    pub status_code: u16,
    pub content_length: Option<u64>,
    pub content_type: Option<String>,
}

/// Options for configuring a fuzz operation
pub struct FuzzOptions {
    pub base_urls: Vec<String>,
    pub wordlist: Vec<String>,
    pub threads: usize,
    pub show_progress_bars: bool,
}

/// Execute fuzzing with given options
pub async fn execute_fuzz(
    options: FuzzOptions,
) -> Result<Vec<FuzzResult>, String> {
    let FuzzOptions {
        base_urls,
        wordlist,
        threads,
        show_progress_bars,
    } = options;

    if base_urls.is_empty() {
        return Err("No base URLs provided".to_string());
    }

    if wordlist.is_empty() {
        return Err("Wordlist is empty".to_string());
    }

    // Build full URLs to test
    let mut urls_to_test = Vec::new();
    for base_url in &base_urls {
        for word in &wordlist {
            let test_url = build_test_url(base_url, word)?;
            urls_to_test.push(test_url);
        }
    }

    let total_requests = urls_to_test.len();
    println!("Testing {} URLs with {} workers\n", total_requests, threads);

    // Set up multi-progress for worker tracking
    let m = if show_progress_bars {
        Some(Arc::new(MultiProgress::new()))
    } else {
        None
    };

    // Create shared results vector
    let results: Arc<Mutex<Vec<FuzzResult>>> = Arc::new(Mutex::new(Vec::new()));

    // Create HTTP client
    let client = Arc::new(
        Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(3))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?
    );

    // Create semaphore to limit concurrent requests
    let semaphore = Arc::new(Semaphore::new(threads));

    // Distribute URLs across workers
    let urls_per_worker = (total_requests + threads - 1) / threads;
    let mut worker_tasks = Vec::new();

    for worker_id in 0..threads {
        let start_idx = worker_id * urls_per_worker;
        let end_idx = std::cmp::min(start_idx + urls_per_worker, total_requests);

        if start_idx >= total_requests {
            break;
        }

        let worker_urls = urls_to_test[start_idx..end_idx].to_vec();
        let worker_count = worker_urls.len();

        // Create progress bar for this worker
        let pb = if show_progress_bars && let Some(ref multi_progress) = m {
            let progress_bar = multi_progress.add(ProgressBar::new(worker_count as u64));
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template("[{bar:40.cyan/blue}] Worker {msg} {pos}/{len}")
                    .unwrap()
                    .progress_chars("=>-"),
            );
            progress_bar.set_message(format!("{}", worker_id));
            Some(progress_bar)
        } else {
            None
        };

        let client_clone = client.clone();
        let results_clone = results.clone();
        let semaphore_clone = semaphore.clone();

        let task = tokio::spawn(async move {
            for url in worker_urls {
                // Acquire semaphore permit
                let _permit = semaphore_clone.acquire().await.unwrap();

                // Make request
                if let Ok(result) = make_fuzz_request(&client_clone, &url).await {
                    // Only save successful responses (200-399) or interesting errors
                    if result.status_code < 500 {
                        results_clone.lock().await.push(result);
                    }
                }

                // Update progress bar
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
        task.await.map_err(|e| format!("Worker task failed: {}", e))?;
    }

    // Extract results
    let final_results = results.lock().await.clone();

    Ok(final_results)
}

/// Make a single fuzz request
async fn make_fuzz_request(client: &Client, url: &str) -> Result<FuzzResult, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

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
    })
}

/// Build a test URL from base URL and wordlist entry
fn build_test_url(base_url: &str, word: &str) -> Result<String, String> {
    let mut url = Url::parse(base_url)
        .map_err(|e| format!("Invalid base URL '{}': {}", base_url, e))?;

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
        return Err(format!("Wordlist {} is empty or contains only comments", path.display()));
    }

    Ok(words)
}

/// Generate a simple fuzz report
pub fn generate_fuzz_report(results: &[FuzzResult]) -> String {
    let mut report = String::new();

    // Group by status code
    let mut by_status: HashMap<u16, Vec<&FuzzResult>> = HashMap::new();
    for result in results {
        by_status.entry(result.status_code).or_default().push(result);
    }

    // Sort status codes
    let mut status_codes: Vec<u16> = by_status.keys().copied().collect();
    status_codes.sort();

    report.push_str("\n═══════════════════════════════════════════════════════════════════════════════\n");
    report.push_str("                            FUZZ RESULTS\n");
    report.push_str("═══════════════════════════════════════════════════════════════════════════════\n\n");

    report.push_str(&format!("Total findings: {}\n\n", results.len()));

    for status_code in status_codes {
        if let Some(status_results) = by_status.get(&status_code) {
            let status_label = match status_code {
                200..=299 => format!("[{}] Success", status_code),
                300..=399 => format!("[{}] Redirect", status_code),
                400..=499 => format!("[{}] Client Error", status_code),
                _ => format!("[{}]", status_code),
            };

            report.push_str(&format!("{} ({} findings)\n", status_label, status_results.len()));
            report.push_str("───────────────────────────────────────────────────────────────────────────────\n");

            for result in status_results {
                report.push_str(&format!("  {}", result.url));
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

    report.push_str("═══════════════════════════════════════════════════════════════════════════════\n");
    report.push_str("                            End of Report\n");
    report.push_str("═══════════════════════════════════════════════════════════════════════════════\n");

    report
}
