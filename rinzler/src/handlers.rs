use clap::ArgMatches;
use indicatif::{ProgressBar, ProgressStyle};
use pager::Pager;
use rinzler_core::data::Database;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tracing_subscriber;
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
pub use rinzler_core::crawl::{execute_crawl, extract_url_path, generate_crawl_report, CrawlOptions, CrawlProgressCallback, FollowMode};

pub fn handle_init(args: &ArgMatches) {
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
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    let url = sub_matches.get_one::<Url>("url");
    let hosts_file = sub_matches.get_one::<std::path::PathBuf>("hosts-file");
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

    // Create crawl options
    let options = CrawlOptions {
        urls,
        threads,
        max_depth: 3,
        follow_mode,
        show_progress_bars: true,  // Enable progress bars in CLI mode
    };

    // Execute crawl with progress callback
    let progress_callback = Arc::new(|msg: String| {
        println!("{}", msg);
    });

    let all_results = match execute_crawl(options, Some(progress_callback)).await {
        Ok(results) => results,
        Err(e) => {
            eprintln!("✗ Crawl failed: {}", e);
            std::process::exit(1);
        }
    };

    println!("\n✓ Crawl complete!\n");

    // Generate and display report
    let report = generate_crawl_report(&all_results);
    Pager::with_pager("less -R").setup();
    print!("{}", report);
}

pub async fn handle_fuzz(sub_matches: &ArgMatches) {
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

pub fn handle_plugin_list() {
    println!("Listing plugins");
    // TODO: Implement plugin listing
}

pub fn handle_plugin_register(args: &ArgMatches) {
    let file = args.get_one::<std::path::PathBuf>("file").unwrap();
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
