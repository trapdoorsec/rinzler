use commands::command_argument_builder;
use rinzler::handlers;
use rinzler_core::print_banner;

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
        Some(("init", primary_command)) => handlers::handle_init(primary_command),
        Some(("workspace", primary_command)) => match primary_command.subcommand() {
            Some(("create", secondary_command)) => handlers::handle_workspace_create(secondary_command),
            Some(("remove", secondary_command)) => handlers::handle_workspace_remove(secondary_command),
            Some(("list", _)) => handlers::handle_workspace_list(),
            Some(("rename", secondary_command)) => handlers::handle_workspace_rename(secondary_command),
            _ => unreachable!("clap should ensure we don't get here"),
        },
        Some(("crawl", primary_command)) => handlers::handle_crawl(primary_command).await,
        Some(("fuzz", primary_command)) => handlers::handle_fuzz(primary_command).await,
        Some(("plugin", primary_command)) => match primary_command.subcommand() {
            Some(("list", _)) => handlers::handle_plugin_list(),
            Some(("register", secondary_command)) => handlers::handle_plugin_register(secondary_command),
            Some(("unregister", secondary_command)) => handlers::handle_plugin_unregister(secondary_command),
            _ => unreachable!("clap should ensure we don't get here"),
        },
        _ => unreachable!("clap should ensure we don't get here"),
    }
}

pub const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);
