use serde::{Deserialize, Serialize};

/// `POST /projects/:projectId/releases`, authenticated by
/// `Authorization: Bearer <application credential>` plus an optional
/// `idempotency-key` header. The only step that ever calls Barrel's API.
pub trait ReleaseApiClient {
    fn create_release(&self, request: &CreateReleaseRequest) -> Result<Release, ApiClientError>;
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum ArtifactIntegrity {
    #[serde(rename = "checksum")]
    Checksum { algorithm: String, value: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseArtifactInput {
    pub filename: String,
    pub platform: String,
    pub architecture: String,
    pub format: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: u64,
    pub url: String,
    pub integrity: ArtifactIntegrity,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateReleasePayload {
    #[serde(rename = "channelId")]
    pub channel_id: String,
    pub version: String,
    #[serde(rename = "releaseNotes")]
    pub release_notes: String,
    pub artifacts: Vec<ReleaseArtifactInput>,
}

#[derive(Debug, Clone)]
pub struct CreateReleaseRequest {
    pub project_id: String,
    /// The raw `brl_app_...` credential; the client adds the `Bearer ` prefix.
    pub credential: String,
    pub idempotency_key: Option<String>,
    pub payload: CreateReleasePayload,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub id: String,
    #[serde(rename = "channelId")]
    pub channel_id: String,
    pub version: String,
    #[serde(rename = "releaseNotes")]
    pub release_notes: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiClientError {
    #[error("network error calling the release-creation API: {0}")]
    Network(#[from] reqwest::Error),
    #[error("release-creation API rejected the request ({status}): {body}")]
    Rejected { status: u16, body: String },
}

pub struct HttpReleaseApiClient {
    base_url: String,
    http: reqwest::blocking::Client,
}

impl HttpReleaseApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::blocking::Client::new(),
        }
    }
}

impl ReleaseApiClient for HttpReleaseApiClient {
    fn create_release(&self, request: &CreateReleaseRequest) -> Result<Release, ApiClientError> {
        let url = format!(
            "{}/projects/{}/releases",
            self.base_url.trim_end_matches('/'),
            request.project_id
        );

        let mut builder = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", request.credential))
            .json(&request.payload);

        if let Some(key) = &request.idempotency_key {
            builder = builder.header("idempotency-key", key);
        }

        let response = builder.send()?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(ApiClientError::Rejected {
                status: status.as_u16(),
                body,
            });
        }

        Ok(response.json::<Release>()?)
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::cell::RefCell;

    /// Records every request it was asked to send, so tests can assert on
    /// exactly what was sent rather than matching a wire-level HTTP mock.
    #[derive(Default)]
    pub struct FakeReleaseApiClient {
        pub requests: RefCell<Vec<CreateReleaseRequest>>,
        pub respond_with: Option<Release>,
        pub fail_with: Option<ApiClientError>,
    }

    impl ReleaseApiClient for FakeReleaseApiClient {
        fn create_release(
            &self,
            request: &CreateReleaseRequest,
        ) -> Result<Release, ApiClientError> {
            self.requests.borrow_mut().push(request.clone());

            if let Some(err) = &self.fail_with {
                return Err(match err {
                    ApiClientError::Rejected { status, body } => ApiClientError::Rejected {
                        status: *status,
                        body: body.clone(),
                    },
                    ApiClientError::Network(_) => ApiClientError::Rejected {
                        status: 0,
                        body: "simulated network error".to_string(),
                    },
                });
            }

            Ok(self.respond_with.clone().unwrap_or(Release {
                id: "rel_fake".to_string(),
                channel_id: request.payload.channel_id.clone(),
                version: request.payload.version.clone(),
                release_notes: request.payload.release_notes.clone(),
                created_at: "2026-01-01T00:00:00.000Z".to_string(),
            }))
        }
    }
}
