use crate::api_client::{self, ReleaseApiClient};
use crate::cli::{OutputFormat, ReleaseCreateArgs};
use crate::config::BarrelConfig;
use crate::terminal::Terminal;
use crate::{RunError, checksum};
use serde::Serialize;

fn resolve_release_notes(args: &ReleaseCreateArgs) -> Result<String, RunError> {
    if let Some(path) = &args.release_notes_file {
        return Ok(std::fs::read_to_string(path)?);
    }
    Ok(args.release_notes.clone().unwrap_or_default())
}

fn generate_idempotency_key() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// What `--dry-run` reports instead of calling the API -- everything that
/// would be sent, minus the credential, which is never printed.
#[derive(Serialize)]
struct DryRunPreview<'a> {
    #[serde(rename = "projectId")]
    project_id: &'a str,
    #[serde(rename = "dryRun")]
    dry_run: bool,
    payload: &'a api_client::CreateReleasePayload,
}

/// Computes the artifact's checksum and makes one call to Barrel's
/// release-creation API. `platform`/`arch`/`format` are forwarded exactly as
/// given -- this function has no framework-specific knowledge of any kind.
pub fn create_release(
    api: &dyn ReleaseApiClient,
    config: &BarrelConfig,
    credential: String,
    args: ReleaseCreateArgs,
    output: OutputFormat,
    terminal: Terminal,
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
    let dry_run = args.dry_run;

    let sha512 = terminal.task("Calculate artifact checksum", || {
        checksum::sha512_base64(&args.artifact)
    })?;
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

    if dry_run {
        let preview = DryRunPreview {
            project_id: &request.project_id,
            dry_run: true,
            payload: &request.payload,
        };
        println!("{}", render_dry_run(&preview, output)?);
        return Ok(());
    }

    let release = terminal.task("Register release", || api.create_release(&request))?;
    println!("{}", render_success(&release, &request, output)?);
    Ok(())
}

fn render_dry_run(preview: &DryRunPreview, output: OutputFormat) -> Result<String, RunError> {
    match output {
        OutputFormat::Json => Ok(serde_json::to_string(preview)?),
        OutputFormat::Text => Ok(format!(
            "Dry run: would register version {} on channel {} for project {} \
             ({} artifact, {} byte(s)). Nothing was sent.\n\
             Next: drop --dry-run to actually register this release.",
            preview.payload.version,
            preview.payload.channel_id,
            preview.project_id,
            preview.payload.artifacts[0].format,
            preview.payload.artifacts[0].size_bytes
        )),
    }
}

fn render_success(
    release: &api_client::Release,
    request: &api_client::CreateReleaseRequest,
    output: OutputFormat,
) -> Result<String, RunError> {
    match output {
        OutputFormat::Json => Ok(serde_json::to_string(release)?),
        OutputFormat::Text => Ok(format!(
            "Created release {} ({})\n{}",
            release.version,
            release.id,
            next_command_hint(request)
        )),
    }
}

/// Suggests the next command a developer would plausibly run after a real
/// release is created: registering another platform's artifact for it,
/// with the values that carry over filled in.
fn next_command_hint(request: &api_client::CreateReleaseRequest) -> String {
    format!(
        "Next: registering another platform's artifact for this same release? Run \
         `barrel release create --version {} --channel {} --project-id {} \
         --artifact <path> --platform <platform> --arch <arch> --format <format> --url <url>`.",
        request.payload.version, request.payload.channel_id, request.project_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_client::fake::FakeReleaseApiClient;
    use crate::terminal;
    use std::io::Write;

    fn create_release(
        api: &dyn ReleaseApiClient,
        config: &BarrelConfig,
        credential: String,
        args: ReleaseCreateArgs,
        output: OutputFormat,
    ) -> Result<(), RunError> {
        let terminal = terminal::Terminal::detect(terminal::Options {
            output,
            no_color: true,
            no_motion: true,
        });
        super::create_release(api, config, credential, args, output, terminal)
    }

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
            dry_run: false,
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
            OutputFormat::Text,
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
            OutputFormat::Text,
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

        create_release(
            &api,
            &config,
            "brl_app_test".to_string(),
            input,
            OutputFormat::Text,
        )
        .unwrap();

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
            OutputFormat::Text,
        );

        assert!(result.is_err());
        assert_eq!(api.requests.borrow().len(), 0);
    }

    #[test]
    fn dry_run_validates_everything_but_never_calls_the_api() {
        let artifact = write_fake_artifact();
        let api = FakeReleaseApiClient::default();

        let mut input = args(artifact.path());
        input.dry_run = true;

        create_release(
            &api,
            &BarrelConfig::default(),
            "brl_app_test".to_string(),
            input,
            OutputFormat::Text,
        )
        .unwrap();

        assert_eq!(api.requests.borrow().len(), 0);
    }

    #[test]
    fn dry_run_still_fails_on_bad_config_without_calling_the_api() {
        let artifact = write_fake_artifact();
        let api = FakeReleaseApiClient::default();

        let mut input = args(artifact.path());
        input.dry_run = true;
        input.project_id = None;

        let result = create_release(
            &api,
            &BarrelConfig::default(),
            "brl_app_test".to_string(),
            input,
            OutputFormat::Text,
        );

        assert!(result.is_err());
        assert_eq!(api.requests.borrow().len(), 0);
    }

    #[test]
    fn next_command_hint_carries_over_version_channel_and_project_id() {
        let request = api_client::CreateReleaseRequest {
            project_id: "proj_123".to_string(),
            credential: "brl_app_test".to_string(),
            idempotency_key: None,
            payload: api_client::CreateReleasePayload {
                channel_id: "stable".to_string(),
                version: "1.2.3".to_string(),
                release_notes: String::new(),
                artifacts: vec![],
            },
        };

        let hint = next_command_hint(&request);
        assert!(hint.contains("--version 1.2.3"));
        assert!(hint.contains("--channel stable"));
        assert!(hint.contains("--project-id proj_123"));
    }

    fn sample_payload() -> api_client::CreateReleasePayload {
        api_client::CreateReleasePayload {
            channel_id: "stable".to_string(),
            version: "1.2.3".to_string(),
            release_notes: String::new(),
            artifacts: vec![api_client::ReleaseArtifactInput {
                filename: "MyApp.dmg".to_string(),
                platform: "sparkle".to_string(),
                architecture: "arm64".to_string(),
                format: "dmg".to_string(),
                size_bytes: 42,
                url: "https://cdn.example.test/MyApp.dmg".to_string(),
                integrity: api_client::ArtifactIntegrity::Checksum {
                    algorithm: "sha512".to_string(),
                    value: "abc".to_string(),
                },
            }],
        }
    }

    #[test]
    fn render_dry_run_json_never_contains_the_credential() {
        let payload = sample_payload();
        let preview = DryRunPreview {
            project_id: "proj_123",
            dry_run: true,
            payload: &payload,
        };

        let rendered = render_dry_run(&preview, OutputFormat::Json).unwrap();
        assert!(!rendered.contains("brl_app_test"));
        assert!(rendered.contains("\"projectId\":\"proj_123\""));
    }

    #[test]
    fn render_dry_run_text_never_contains_the_credential_and_suggests_dropping_the_flag() {
        let payload = sample_payload();
        let preview = DryRunPreview {
            project_id: "proj_123",
            dry_run: true,
            payload: &payload,
        };

        let rendered = render_dry_run(&preview, OutputFormat::Text).unwrap();
        assert!(!rendered.contains("brl_app_test"));
        assert!(rendered.contains("Nothing was sent"));
        assert!(rendered.contains("drop --dry-run"));
    }

    #[test]
    fn render_success_json_contains_only_the_release_no_human_text() {
        let release = api_client::Release {
            id: "rel_1".to_string(),
            channel_id: "stable".to_string(),
            version: "1.2.3".to_string(),
            release_notes: String::new(),
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
        };
        let request = api_client::CreateReleaseRequest {
            project_id: "proj_123".to_string(),
            credential: "brl_app_test".to_string(),
            idempotency_key: None,
            payload: api_client::CreateReleasePayload {
                channel_id: "stable".to_string(),
                version: "1.2.3".to_string(),
                release_notes: String::new(),
                artifacts: vec![],
            },
        };

        let rendered = render_success(&release, &request, OutputFormat::Json).unwrap();
        assert!(!rendered.contains("Created release"));
        assert!(!rendered.contains("Next:"));
        assert!(!rendered.contains("brl_app_test"));
        let parsed: api_client::Release = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed.id, "rel_1");
    }
}
