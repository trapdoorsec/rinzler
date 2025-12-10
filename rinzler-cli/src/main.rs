mod arguments;

use clap::Parser;
use arguments::Args;

fn main() {
    let args: Args = Args::parse();
    if args.sub_command.is_some() {
        println!("{}", args.sub_command.unwrap());
    }
}
