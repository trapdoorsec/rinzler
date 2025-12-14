use clap::ArgMatches;
use colored::Colorize;
use pager::Pager;
use rinzler_core::data::Database;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
        show_progress_bars: true, // Enable progress bars in CLI mode
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
