use barrel_cli::cli::Cli;
use clap::error::{ContextKind, ContextValue};

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    let no_color = args.iter().any(|arg| arg == "--no-color")
        || std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
    let parser = <Cli as clap::CommandFactory>::command();
    let parser = if no_color {
        parser.color(clap::ColorChoice::Never)
    } else {
        parser
    };
    let parsed = parser
        .try_get_matches_from(args)
        .and_then(|matches| <Cli as clap::FromArgMatches>::from_arg_matches(&matches));
    match parsed {
        Ok(cli) => {
            let exit_code = barrel_cli::run(cli);
            std::process::exit(exit_code.into());
        }
        Err(error) => {
            let hint = invalid_subcommand(&error).and_then(barrel_cli::command_hint);

            let _ = error.print();
            if let Some(right_command) = hint {
                eprintln!("Hint: You probably want `{right_command}`.");
            }
            std::process::exit(error.exit_code());
        }
    }
}

/// The subcommand the user actually typed, if this error is clap rejecting
/// an unrecognized one.
fn invalid_subcommand(error: &clap::Error) -> Option<&str> {
    error
        .context()
        .find_map(|(kind, value)| match (kind, value) {
            (ContextKind::InvalidSubcommand, ContextValue::String(s)) => Some(s.as_str()),
            _ => None,
        })
}
