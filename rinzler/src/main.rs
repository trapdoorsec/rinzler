use clap;
use clap::ArgMatches;
use commands::command_argument_builder;
use indicatif::{ProgressBar, ProgressStyle};
use rinzler_core::{data::Database, print_banner};
use rinzler_scanner::Crawler;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;
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

    let url_str = url.as_str();

    println!("\n🕷️  Starting passive crawl of {}", url_str);
    println!("Workers: {}", threads);
    println!("Max depth: 3");
    println!("Max pages: 100\n");

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner.set_message("Initializing crawler...");

    // Create crawler
    let crawler = Crawler::new()
        .with_max_depth(3)
        .with_max_pages(100);

    spinner.set_message(format!("Crawling {}...", url_str));

    // Start crawl
    match crawler.crawl(url_str, *threads).await {
        Ok(results) => {
            spinner.finish_and_clear();

            println!("\n✓ Crawl complete!");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

            println!("📊 Summary:");
            println!("  Pages crawled: {}", results.len());

            let total_links: usize = results.iter().map(|r| r.links_found.len()).sum();
            println!("  Total links found: {}", total_links);

            let total_forms: usize = results.iter().map(|r| r.forms_found).sum();
            println!("  Total forms found: {}", total_forms);

            let total_scripts: usize = results.iter().map(|r| r.scripts_found).sum();
            println!("  Total scripts found: {}", total_scripts);

            println!("\n📄 Pages discovered:");
            for result in &results {
                let status_emoji = match result.status_code {
                    200..=299 => "✓",
                    300..=399 => "↪",
                    400..=499 => "⚠",
                    500..=599 => "✗",
                    _ => "?",
                };

                let content_type = result.content_type.as_deref().unwrap_or("unknown");
                println!("  {} {} [{}] {}", status_emoji, result.status_code, content_type, result.url);
            }

            // Handle hosts file if provided
            if let Some(_hosts_file) = hosts_file {
                println!("\nNote: Hosts file support not yet implemented");
            }
        }
        Err(e) => {
            spinner.finish_and_clear();
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
