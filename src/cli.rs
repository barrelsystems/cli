use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "barrel",
    version,
    about = "Register a release with Barrel for an artifact you've already built, signed, and hosted."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Release-related commands.
    Release(ReleaseCmd),
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
    /// e.g. "zip", "dmg", "nsis", "AppImage".
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
}
