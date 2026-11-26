use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "barrel",
    version,
    about = "Register and track desktop app releases with Barrel."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// "text" for humans, "json" for scripts/CI/agents to parse.
    #[arg(long, value_enum, global = true, default_value = "text")]
    pub output: OutputFormat,
    /// Keep status output static, without terminal animation.
    #[arg(long, global = true)]
    pub no_motion: bool,
    /// Disable decorative colors, including in help output.
    #[arg(long, global = true)]
    pub no_color: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Release-related commands.
    Release(Box<ReleaseCmd>),
    /// Preview the sign-in experience. Creates no session; credentials remain env-var-only.
    Login,
}

#[derive(Args, Debug)]
pub struct ReleaseCmd {
    #[command(subcommand)]
    pub command: ReleaseCommands,
}

#[derive(Subcommand, Debug)]
pub enum ReleaseCommands {
    /// Register a release with Barrel's API for an already-built, already-signed, already-hosted artifact.
    Create(ReleaseCreateArgs),
}

#[derive(Args, Debug)]
pub struct ReleaseCreateArgs {
    /// Path to the artifact to checksum and register. Must already be signed and hosted.
    #[arg(long)]
    pub artifact: PathBuf,
    /// e.g. "electron", "tauri", "sparkle".
    #[arg(long)]
    pub platform: String,
    /// e.g. "x64", "arm64".
    #[arg(long)]
    pub arch: String,
    /// e.g. "zip", "dmg", "nsis", "`AppImage`".
    #[arg(long)]
    pub format: String,
    #[arg(long)]
    pub version: String,
    /// Falls back to `barrel.jsonc`'s `defaultChannel` when omitted.
    #[arg(long)]
    pub channel: Option<String>,
    /// Falls back to `barrel.jsonc`'s `projectId` when omitted.
    #[arg(long = "project-id")]
    pub project_id: Option<String>,
    /// Where the developer's own CI/CD already hosts this artifact. Barrel never uploads it.
    #[arg(long)]
    pub url: String,
    #[arg(long = "release-notes", conflicts_with = "release_notes_file")]
    pub release_notes: Option<String>,
    #[arg(long = "release-notes-file", conflicts_with = "release_notes")]
    pub release_notes_file: Option<PathBuf>,
    /// Overrides the auto-generated idempotency key passed on the request.
    #[arg(long = "idempotency-key")]
    pub idempotency_key: Option<String>,
    /// Path to `barrel.jsonc`. Defaults to `./barrel.jsonc`.
    #[arg(long, default_value = "barrel.jsonc")]
    pub config: PathBuf,
    /// Validate everything (config, credential, artifact, checksum, payload
    /// shape) without actually calling Barrel's API.
    #[arg(long = "dry-run")]
    pub dry_run: bool,
}
