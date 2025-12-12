use clap::{arg, command};
use url::Url;

mod arguments;

fn main() {
    let cmd = clap::Command::new("rinzler")
        .version(env!("CARGO_PKG_VERSION"))
        .bin_name("rinzler")
        .styles(CLAP_STYLING)
        .arg(arg!(--"quiet").required(false))
        .subcommand_required(true)
        .subcommand(
            command!("scan")
                .arg(
                    arg!(--"url" -u <URL>)
                        .required(false)
                        .help("The IP address to scan")
                        .value_parser(clap::value_parser!(Url))
                        .default_value("http://127.0.0.1"),
                )
                .arg(
                    arg!(--"hosts-file" -H <PATH>)
                        .required(false)
                        .help("a line delimited list of hosts to scan")
                        .value_parser(clap::value_parser!(std::path::PathBuf)),
                ),
        );

    let matches = cmd.get_matches();
    let matches = match matches.subcommand() {
        Some(("scan", matches)) => matches,
        _ => unreachable!("clap should ensure we don't get here"),
    };

    let url = matches.get_one::<Url>("url");
    println!("URL: {url:?}");
}

pub const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);
