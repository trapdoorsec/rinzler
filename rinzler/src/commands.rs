use crate::CLAP_STYLING;
use clap::{arg, command};
use url::Url;

pub(crate) fn command_argument_builder() -> clap::Command {
    clap::Command::new("rinzler")
        .version(env!("CARGO_PKG_VERSION"))
        .bin_name("rinzler")
        .styles(CLAP_STYLING)
        .arg(arg!(-q --"quiet" "Suppress banner and non-essential output").required(false))
        .subcommand_required(false)
        .subcommand(
            command!("init")
                .about("Initializes the rinzler database on your filesystem")
                .arg(
                    arg!([PATH])
                        .required(false)
                        .help("Location to store the rinzler database")
                        .default_value("~/.config/rinzler/"),
                )
                .arg(
                    arg!(-f - -"force")
                        .help(
                            "Forces the overwriting of any existing database at the specified \
                        location.",
                        )
                        .required(false),
                ),
        )
        .subcommand(
            command!("workspace")
                .about("Manage rinzler workspaces")
                .subcommand(
                    command!("create").about("Creates a workspace").arg(
                        arg!(-n --"name" <NAME>)
                            .required(true)
                            .help("The name of the workspace"),
                    ),
                )
                .subcommand(
                    command!("remove").about("Removes the workspace").arg(
                        arg!(-n --"name" <NAME>)
                            .required(true)
                            .help("The name of the workspace"),
                    ),
                )
                .subcommand(command!("list").about("List all workspaces"))
                .subcommand(
                    command!("rename")
                        .about("Renames the workspace")
                        .arg(
                            arg!(--"old-name" <NAME>)
                                .required(true)
                                .help("The current name of the workspace"),
                        )
                        .arg(
                            arg!(--"new-name" <NAME>)
                                .required(true)
                                .help("The new name for the workspace"),
                        ),
                ),
        )
        .subcommand(
            command!("crawl")
                .about(
                    "Passively crawl a host or collection of hosts. Contributes findings to the \
                map.",
                )
                .arg(
                    arg!(-u --"url" <URL>)
                        .required(false)
                        .help("The URL to crawl")
                        .value_parser(clap::value_parser!(Url))
                        .conflicts_with("hosts-file"),
                )
                .arg(
                    arg!(-H --"hosts-file" <PATH>)
                        .required(false)
                        .help("Path to a newline-delimited file of URLs to crawl")
                        .value_parser(clap::value_parser!(std::path::PathBuf))
                        .conflicts_with("url"),
                )
                .arg(
                    arg!(-t --"threads" <NUM_WORKERS>)
                        .required(false)
                        .help("The number of async worker 'threads' in the worker pool.")
                        .value_parser(clap::value_parser!(usize))
                        .default_value("10"),
                )
                .arg(
                    arg!(--"follow")
                        .required(false)
                        .help("Follow cross-domain links with user prompts (default: stay on same domain)")
                        .action(clap::ArgAction::SetTrue)
                        .conflicts_with("auto-follow"),
                )
                .arg(
                    arg!(--"auto-follow")
                        .required(false)
                        .help("Automatically follow all cross-domain links without prompting")
                        .action(clap::ArgAction::SetTrue)
                        .conflicts_with("follow"),
                )
                .arg(
                    arg!(-o --"output" <PATH>)
                        .required(false)
                        .help("Save report to file (default: display to screen)")
                        .value_parser(clap::value_parser!(std::path::PathBuf)),
                )
                .arg(
                    arg!(-f --"format" <FORMAT>)
                        .required(false)
                        .help("Report format: text, json, csv, html, markdown")
                        .value_parser(["text", "json", "csv", "html", "markdown"])
                        .default_value("text"),
                )
                .arg(
                    arg!(--"include-sitemap")
                        .required(false)
                        .help("Include a visual sitemap tree in the report")
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .subcommand(
            command!("fuzz")
                .about(
                    "Actively fuzz a host or collection of hosts, using a forced browsing and \
                dictionary based techniques. Contributes findings to the map.",
                )
                .arg(
                    arg!(-u --"url" <URL>)
                        .required(false)
                        .help("The IP address to scan")
                        .value_parser(clap::value_parser!(Url))
                        .default_value("http://127.0.0.1"),
                )
                .arg(
                    arg!(-H --"hosts-file" <PATH>)
                        .required(false)
                        .help("a line delimited list of hosts to scan")
                        .value_parser(clap::value_parser!(std::path::PathBuf)),
                )
                .arg(
                    arg!(-w --"wordlist-file" <PATH>)
                        .required(false)
                        .help("Path to wordlist file (default: ~/.config/rinzler/wordlists/default.txt)")
                        .value_parser(clap::value_parser!(std::path::PathBuf)),
                )
                .arg(
                    arg!(-t --"threads" <NUM_WORKERS>)
                        .required(false)
                        .help("The number of async worker 'threads' in the worker pool.")
                        .value_parser(clap::value_parser!(usize))
                        .default_value("10"),
                )
                .arg(
                    arg!(--"full-body")
                        .required(false)
                        .help("Use GET requests to download full response bodies (default: HEAD requests)")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    arg!(--"timeout" <SECONDS>)
                        .required(false)
                        .help("Request timeout in seconds")
                        .value_parser(clap::value_parser!(u64))
                        .default_value("5"),
                ),
        )
        .subcommand(
            command!("plugin")
                .about("Manage rinzler plugins")
                .subcommand(command!("list").about("List all registered plugins"))
                .subcommand(
                    command!("register")
                        .about("Register a plugin")
                        .arg(
                            arg!(-f --"file" <PATH>)
                                .required(true)
                                .help("The path of the plugin file to register")
                                .value_parser(clap::value_parser!(std::path::PathBuf)),
                        )
                        .arg(
                            arg!(-n --"name" <NAME>)
                                .required(true)
                                .help("The name of the plugin"),
                        ),
                )
                .subcommand(
                    command!("unregister").about("Unregister a plugin").arg(
                        arg!(-n --"name" <NAME>)
                            .required(true)
                            .help("The name of the plugin"),
                    ),
                ),
        )
}
