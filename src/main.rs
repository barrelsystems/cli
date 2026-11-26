use barrel_cli::cli::Cli;
use clap::Parser;

fn main() {
    let cli = Cli::parse();
    let exit_code = barrel_cli::run(cli);
    std::process::exit(exit_code.into());
}
