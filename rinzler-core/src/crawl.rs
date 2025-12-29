use indicatif::{ProgressBar, ProgressStyle};
use rinzler_scanner::Crawler;
use rinzler_scanner::result::CrawlResult;
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::sync::{Arc, Mutex as StdMutex};
use url::Url;

/// Options for configuring a crawl operation
pub struct CrawlOptions {
    pub urls: Vec<String>,
    pub threads: usize,
    pub max_depth: usize,
    pub follow_mode: FollowMode,
    pub show_progress_bars: bool,
}

/// Cross-domain following behavior
pub enum FollowMode {
    /// Never follow cross-domain links
    Disabled,
    /// Prompt user for each new cross-domain
    Prompt,
    /// Automatically follow all cross-domain links
    Auto,
}

/// Callback for reporting crawl progress
pub type CrawlProgressCallback = Arc<dyn Fn(String) + Send + Sync>;

/// Callback for reporting individual crawl results as they come in
pub type CrawlResultCallback = Arc<dyn Fn(CrawlResult) + Send + Sync>;

/// Extract the path component from a URL
pub fn extract_url_path(url: &str) -> String {
    Url::parse(url)
        .ok()
        .map(|u| {
            let path = u.path().to_string();
            if path.is_empty() || path == "/" {
                "/".to_string()
            } else {
                path
            }
        })
        .unwrap_or_else(|| url.to_string())
}

/// Execute a crawl with the given options
/// Returns the crawl results
pub async fn execute_crawl(
    options: CrawlOptions,
    progress_callback: Option<CrawlProgressCallback>,
    result_callback: Option<CrawlResultCallback>,
) -> Result<Vec<CrawlResult>, String> {
    let CrawlOptions {
        urls,
        threads,
        max_depth,
        follow_mode,
        show_progress_bars,
    } = options;

    // Set up single progress bar for overall crawl progress (only if enabled)
    let progress_bar = if show_progress_bars {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message("Starting crawl...");
        Some(Arc::new(pb))
    } else {
        None
    };

    // Counter for tracking processed URLs
    let processed_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // Progress callback for worker updates (only if progress bars enabled)
    let internal_progress_callback: rinzler_scanner::ProgressCallback = if show_progress_bars {
        let pb_clone = progress_bar.clone().unwrap();
        let count_clone = processed_count.clone();
        Arc::new(move |_worker_id: usize, _url: String| {
            let count = count_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            pb_clone.set_message(format!("Crawling... {} URLs processed", count));
            pb_clone.tick();
        })
    } else {
        // No-op callback when progress bars are disabled
        Arc::new(|_worker_id: usize, _url: String| {})
    };

    // Cross-domain callback (changes behavior based on follow_mode)
    let cross_domain_callback: rinzler_scanner::CrossDomainCallback = match follow_mode {
        FollowMode::Auto => {
            // Auto-follow mode: always accept cross-domain links
            Arc::new(|_url: String, _base: String| -> bool { true })
        }
        FollowMode::Prompt => {
            // Prompt mode: ask user and remember decisions
            // Note: This only works when show_progress_bars is true (CLI mode)
            // In TUI mode, use FollowMode::Disabled instead
            let domain_decisions: Arc<StdMutex<(HashSet<String>, HashSet<String>)>> =
                Arc::new(StdMutex::new((HashSet::new(), HashSet::new())));

            let pb_clone = progress_bar.clone();
            let domain_decisions_clone = domain_decisions.clone();
            Arc::new(move |url: String, _base: String| -> bool {
                let parsed = Url::parse(&url).ok();
                let domain = parsed
                    .as_ref()
                    .and_then(|u| u.host_str())
                    .unwrap_or("unknown")
                    .to_string();

                // Lock to check decisions atomically
                let mut decisions = domain_decisions_clone.lock().unwrap();
                let (ref mut approved, ref mut denied) = *decisions;

                // Check if we've already made a decision for this domain
                if approved.contains(&domain) {
                    return true;
                }
                if denied.contains(&domain) {
                    return false;
                }

                // Not in either set - ask the user (only if progress bar is available)
                let result = if let Some(ref pb) = pb_clone {
                    pb.suspend(|| {
                        print!(
                            "\n[!] Cross-domain link detected: {}\nFollow this link? [y/N]: ",
                            domain
                        );
                        io::stdout().flush().unwrap();

                        let mut response = String::new();
                        io::stdin().read_line(&mut response).unwrap();
                        let response = response.trim().to_lowercase();

                        response == "y" || response == "yes"
                    })
                } else {
                    // No progress bars available (TUI mode) - deny by default
                    false
                };

                // Store the decision before releasing the lock
                if result {
                    approved.insert(domain);
                } else {
                    denied.insert(domain);
                }

                result
            })
        }
        FollowMode::Disabled => {
            // Default mode: never follow cross-domain links
            Arc::new(|_url: String, _base: String| -> bool { false })
        }
    };

    // Create crawler with callbacks
    let mut crawler = Crawler::new()
        .with_max_depth(max_depth)
        .with_auto_follow(false) // We handle cross-domain logic in the callback now
        .with_progress_callback(internal_progress_callback)
        .with_cross_domain_callback(cross_domain_callback);

    // Add result callback if provided (converts CrawlResultCallback to ResultCallback)
    if let Some(ref cb) = result_callback {
        let cb_clone = cb.clone();
        let result_cb: rinzler_scanner::ResultCallback = Arc::new(move |result: CrawlResult| {
            cb_clone(result);
        });
        crawler = crawler.with_result_callback(result_cb);
    }

    // Crawl each URL
    let mut all_results = Vec::new();
    for (idx, url_str) in urls.iter().enumerate() {
        if let Some(ref callback) = progress_callback
            && urls.len() > 1
        {
            callback(format!(
                "Crawling host {}/{}: {}",
                idx + 1,
                urls.len(),
                url_str
            ));
        }

        match crawler.crawl(url_str, threads).await {
            Ok(results) => {
                all_results.extend(results);
            }
            Err(e) => {
                if let Some(ref callback) = progress_callback {
                    callback(format!("[!]  Failed to crawl {}: {}", url_str, e));
                }
            }
        }
    }

    // Finish progress bar (only if enabled)
    if let Some(ref pb) = progress_bar {
        let total = processed_count.load(std::sync::atomic::Ordering::Relaxed);
        pb.finish_with_message(format!("Crawl complete! {} URLs processed", total));
    }

    Ok(all_results)
}

/// Generate a crawl report from results
pub fn generate_crawl_report(results: &[CrawlResult]) -> String {
    // Filter out 404s
    let filtered_results: Vec<&CrawlResult> =
        results.iter().filter(|r| r.status_code != 404).collect();

    let mut report = String::new();
    report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");
    report.push_str("# Summary:\n");
    report.push_str(&format!("  Pages crawled: {}\n", filtered_results.len()));

    let total_links: usize = filtered_results.iter().map(|r| r.links_found.len()).sum();
    report.push_str(&format!("  Total links found: {}\n", total_links));

    let total_forms: usize = filtered_results.iter().map(|r| r.forms_found).sum();
    report.push_str(&format!("  Total forms found: {}\n", total_forms));

    let total_scripts: usize = filtered_results.iter().map(|r| r.scripts_found).sum();
    report.push_str(&format!("  Total scripts found: {}\n", total_scripts));

    report.push_str("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    // Group results by host
    let mut by_host: HashMap<String, Vec<&CrawlResult>> = HashMap::new();

    for result in filtered_results {
        if let Ok(url) = Url::parse(&result.url)
            && let Some(host) = url.host_str()
        {
            by_host.entry(host.to_string()).or_default().push(result);
        }
    }

    // Display results grouped by host
    for (host, host_results) in by_host.iter() {
        report.push_str(&format!("## {}\n", host));
        report.push_str(&format!("  {} pages found\n\n", host_results.len()));

        for result in host_results {
            let path = extract_url_path(&result.url);

            // Color code based on status
            let status_str = match result.status_code {
                100..=199 => format!("\x1b[37m{}\x1b[0m", result.status_code), // White
                200..=299 => format!("\x1b[32m{}\x1b[0m", result.status_code), // Green
                300..=399 => format!("\x1b[36m{}\x1b[0m", result.status_code), // Cyan
                400..=499 => format!("\x1b[33m{}\x1b[0m", result.status_code), // Orange/Yellow
                500..=599 => format!("\x1b[31m{}\x1b[0m", result.status_code), // Red
                _ => format!("{}", result.status_code),
            };

            // Build line with path and status
            let mut line = format!("  {} {}", status_str, path);

            // Only show MIME type if it's not text/html
            if let Some(ref content_type) = result.content_type
                && content_type != "text/html"
            {
                line.push_str(&format!(" \x1b[90m{}\x1b[0m", content_type));
            }

            report.push_str(&line);
            report.push('\n');
        }
        report.push('\n');
    }

    report
}

#[cfg(test)]
mod tests {
}
