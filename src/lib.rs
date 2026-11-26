pub mod api_client;
pub mod checksum;
pub mod cli;
mod command_hint;
pub mod config;
pub mod exit_code;
pub mod login;
mod release;
mod terminal;

pub use command_hint::command_hint;

use cli::{Cli, Commands, OutputFormat, ReleaseCommands};
use exit_code::ExitCode;

/// The real backend base URL. Overridable via `BARREL_API_BASE_URL` for
/// local/staging use -- never hardcoded as the only option, since this CLI
/// is a public repo that must also work against non-production backends.
const DEFAULT_API_BASE_URL: &str = "https://terminal.barrel.codes";

#[must_use]
pub fn run(cli: Cli) -> ExitCode {
    match run_inner(cli) {
        Ok(()) => ExitCode::Success,
        Err(err) => {
            eprintln!("error: {err:#}");
            err.exit_code()
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("{0}")]
    Config(String),
    #[error(transparent)]
    Api(#[from] api_client::ApiClientError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("failed to serialize output as JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl RunError {
    const fn exit_code(&self) -> ExitCode {
        match self {
            Self::Config(_) => ExitCode::ConfigValidation,
            Self::Api(_) => ExitCode::ApiNetwork,
            Self::Io(_) | Self::Json(_) => ExitCode::Generic,
        }
    }
}

fn api_base_url() -> String {
    std::env::var("BARREL_API_BASE_URL").unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_string())
}

const QUICKSTART: &str = "\
barrel registers a release with Barrel for an artifact you've already built, signed, and hosted.

Quickstart:
  barrel release create --artifact <path> --platform <platform> --arch <arch>
    --format <format> --version <version> --url <url> --project-id <id> --channel <channel>

Run `barrel --help` for the full command reference.";

fn run_inner(cli: Cli) -> Result<(), RunError> {
    let output = cli.output;
    let terminal = terminal::Terminal::detect(terminal::Options {
        output,
        no_color: cli.no_color,
        no_motion: cli.no_motion,
    });
    match cli.command {
        None => {
            if output == OutputFormat::Json {
                println!(
                    "{}",
                    serde_json::json!({"name": "barrel", "version": env!("CARGO_PKG_VERSION"), "commands": ["release create", "login"]})
                );
            } else {
                terminal.introduction("Release control", concat!("v", env!("CARGO_PKG_VERSION")));
                println!("{QUICKSTART}");
            }
            Ok(())
        }
        Some(Commands::Login) => {
            login::run(terminal, output);
            Ok(())
        }
        Some(Commands::Release(cmd)) => match cmd.command {
            ReleaseCommands::Create(args) => {
                let config =
                    config::load(&args.config).map_err(|e| RunError::Config(e.to_string()))?;
                let credential = config::read_credential().map_err(|_| {
                    RunError::Config(format!(
                        "{} is not set -- the CLI reads its credential only from this environment variable",
                        config::CREDENTIAL_ENV_VAR
                    ))
                })?;

                let api = api_client::HttpReleaseApiClient::new(api_base_url());
                terminal.heading(
                    "Release",
                    &format!(
                        "{} / {}",
                        args.version,
                        args.channel
                            .as_deref()
                            .or(config.default_channel.as_deref())
                            .unwrap_or("channel not set")
                    ),
                );
                terminal.note("Your artifact stays at its existing URL.\n");
                release::create_release(&api, &config, credential, args, output, terminal)
            }
        },
    }
}
