use clap::Parser;
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub(crate) struct Args {
    #[arg(short, long)]
    pub sub_command: Option<String>,
}