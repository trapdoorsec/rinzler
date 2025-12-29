use clap::ArgMatches;
use colored::Colorize;
use rinzler_core::data::Database;
use rinzler_tui::crawl_monitor::{self, CrawlMessage, LogLevel};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use url::Url;

const DEFAULT_WORDLIST: &str = include_str!("../wordlists/default.txt");

// Helper functions for crawl handler

/// Load URLs from either a file or a single URL argument
pub fn load_urls_from_source(
    url: Option<&Url>,
    hosts_file: Option<&PathBuf>,
) -> Result<Vec<String>, String> {
    if let Some(hosts_file_path) = hosts_file {
        load_urls_from_file(hosts_file_path)
    } else if let Some(url) = url {
        Ok(vec![url.as_str().to_string()])
    } else {
        Err("Either --url or --hosts-file must be provided".to_string())
    }
}

/// Load and parse URLs from a file
pub fn load_urls_from_file(path: &PathBuf) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read hosts file {}: {}", path.display(), e))?;

    let urls: Vec<String> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| parse_url_line(line.trim()))
        .collect();

    if urls.is_empty() {
        return Err(format!("No valid URLs found in {}", path.display()));
    }

    Ok(urls)
}

/// Parse a single line as a URL, trying to add http:// if needed
pub fn parse_url_line(line: &str) -> Option<String> {
    // Try to parse as-is
    if Url::parse(line).is_ok() {
        return Some(line.to_string());
    }

    // Try adding http://
    let with_scheme = format!("http://{}", line);
    if Url::parse(&with_scheme).is_ok() {
        return Some(with_scheme);
    }

    eprintln!("⚠️  Skipping invalid URL '{}'", line);
    None
}

// Re-export crawl types and functions from rinzler-core
pub use rinzler_core::crawl::{
    CrawlOptions, CrawlProgressCallback, FollowMode, execute_crawl, extract_url_path,
    generate_crawl_report,
};

fn print_divider() {
    println!("{}", "═".repeat(60).bright_blue().bold());
}

fn print_prompt(msg: &str) -> String {
    print!("{} ", msg.bright_cyan().bold());
    io::stdout().flush().unwrap();
    let mut response = String::new();
    io::stdin().read_line(&mut response).unwrap();
    response.trim().to_lowercase()
}

pub fn handle_init(args: &ArgMatches) {
    print_divider();
    println!("{}", "  RINZLER INITIALIZATION".bright_white().bold());
    print_divider();
    println!();

    let db_path = args.get_one::<String>("PATH").unwrap();
    let force = args.get_flag("force");
    let expanded_config_dir = shellexpand::tilde(db_path);
    let rinzler_config_dir = Path::new(expanded_config_dir.as_ref());
    let db_loc = rinzler_config_dir.join("rinzler.db");
    let db_path = db_loc.as_path();
    let user_config_root = rinzler_config_dir.parent().expect("Invalid database path");

    println!("{} Parsed arguments", "✓".green().bold());
    println!(
        "{} Target: {}",
        "→".blue(),
        rinzler_config_dir.display().to_string().bright_white()
    );
    println!();

    let dir_exists = rinzler_config_dir.exists();
    let wordlist_dir = rinzler_config_dir.join("wordlists");
    let wordlist_path = wordlist_dir.join("default.txt");
    let wordlist_exists = wordlist_path.exists();

    // Check for existing installation
    if (dir_exists || wordlist_exists) && !force {
        println!("{}", "⚠ WARNING".yellow().bold());
        println!("Configuration directory already exists:");
        if dir_exists {
            println!(
                "  {} {}",
                "•".yellow(),
                user_config_root.display().to_string().bright_white()
            );
        }
        if wordlist_exists {
            println!(
                "  {} {}",
                "•".yellow(),
                wordlist_path.display().to_string().bright_white()
            );
        }
        println!();
        println!(
            "{}",
            "This operation will overwrite existing files.".yellow()
        );

        let response = print_prompt("Do you want to continue? [y/N]:");
        println!();

        if response != "y" && response != "yes" {
            println!("{} Initialization cancelled.", "✗".red().bold());
            return;
        }
        println!("{} Proceeding with overwrite", "→".yellow().bold());
        println!();
    }

    // Wordlist installation prompt
    let install_wordlist = if !force {
        println!("{}", "WORDLIST SETUP".bright_blue().bold());
        println!("Rinzler includes a default API endpoint wordlist.");
        println!(
            "{} {}",
            "Target:".blue(),
            wordlist_path.display().to_string().bright_white()
        );
        println!();

        let response = print_prompt("Would you like to install it? [Y/n]:");
        println!();

        response != "n" && response != "no"
    } else {
        true
    };

    // Create configuration assets
    if install_wordlist {
        create_configuration_assets(&rinzler_config_dir, &wordlist_dir, &wordlist_path);
    } else {
        println!("{} Skipping wordlist installation", "→".blue());
        println!(
            "{} Manual wordlist location: {}",
            "ℹ".blue(),
            wordlist_dir.display().to_string().bright_white()
        );
        println!();
    }

    // Handle existing database in force mode
    if force && Database::exists(db_path) {
        println!(
            "{} Deleting existing database (force mode)",
            "→".yellow().bold()
        );
        Database::drop(db_path);
        println!("{} Existing database removed", "✓".green().bold());
        println!();
    }

    // Database creation
    if Database::exists(db_path) && !force {
        println!("{}", "⚠ WARNING".yellow().bold());
        println!("Database already exists at:");
        println!(
            "  {} {}",
            "•".yellow(),
            db_path.display().to_string().bright_white()
        );
        println!();

        let response = print_prompt("Would you like to overwrite it? [Y/n]:");
        println!();

        if response == "n" || response == "no" {
            println!("{} Keeping existing database", "→".blue());
            println!();
        } else {
            Database::drop(db_path);
            println!("{} Existing database removed", "✓".green().bold());
            println!();
        }
    }

    if !Database::exists(db_path) {
        println!("{} Creating database...", "→".blue());
        Database::new(db_path).expect("Failed to create database");
        println!(
            "{} Database initialized: {}",
            "✓".green().bold(),
            db_path.display().to_string().bright_white()
        );
    }

    println!();
    print_divider();
    println!("{}", "  INITIALIZATION COMPLETE".green().bold());
    print_divider();
    println!();
    println!(
        "{} Config directory: {}",
        "✓".green().bold(),
        user_config_root.display().to_string().bright_white()
    );
    println!(
        "{} Database: {}",
        "✓".green().bold(),
        db_path.display().to_string().bright_white()
    );
    if install_wordlist {
        println!(
            "{} Wordlist: {}",
            "✓".green().bold(),
            wordlist_path.display().to_string().bright_white()
        );
    }
    println!();
}

fn create_configuration_assets(
    rinzler_config_dir: &&Path,
    wordlist_dir: &PathBuf,
    wordlist_path: &PathBuf,
) {
    println!("{} Creating directory structure...", "→".blue());

    fs::create_dir_all(rinzler_config_dir).expect("Failed to create config directory");
    println!(
        "  {} {}",
        "✓".green(),
        rinzler_config_dir.display().to_string().bright_white()
    );

    fs::create_dir_all(wordlist_dir).expect("Failed to create wordlists directory");
    println!(
        "  {} {}",
        "✓".green(),
        wordlist_dir.display().to_string().bright_white()
    );

    println!("{} Installing default wordlist...", "→".blue());
    fs::write(wordlist_path, DEFAULT_WORDLIST).expect("Failed to write default wordlist");

    let wordlist_size = DEFAULT_WORDLIST.len();
    let line_count = DEFAULT_WORDLIST.lines().count();
    println!(
        "  {} {} ({} entries, {} bytes)",
        "✓".green().bold(),
        wordlist_path.display().to_string().bright_white(),
        line_count.to_string().cyan(),
        wordlist_size.to_string().cyan()
    );
    println!();
}

pub fn handle_workspace_create(args: &ArgMatches) {
    let name = args.get_one::<String>("name").unwrap();
    println!("Creating workspace: {}", name);
    // TODO: Implement workspace creation
}

pub fn handle_workspace_remove(args: &ArgMatches) {
    let name = args.get_one::<String>("name").unwrap();
    println!("Removing workspace: {}", name);
    // TODO: Implement workspace removal
}

pub fn handle_workspace_list() {
    println!("Listing workspaces");
    // TODO: Implement workspace listing
}

pub fn handle_workspace_rename(args: &ArgMatches) {
    let old_name = args.get_one::<String>("old-name").unwrap();
    let new_name = args.get_one::<String>("new-name").unwrap();
    println!("Renaming workspace from '{}' to '{}'", old_name, new_name);
    // TODO: Implement workspace renaming
}

pub async fn handle_crawl(sub_matches: &ArgMatches) {
    let url = sub_matches.get_one::<Url>("url");
    let hosts_file = sub_matches.get_one::<PathBuf>("hosts-file");
    let threads = *sub_matches.get_one::<usize>("threads").unwrap_or(&10);
    let follow = sub_matches.get_flag("follow");
    let auto_follow = sub_matches.get_flag("auto-follow");

    // Load URLs from source
    let urls = match load_urls_from_source(url, hosts_file) {
        Ok(urls) => urls,
        Err(e) => {
            eprintln!("✗ {}", e);
            std::process::exit(1);
        }
    };

    // Determine follow mode
    let follow_mode = if auto_follow {
        FollowMode::Auto
    } else if follow {
        FollowMode::Prompt
    } else {
        FollowMode::Disabled
    };

    // Print crawl configuration
    println!("\n🕷️  Crawling {} host(s)", urls.len());
    println!("Workers: {}", threads);
    println!("Max depth: 3");
    let follow_mode_str = match follow_mode {
        FollowMode::Auto => "auto (follow all)",
        FollowMode::Prompt => "prompt (ask user)",
        FollowMode::Disabled => "disabled (same domain only)",
    };
    println!("Cross-domain: {}\n", follow_mode_str);

    // Open database
    let db_path = shellexpand::tilde("~/.config/rinzler/rinzler.db");
    let db = match Database::new(Path::new(db_path.as_ref())) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("✗ Failed to open database: {}", e);
            eprintln!("  Run 'rinzler init' first to create the database.");
            std::process::exit(1);
        }
    };

    // Create session
    let seed_urls_json = serde_json::to_string(&urls).unwrap();
    let session_id = match db.create_session("crawl", &seed_urls_json) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("✗ Failed to create session: {}", e);
            std::process::exit(1);
        }
    };

    // Create map
    let map_id = match db.create_map(&session_id) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("✗ Failed to create map: {}", e);
            std::process::exit(1);
        }
    };

    println!("Session ID: {}", session_id.bright_white());
    println!();

    // Create TUI monitor channel and spawn monitor in separate OS thread
    let (tx, rx) = crawl_monitor::create_monitor_channel();
    let should_exit = Arc::new(AtomicBool::new(false));
    let should_exit_clone = should_exit.clone();

    let tui_handle = std::thread::spawn(move || {
        if let Err(e) = crawl_monitor::run_monitor(rx, should_exit_clone) {
            eprintln!("TUI error: {}", e);
        }
    });

    // Send session ID to TUI
    let _ = tx.send(CrawlMessage::SessionStarted {
        session_id: session_id.clone(),
    });

    // Create crawl options (disable built-in progress bars, using TUI instead)
    let options = CrawlOptions {
        urls,
        threads,
        max_depth: 3,
        follow_mode,
        show_progress_bars: false,  // Using TUI instead
    };

    // Execute crawl with progress callback that sends to TUI
    let tx_progress = tx.clone();
    let progress_callback = Arc::new(move |msg: String| {
        let _ = tx_progress.send(CrawlMessage::Log {
            level: LogLevel::Info,
            message: msg,
        });
    });

    // Result callback that sends findings to TUI in real-time
    let tx_result = tx.clone();
    let result_callback = Arc::new(move |result: rinzler_scanner::result::CrawlResult| {
        // Perform security analysis on this result
        // Note: We use a dummy node_id of 0 since we haven't inserted to DB yet
        let findings = rinzler_core::security::analyze_crawl_result(&result, 0);

        // Convert findings to TUI SecurityFinding format
        let security_findings: Vec<crawl_monitor::SecurityFinding> = findings
            .iter()
            .map(|f| {
                let severity_str = match f.severity {
                    rinzler_core::data::Severity::Critical => "critical",
                    rinzler_core::data::Severity::High => "high",
                    rinzler_core::data::Severity::Medium => "medium",
                    rinzler_core::data::Severity::Low => "low",
                    rinzler_core::data::Severity::Info => "info",
                };

                crawl_monitor::SecurityFinding {
                    title: f.title.clone(),
                    severity: severity_str.to_string(),
                    description: f.description.clone(),
                    impact: f.impact.clone().unwrap_or_else(|| "No impact information available".to_string()),
                    remediation: f.remediation.clone().unwrap_or_else(|| "No remediation available".to_string()),
                    cwe: f.cwe_id.clone(),
                    owasp: f.owasp_category.clone(),
                }
            })
            .collect();

        let _ = tx_result.send(CrawlMessage::Finding {
            url: result.url.clone(),
            status_code: result.status_code,
            content_type: result.content_type.clone(),
            security_findings,
        });
    });

    let start_time = std::time::Instant::now();
    let all_results = match execute_crawl(options, Some(progress_callback), Some(result_callback)).await {
        Ok(results) => results,
        Err(e) => {
            let _ = tx.send(CrawlMessage::Log {
                level: LogLevel::Error,
                message: format!("Crawl failed: {}", e),
            });
            let _ = db.fail_session(&session_id);
            should_exit.store(true, Ordering::Relaxed);
            let _ = tui_handle.join();
            std::process::exit(1);
        }
    };
    let duration = start_time.elapsed();

    // Note: Findings are already sent in real-time via result_callback
    // No need to send them again here

    let _ = tx.send(CrawlMessage::Log {
        level: LogLevel::Info,
        message: format!("Crawl complete! Duration: {:.2}s", duration.as_secs_f64()),
    });

    let _ = tx.send(CrawlMessage::Log {
        level: LogLevel::Info,
        message: "Persisting results to database...".to_string(),
    });

    // Persist results to database
    let mut findings_count = 0;
    for result in &all_results {
        // Extract domain from URL
        let domain = Url::parse(&result.url)
            .ok()
            .and_then(|u| u.host_str().map(String::from))
            .unwrap_or_else(|| "unknown".to_string());

        // Create node structure
        let node = rinzler_core::data::CrawlNode {
            url: result.url.clone(),
            domain,
            status_code: result.status_code,
            content_type: result.content_type.clone(),
            content_length: None,
            response_time_ms: None,
            title: None,
            forms_count: result.forms_found,
            service_type: None,
            headers: None,
            body_sample: None,
        };

        // Insert node
        match db.insert_node(&map_id, &node) {
            Ok(node_id) => {
                // Run security checks
                let findings = rinzler_core::security::analyze_crawl_result(result, node_id);

                // Insert findings
                for finding in findings {
                    if db.insert_finding(&session_id, &finding).is_ok() {
                        findings_count += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "  {} Failed to insert node {}: {}",
                    "⚠".yellow(),
                    result.url,
                    e
                );
            }
        }
    }

    // Complete session
    if let Err(e) = db.complete_session(&session_id) {
        let _ = tx.send(CrawlMessage::Log {
            level: LogLevel::Error,
            message: format!("Failed to complete session: {}", e),
        });
    }

    let _ = tx.send(CrawlMessage::Log {
        level: LogLevel::Info,
        message: format!("Saved {} nodes and {} findings to database", all_results.len(), findings_count),
    });

    // Send findings summary to TUI
    if findings_count > 0 {
        let _ = tx.send(CrawlMessage::Log {
            level: LogLevel::Info,
            message: "=".repeat(50),
        });
        let _ = tx.send(CrawlMessage::Log {
            level: LogLevel::Info,
            message: "SECURITY FINDINGS SUMMARY".to_string(),
        });
        let _ = tx.send(CrawlMessage::Log {
            level: LogLevel::Info,
            message: "=".repeat(50),
        });

        // Display findings summary
        if let Ok(severity_counts) = db.get_findings_count_by_severity(&session_id) {
            for (severity, count) in severity_counts {
                let _ = tx.send(CrawlMessage::Log {
                    level: LogLevel::Info,
                    message: format!("  {}: {}", severity.to_uppercase(), count),
                });
            }
        }
    }

    // Handle report generation and output to file (if specified)
    let output_path = sub_matches.get_one::<PathBuf>("output");
    let format = sub_matches
        .get_one::<String>("format")
        .map(|s| s.as_str())
        .unwrap_or("text");
    let include_sitemap = sub_matches.get_flag("include-sitemap");

    if let Some(path) = output_path {
        let _ = tx.send(CrawlMessage::Log {
            level: LogLevel::Info,
            message: format!("Generating {} report...", format),
        });

        match rinzler_core::report::gather_report_data(&db, &session_id, include_sitemap) {
            Ok(report_data) => {
                let report_content = match format {
                    "text" => rinzler_core::report::generate_text_report(&report_data),
                    "json" => rinzler_core::report::generate_json_report(&report_data)
                        .unwrap_or_else(|e| {
                            let _ = tx.send(CrawlMessage::Log {
                                level: LogLevel::Error,
                                message: format!("Failed to generate JSON: {}", e),
                            });
                            String::new()
                        }),
                    "csv" => {
                        let _ = tx.send(CrawlMessage::Log {
                            level: LogLevel::Warn,
                            message: "CSV format not yet implemented".to_string(),
                        });
                        String::new()
                    }
                    "html" => {
                        let _ = tx.send(CrawlMessage::Log {
                            level: LogLevel::Warn,
                            message: "HTML format not yet implemented".to_string(),
                        });
                        String::new()
                    }
                    "markdown" => {
                        let _ = tx.send(CrawlMessage::Log {
                            level: LogLevel::Warn,
                            message: "Markdown format not yet implemented".to_string(),
                        });
                        String::new()
                    }
                    _ => {
                        let _ = tx.send(CrawlMessage::Log {
                            level: LogLevel::Error,
                            message: format!("Unknown format: {}", format),
                        });
                        String::new()
                    }
                };

                if !report_content.is_empty() {
                    match rinzler_core::report::save_report(&report_content, path) {
                        Ok(_) => {
                            let _ = tx.send(CrawlMessage::Log {
                                level: LogLevel::Info,
                                message: format!("Report saved to: {}", path.display()),
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(CrawlMessage::Log {
                                level: LogLevel::Error,
                                message: format!("Failed to save report: {}", e),
                            });
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(CrawlMessage::Log {
                    level: LogLevel::Error,
                    message: format!("Failed to generate report: {}", e),
                });
            }
        }
    }

    // Send completion message to TUI with all required fields
    let _ = tx.send(CrawlMessage::Complete {
        total: all_results.len(),
        findings_count,
    });

    // Wait for TUI to close (user presses 'q' or ESC)
    let _ = tui_handle.join();
}

pub async fn handle_fuzz(sub_matches: &ArgMatches) {
    let url = sub_matches.get_one::<Url>("url");
    let hosts_file = sub_matches.get_one::<PathBuf>("hosts-file");
    let wordlist_file = sub_matches.get_one::<PathBuf>("wordlist-file");
    let threads = *sub_matches.get_one::<usize>("threads").unwrap_or(&10);
    let full_body = sub_matches.get_flag("full-body");
    let use_head = !full_body; // Default to HEAD unless --full-body is specified
    let timeout = *sub_matches.get_one::<u64>("timeout").unwrap_or(&5);

    // Load URLs from source
    let urls = match load_urls_from_source(url, hosts_file) {
        Ok(urls) => urls,
        Err(e) => {
            eprintln!("✗ {}", e);
            std::process::exit(1);
        }
    };

    // Load wordlist - use default if not specified
    let default_wordlist_path = {
        let expanded = shellexpand::tilde("~/.config/rinzler/wordlists/default.txt");
        PathBuf::from(expanded.as_ref())
    };

    let wordlist_path = wordlist_file.cloned().unwrap_or(default_wordlist_path);

    let wordlist = match rinzler_core::fuzz::load_wordlist(&wordlist_path) {
        Ok(words) => words,
        Err(e) => {
            eprintln!("✗ Failed to load wordlist: {}", e);
            eprintln!("  Try specifying a wordlist with -w or ensure the default wordlist exists");
            std::process::exit(1);
        }
    };

    // Print fuzz configuration
    println!("\n🎯 Fuzzing {} target(s)", urls.len());
    println!("Workers: {}", threads);
    println!(
        "Wordlist: {} entries from {}",
        wordlist.len(),
        wordlist_path.display()
    );
    println!("Method: {}", if use_head { "HEAD" } else { "GET" });
    println!("Timeout: {}s", timeout);
    println!("Total requests: {}\n", urls.len() * wordlist.len());

    // Get database path
    let db_path = {
        let expanded = shellexpand::tilde("~/.config/rinzler/rinzler.db");
        let path = PathBuf::from(expanded.as_ref());
        if path.exists() { Some(path) } else { None }
    };

    // Execute fuzzing
    let options = rinzler_core::fuzz::FuzzOptions {
        base_urls: urls,
        wordlist,
        threads,
        show_progress_bars: true,
        use_head_requests: use_head,
        timeout_secs: timeout,
        db_path,
    };

    let start_time = std::time::Instant::now();
    let results = match rinzler_core::fuzz::execute_fuzz(options).await {
        Ok(results) => results,
        Err(e) => {
            eprintln!("✗ Fuzzing failed: {}", e);
            std::process::exit(1);
        }
    };
    let duration = start_time.elapsed();

    println!("\n✓ Fuzzing complete!");
    println!(
        "  Duration: {:.2}s",
        duration.as_secs_f64()
    );
    println!(
        "  Requests/sec: {:.2}\n",
        results.len() as f64 / duration.as_secs_f64()
    );

    // Generate and display report
    let report = rinzler_core::fuzz::generate_fuzz_report(&results);
    println!("{}", report);
}

pub fn handle_plugin_list() {
    println!("Listing plugins");
    // TODO: Implement plugin listing
}

pub fn handle_plugin_register(args: &ArgMatches) {
    let file = args.get_one::<PathBuf>("file").unwrap();
    let name = args.get_one::<String>("name").unwrap();
    println!(
        "Registering plugin '{}' from file: {}",
        name,
        file.display()
    );
    // TODO: Implement plugin registration
}

pub fn handle_plugin_unregister(args: &ArgMatches) {
    let name = args.get_one::<String>("name").unwrap();
    println!("Unregistering plugin: {}", name);
    // TODO: Implement plugin unregistration
}
