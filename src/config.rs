use serde::Deserialize;
use std::path::Path;

/// `barrel.jsonc`'s own shape: settings a developer commits, each also
/// flag-overridable at the CLI layer. The credential is deliberately
/// absent -- it's env-var-only, never read from or written to this file.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct BarrelConfig {
    #[serde(rename = "projectId")]
    pub project_id: Option<String>,
    #[serde(rename = "defaultChannel")]
    pub default_channel: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {path} as JSONC: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

/// Loads `barrel.jsonc` if it exists; an absent file is not an error -- a
/// developer invoking every flag explicitly needs no committed file at all.
/// No "current project" state is persisted anywhere by this CLI.
pub fn load(path: &Path) -> Result<BarrelConfig, ConfigError> {
    if !path.exists() {
        return Ok(BarrelConfig::default());
    }

    let file = std::fs::File::open(path).map_err(|source| ConfigError::Read {
        path: path.display().to_string(),
        source,
    })?;

    let stripped = json_comments::StripComments::new(file);
    serde_json::from_reader(stripped).map_err(|source| ConfigError::Parse {
        path: path.display().to_string(),
        source,
    })
}

/// The name of the environment variable `barrel` reads its credential from.
/// Never read from `barrel.jsonc`, and never persisted to disk.
pub const CREDENTIAL_ENV_VAR: &str = "BARREL_CREDENTIAL";

pub fn read_credential() -> Result<String, std::env::VarError> {
    std::env::var(CREDENTIAL_ENV_VAR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_returns_defaults_when_the_file_does_not_exist() {
        let config = load(Path::new("/nonexistent/barrel.jsonc")).unwrap();
        assert_eq!(config.project_id, None);
        assert_eq!(config.default_channel, None);
    }

    #[test]
    fn load_strips_comments_before_parsing() {
        let mut file = tempfile::Builder::new()
            .suffix(".jsonc")
            .tempfile()
            .unwrap();
        write!(
            file,
            r#"{{
                // a committed default, overridable via --project-id
                "projectId": "proj_123",
                "defaultChannel": "stable" /* inline comment */
            }}"#
        )
        .unwrap();

        let config = load(file.path()).unwrap();
        assert_eq!(config.project_id, Some("proj_123".to_string()));
        assert_eq!(config.default_channel, Some("stable".to_string()));
    }
}
