pub mod api_client;
pub mod checksum;
pub mod cli;
pub mod config;
pub mod exit_code;

use api_client::{HttpReleaseApiClient, ReleaseApiClient};
use cli::{Cli, Commands, ReleaseCommands, ReleaseCreateArgs};
use config::BarrelConfig;
use exit_code::ExitCode;

/// The real backend base URL. Overridable via `BARREL_API_BASE_URL` for
/// local/staging use -- never hardcoded as the only option, since this CLI
/// is a public repo that must also work against non-production backends.
const DEFAULT_API_BASE_URL: &str = "https://api.barrel.dev";

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
}

impl RunError {
    fn exit_code(&self) -> ExitCode {
        match self {
            RunError::Config(_) => ExitCode::ConfigValidation,
            RunError::Api(_) => ExitCode::ApiNetwork,
            RunError::Io(_) => ExitCode::Generic,
        }
    }
}

fn api_base_url() -> String {
    std::env::var("BARREL_API_BASE_URL").unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_string())
}

fn resolve_release_notes(args: &ReleaseCreateArgs) -> Result<String, RunError> {
    if let Some(path) = &args.release_notes_file {
        return Ok(std::fs::read_to_string(path)?);
    }
    Ok(args.release_notes.clone().unwrap_or_default())
}

fn generate_idempotency_key() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn run_inner(cli: Cli) -> Result<(), RunError> {
    match cli.command {
        Commands::Release(cmd) => match cmd.command {
            ReleaseCommands::Create(args) => {
                let config =
                    config::load(&args.config).map_err(|e| RunError::Config(e.to_string()))?;
                let credential = config::read_credential().map_err(|_| {
                    RunError::Config(format!(
                        "{} is not set -- the CLI reads its credential only from this environment variable",
                        config::CREDENTIAL_ENV_VAR
                    ))
                })?;

                let api = HttpReleaseApiClient::new(api_base_url());
                create_release(&api, &config, credential, args)
            }
        },
    }
}

/// Computes the artifact's checksum and makes one call to Barrel's
/// release-creation API. `platform`/`arch`/`format` are forwarded exactly as
/// given -- this function has no framework-specific knowledge of any kind.
fn create_release(
    api: &dyn ReleaseApiClient,
    config: &BarrelConfig,
    credential: String,
    args: ReleaseCreateArgs,
) -> Result<(), RunError> {
    let project_id = args
        .project_id
        .clone()
        .or_else(|| config.project_id.clone())
        .ok_or_else(|| {
            RunError::Config("no --project-id given and barrel.jsonc has no projectId".to_string())
        })?;

    let channel_id = args
        .channel
        .clone()
        .or_else(|| config.default_channel.clone())
        .ok_or_else(|| {
            RunError::Config(
                "no --channel given and barrel.jsonc has no defaultChannel".to_string(),
            )
        })?;

    let release_notes = resolve_release_notes(&args)?;
    let idempotency_key = args
        .idempotency_key
        .clone()
        .unwrap_or_else(generate_idempotency_key);

    let sha512 = checksum::sha512_base64(&args.artifact)?;
    let size = checksum::file_size(&args.artifact)?;
    let filename = args
        .artifact
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let request = api_client::CreateReleaseRequest {
        project_id,
        credential,
        idempotency_key: Some(idempotency_key),
        payload: api_client::CreateReleasePayload {
            channel_id,
            version: args.version,
            release_notes,
            artifacts: vec![api_client::ReleaseArtifactInput {
                filename,
                platform: args.platform,
                architecture: args.arch,
                format: args.format,
                size_bytes: size,
                url: args.url,
                integrity: api_client::ArtifactIntegrity::Checksum {
                    algorithm: checksum::ALGORITHM.to_string(),
                    value: sha512,
                },
            }],
        },
    };

    let release = api.create_release(&request)?;
    println!("Created release {} ({})", release.version, release.id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use api_client::fake::FakeReleaseApiClient;
    use std::io::Write;

    fn write_fake_artifact() -> tempfile::NamedTempFile {
        let mut file = tempfile::Builder::new().suffix(".zip").tempfile().unwrap();
        file.write_all(b"fake artifact bytes").unwrap();
        file
    }

    fn args(artifact: &std::path::Path) -> ReleaseCreateArgs {
        ReleaseCreateArgs {
            artifact: artifact.to_path_buf(),
            platform: "tauri".to_string(),
            arch: "arm64".to_string(),
            format: "dmg".to_string(),
            version: "1.2.3".to_string(),
            channel: Some("stable".to_string()),
            project_id: Some("proj_123".to_string()),
            url: "https://cdn.example.test/MyApp-1.2.3.dmg".to_string(),
            release_notes: Some("Bug fixes".to_string()),
            release_notes_file: None,
            idempotency_key: Some("idem_abc".to_string()),
            config: std::path::PathBuf::from("nonexistent-barrel.jsonc"),
        }
    }

    #[test]
    fn create_release_forwards_platform_arch_and_format_exactly_as_given() {
        let artifact = write_fake_artifact();
        let api = FakeReleaseApiClient::default();

        create_release(
            &api,
            &BarrelConfig::default(),
            "brl_app_test".to_string(),
            args(artifact.path()),
        )
        .unwrap();

        let requests = api.requests.borrow();
        assert_eq!(requests.len(), 1);
        let artifact_input = &requests[0].payload.artifacts[0];
        assert_eq!(artifact_input.platform, "tauri");
        assert_eq!(artifact_input.architecture, "arm64");
        assert_eq!(artifact_input.format, "dmg");
    }

    #[test]
    fn create_release_passes_the_idempotency_key_on_the_request() {
        let artifact = write_fake_artifact();
        let api = FakeReleaseApiClient::default();

        create_release(
            &api,
            &BarrelConfig::default(),
            "brl_app_test".to_string(),
            args(artifact.path()),
        )
        .unwrap();

        let requests = api.requests.borrow();
        assert_eq!(requests[0].idempotency_key.as_deref(), Some("idem_abc"));
    }

    #[test]
    fn create_release_falls_back_to_config_when_flags_are_omitted() {
        let artifact = write_fake_artifact();
        let api = FakeReleaseApiClient::default();
        let config = BarrelConfig {
            project_id: Some("proj_from_config".to_string()),
            default_channel: Some("beta".to_string()),
        };

        let mut input = args(artifact.path());
        input.project_id = None;
        input.channel = None;

        create_release(&api, &config, "brl_app_test".to_string(), input).unwrap();

        let requests = api.requests.borrow();
        assert_eq!(requests[0].project_id, "proj_from_config");
        assert_eq!(requests[0].payload.channel_id, "beta");
    }

    #[test]
    fn create_release_fails_without_calling_the_api_when_no_project_id_is_available() {
        let artifact = write_fake_artifact();
        let api = FakeReleaseApiClient::default();

        let mut input = args(artifact.path());
        input.project_id = None;

        let result = create_release(
            &api,
            &BarrelConfig::default(),
            "brl_app_test".to_string(),
            input,
        );

        assert!(result.is_err());
        assert_eq!(api.requests.borrow().len(), 0);
    }
}
