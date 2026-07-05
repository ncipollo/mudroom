use clap::CommandFactory;
use clap_complete::{Shell, generate};

use super::Cli;

pub fn run(shell: Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "mudroom", &mut std::io::stdout());
}
