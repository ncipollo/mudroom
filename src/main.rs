use clap::Parser;
use mudroom::{cli::Cli, run};

fn main() {
    let cli = Cli::parse();
    println!("{}", run(cli));
}
