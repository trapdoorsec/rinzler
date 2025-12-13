use clap;
use clap::ArgMatches;
use commands::command_argument_builder;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use pager::Pager;
use rinzler_core::{data::Database, print_banner};
use rinzler_scanner::Crawler;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing_subscriber;
use url::Url;

mod commands;

#[tokio::main]
async fn main() {
    let cmd = command_argument_builder();
    let chosen_command = cmd.get_matches();
    let quiet = chosen_command.get_flag("quiet");

    // Show banner unless --quiet flag is set
    if !quiet {
        print_banner();
    }

    if chosen_command.subcommand().is_none() {
        // No subcommand provided, just show the banner
        return;
    }

    match chosen_command.subcommand() {
        Some(("ui", _)) => {
            // Launch TUI REPL
            if let Err(e) = rinzler_tui::run() {
                eprintln!("Error running TUI: {}", e);
                std::process::exit(1);
            }
        }
        Some(("init", primary_command)) => handle_init(primary_command),
        Some(("workspace", primary_command)) => match primary_command.subcommand() {
            Some(("create", secondary_command)) => handle_workspace_create(secondary_command),
            Some(("remove", secondary_command)) => handle_workspace_remove(secondary_command),
            Some(("list", _)) => handle_workspace_list(),
            Some(("rename", secondary_command)) => handle_workspace_rename(secondary_command),
            _ => unreachable!("clap should ensure we don't get here"),
        },
        Some(("crawl", primary_command)) => handle_crawl(primary_command).await,
        Some(("fuzz", primary_command)) => handle_fuzz(primary_command).await,
        Some(("plugin", primary_command)) => match primary_command.subcommand() {
            Some(("list", _)) => handle_plugin_list(),
            Some(("register", secondary_command)) => handle_plugin_register(secondary_command),
            Some(("unregister", secondary_command)) => handle_plugin_unregister(secondary_command),
            _ => unreachable!("clap should ensure we don't get here"),
        },
        _ => unreachable!("clap should ensure we don't get here"),
    }
}

// Handler functions
fn handle_init(args: &ArgMatches) {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner.set_message("Let's get this show on the road!");

    let db_path = args.get_one::<String>("PATH").unwrap();
    let force = args.get_flag("force");
    let expanded_config_dir = shellexpand::tilde(db_path);
    let rinzler_config_dir = Path::new(expanded_config_dir.as_ref());
    let db_loc = rinzler_config_dir.join("rinzler.db");
    let db_path = db_loc.as_path();
    let user_config_root = rinzler_config_dir.parent().expect("Invalid database path");

    // Check if config directory exists
    let dir_exists = rinzler_config_dir.exists();
    let wordlist_dir = rinzler_config_dir.join("wordlists");
    let wordlist_path = wordlist_dir.join("default.txt");
    let wordlist_exists = wordlist_path.exists();

    // If directory exists and force is not set, ask for confirmation
    if (dir_exists || wordlist_exists) && !force {
        spinner.println("[WARNING] Configuration directory already exists:");
        if dir_exists {
            spinner.println(format!("  - Directory: {}", user_config_root.display()));
        }
        if wordlist_exists {
            spinner.println(format!("  - Wordlist: {}", wordlist_path.display()));
        }

        spinner.println("This operation will overwrite existing files.");
        spinner.println("Do you want to continue? [y/N]: ");
        io::stdout().flush().unwrap();

        let mut response = String::new();
        io::stdin().read_line(&mut response).unwrap();
        let response = response.trim().to_lowercase();

        if response != "y" && response != "yes" {
            println!("\nInitialization cancelled.");
            return;
        }
    }

    // Ask if user wants to install the default wordlist
    if !force {
        println!("\n[SETUP] Rinzler includes a default API endpoint wordlist.");
        print!(
            "Would you like to install it to {}? [Y/n]: ",
            wordlist_path.display()
        );
        io::stdout().flush().unwrap();

        let mut response = String::new();
        io::stdin().read_line(&mut response).unwrap();
        let response = response.trim().to_lowercase();

        if response == "n" || response == "no" {
            println!("\nSkipping wordlist installation.");
            println!(
                "You can manually add wordlists to: {}",
                wordlist_dir.display()
            );
        } else {
            create_configuration_assets(
                &spinner,
                &rinzler_config_dir,
                &wordlist_dir,
                &wordlist_path,
            );
        }
    } else {
        create_configuration_assets(&spinner, &rinzler_config_dir, &wordlist_dir, &wordlist_path);
        sleep(Duration::from_millis(1000));
        //if database already exists
        if Database::exists(db_path) {
            spinner.set_message("\n✓ deleting existing database");
            sleep(Duration::from_millis(1000));
            Database::drop(db_path);
        }
    }

    // Initialize database
    spinner.set_message(format!("Initializing database at: {}", db_path.display()));
    sleep(Duration::from_millis(1000));
    Database::new(db_path).expect("Failed to create database");

    spinner.finish_with_message(format!(
        r#"
    ✓ Rinzler initialization complete!
    ✓ Config directory: {}
    ✓ Database: {}
    "#,
        user_config_root.display(),
        db_path.display()
    ));
    sleep(Duration::from_millis(300));
}
const DEFAULT_WORDLIST: &str = include_str!("../wordlists/default.txt");
fn create_configuration_assets(
    spinner: &ProgressBar,
    rinzler_config_dir: &&Path,
    wordlist_dir: &PathBuf,
    wordlist_path: &PathBuf,
) {
    sleep(Duration::from_millis(2000));
    // Create directory structure
    spinner.set_message("Creating configuration directory structure...");
    sleep(Duration::from_millis(1000));
    fs::create_dir_all(&rinzler_config_dir).expect("Failed to create config directory");
    fs::create_dir_all(&wordlist_dir).expect("Failed to create wordlists directory");
    spinner.set_message("✓ Directories created");
    sleep(Duration::from_millis(1000));
    // Write the bundled wordlist
    spinner.set_message("Installing default wordlist...");
    sleep(Duration::from_millis(1000));
    fs::write(&wordlist_path, DEFAULT_WORDLIST).expect("Failed to write default wordlist");
    spinner.set_message(format!(
        "✓ Default wordlist installed to: {}",
        wordlist_path.display()
    ));
    sleep(Duration::from_millis(1000));
}

fn handle_workspace_create(args: &ArgMatches) {
    let name = args.get_one::<String>("name").unwrap();
    println!("Creating workspace: {}", name);
    // TODO: Implement workspace creation
}

fn handle_workspace_remove(args: &ArgMatches) {
    let name = args.get_one::<String>("name").unwrap();
    println!("Removing workspace: {}", name);
    // TODO: Implement workspace removal
}

fn handle_workspace_list() {
    println!("Listing workspaces");
    // TODO: Implement workspace listing
}

fn handle_workspace_rename(args: &ArgMatches) {
    let old_name = args.get_one::<String>("old-name").unwrap();
    let new_name = args.get_one::<String>("new-name").unwrap();
    println!("Renaming workspace from '{}' to '{}'", old_name, new_name);
    // TODO: Implement workspace renaming
}

async fn handle_crawl(sub_matches: &ArgMatches) {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    let url = sub_matches.get_one::<Url>("url").unwrap();
    let hosts_file = sub_matches.get_one::<std::path::PathBuf>("hosts-file");
    let threads = sub_matches.get_one::<usize>("threads").unwrap_or(&10);
    let auto_follow = sub_matches.get_flag("auto-follow");
    let no_follow = sub_matches.get_flag("no-follow");

    let url_str = url.as_str();
    let base_domain = url.host_str().unwrap_or("unknown");

    println!("\n🕷️  Crawling {}", base_domain);
    println!("Workers: {}", threads);
    println!("Max depth: 3");

    let follow_mode = if no_follow {
        "disabled (no cross-domain)"
    } else if auto_follow {
        "enabled (auto)"
    } else {
        "prompt on cross-domain"
    };
    println!("Cross-domain: {}\n", follow_mode);

    // Set up multi-progress
    let m = Arc::new(MultiProgress::new());
    let worker_bars: Arc<Mutex<HashMap<usize, ProgressBar>>> = Arc::new(Mutex::new(HashMap::new()));

    // Create progress bars for each worker
    for i in 0..*threads {
        let pb = m.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} Worker {msg}")
                .unwrap(),
        );
        pb.enable_steady_tick(Duration::from_millis(100));
        pb.set_message(format!("{}: idle", i));
        worker_bars.lock().await.insert(i, pb);
    }

    // Progress callback
    let worker_bars_clone = worker_bars.clone();
    let progress_callback = Arc::new(move |worker_id: usize, url: String| {
        let path = Url::parse(&url)
            .ok()
            .and_then(|u| {
                let path = u.path().to_string();
                if path.is_empty() || path == "/" {
                    Some("/".to_string())
                } else {
                    Some(path)
                }
            })
            .unwrap_or_else(|| url.clone());

        // Use try_lock to avoid blocking in async context
        if let Ok(bars) = worker_bars_clone.try_lock() {
            if let Some(pb) = bars.get(&worker_id) {
                pb.set_message(format!("{}: {}", worker_id, path));
            }
        }
    });

    // Cross-domain callback (changes behavior based on flags)
    let cross_domain_callback: rinzler_scanner::CrossDomainCallback = if no_follow {
        // No-follow mode: always reject cross-domain links
        Arc::new(|_url: String, _base: String| -> bool { false })
    } else if auto_follow {
        // Auto-follow mode: always accept cross-domain links
        Arc::new(|_url: String, _base: String| -> bool { true })
    } else {
        // Prompt mode: ask user and remember decisions
        // Track approved and denied cross-domains using std::sync::Mutex for blocking locks
        // This ensures atomicity across all workers
        let domain_decisions: Arc<StdMutex<(HashSet<String>, HashSet<String>)>> =
            Arc::new(StdMutex::new((HashSet::new(), HashSet::new())));

        let m_clone = m.clone();
        let domain_decisions_clone = domain_decisions.clone();
        Arc::new(move |url: String, _base: String| -> bool {
            let parsed = Url::parse(&url).ok();
            let domain = parsed.as_ref().and_then(|u| u.host_str()).unwrap_or("unknown").to_string();

            // Lock to check decisions atomically - this blocks if another worker is prompting
            let mut decisions = domain_decisions_clone.lock().unwrap();
            let (ref mut approved, ref mut denied) = *decisions;

            // Check if we've already made a decision for this domain
            if approved.contains(&domain) {
                return true;
            }
            if denied.contains(&domain) {
                return false;
            }

            // Not in either set - ask the user (while holding the lock to prevent duplicate prompts)
            let result = m_clone.suspend(|| {
                print!("\n⚠️  Cross-domain link detected: {}\nFollow this link? [y/N]: ", domain);
                io::stdout().flush().unwrap();

                let mut response = String::new();
                io::stdin().read_line(&mut response).unwrap();
                let response = response.trim().to_lowercase();

                response == "y" || response == "yes"
            });

            // Store the decision before releasing the lock
            if result {
                approved.insert(domain);
            } else {
                denied.insert(domain);
            }

            result
        })
    };

    // Create crawler with callbacks
    let crawler = Crawler::new()
        .with_max_depth(3)
        .with_auto_follow(false)  // We handle cross-domain logic in the callback now
        .with_progress_callback(progress_callback)
        .with_cross_domain_callback(cross_domain_callback);

    // Start crawl
    match crawler.crawl(url_str, *threads).await {
        Ok(results) => {
            // Clear all progress bars
            for (_, pb) in worker_bars.lock().await.iter() {
                pb.finish_and_clear();
            }
            m.clear().unwrap();

            println!("\n✓ Crawl complete!\n");

            // Build the report as a string
            let mut report = String::new();
            report.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");
            report.push_str("📊 Summary:\n");
            report.push_str(&format!("  Pages crawled: {}\n", results.len()));

            let total_links: usize = results.iter().map(|r| r.links_found.len()).sum();
            report.push_str(&format!("  Total links found: {}\n", total_links));

            let total_forms: usize = results.iter().map(|r| r.forms_found).sum();
            report.push_str(&format!("  Total forms found: {}\n", total_forms));

            let total_scripts: usize = results.iter().map(|r| r.scripts_found).sum();
            report.push_str(&format!("  Total scripts found: {}\n", total_scripts));

            // Group results by host
            let mut results_by_host: HashMap<String, Vec<&rinzler_scanner::result::CrawlResult>> = HashMap::new();
            for result in &results {
                if let Ok(parsed) = Url::parse(&result.url) {
                    let host = parsed.host_str().unwrap_or("unknown").to_string();
                    results_by_host.entry(host).or_insert_with(Vec::new).push(result);
                }
            }

            report.push_str("\n📄 Pages discovered:\n");
            for (host, host_results) in results_by_host.iter() {
                report.push_str(&format!("\n  {}\n", host));
                report.push_str(&format!("  {}\n", "─".repeat(host.len())));

                for result in host_results {
                    let (status_emoji, status_color) = match result.status_code {
                        100..=199 => ("ℹ", "\x1b[37m"),      // white
                        200..=299 => ("✓", "\x1b[32m"),      // green
                        300..=399 => ("↪", "\x1b[36m"),      // cyan
                        400..=499 => ("⚠", "\x1b[33m"),      // orange/yellow
                        500..=599 => ("✗", "\x1b[31m"),      // red
                        _ => ("?", "\x1b[37m"),              // white
                    };

                    // Extract just the path from the URL
                    let path = Url::parse(&result.url)
                        .ok()
                        .and_then(|u| {
                            let path = u.path().to_string();
                            if path.is_empty() || path == "/" {
                                Some("/".to_string())
                            } else {
                                Some(path)
                            }
                        })
                        .unwrap_or_else(|| result.url.clone());

                    // Only show content type if it's not text/html
                    let content_type_suffix = result.content_type
                        .as_ref()
                        .filter(|ct| !ct.contains("text/html"))
                        .map(|ct| format!("\x1b[90m  {}\x1b[0m", ct))
                        .unwrap_or_default();

                    report.push_str(&format!("  {} {}{}{} {}{}\n",
                        status_emoji,
                        status_color,
                        result.status_code,
                        "\x1b[0m",
                        path,
                        content_type_suffix
                    ));
                }
            }

            // Handle hosts file if provided
            if let Some(_hosts_file) = hosts_file {
                report.push_str("\nNote: Hosts file support not yet implemented\n");
            }

            // Display report in pager
            Pager::with_pager("less -R").setup();
            print!("{}", report);
        }
        Err(e) => {
            // Clear all progress bars
            for (_, pb) in worker_bars.lock().await.iter() {
                pb.finish_and_clear();
            }
            m.clear().unwrap();
            eprintln!("✗ Crawl failed: {}", e);
            std::process::exit(1);
        }
    }
}

async fn handle_fuzz(sub_matches: &ArgMatches) {
    let url = sub_matches.get_one::<Url>("url");
    let hosts_file = sub_matches.get_one::<std::path::PathBuf>("hosts-file");
    let wordlist_file = sub_matches.get_one::<std::path::PathBuf>("wordlist-file");
    let threads = sub_matches.get_one::<usize>("threads");

    if let Some(url) = url {
        println!("Fuzzing URL: {}", url);
    }
    if let Some(hosts_file) = hosts_file {
        println!("Fuzzing hosts from file: {}", hosts_file.display());
    }
    if let Some(wordlist_file) = wordlist_file {
        println!("Using wordlist: {}", wordlist_file.display());
    }
    if let Some(threads) = threads {
        println!("Using {} worker threads", threads);
    }
    // TODO: Implement fuzzing logic
    println!("Note: Fuzzing not yet implemented. Use crawl for now.");
}

fn handle_plugin_list() {
    println!("Listing plugins");
    // TODO: Implement plugin listing
}

fn handle_plugin_register(args: &ArgMatches) {
    let file = args.get_one::<std::path::PathBuf>("file").unwrap();
    let name = args.get_one::<String>("name").unwrap();
    println!(
        "Registering plugin '{}' from file: {}",
        name,
        file.display()
    );
    // TODO: Implement plugin registration
}

fn handle_plugin_unregister(args: &ArgMatches) {
    let name = args.get_one::<String>("name").unwrap();
    println!("Unregistering plugin: {}", name);
    // TODO: Implement plugin unregistration
}

pub const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);
