use serde::{Deserialize, Serialize};
use std::time::Duration;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

const MAX_ATTEMPTS: u32 = 3;
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);

/// Delay before the next attempt, or `None` if this outcome shouldn't be
/// retried at all. `attempt` is 1-based (the attempt that just finished).
fn retry_delay(attempt: u32, retryable: bool) -> Option<Duration> {
    if !retryable || attempt >= MAX_ATTEMPTS {
        return None;
    }
    Some(INITIAL_BACKOFF * 2u32.pow(attempt - 1))
}

fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    status.is_server_error()
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

    fn send_once(
        &self,
        request: &CreateReleaseRequest,
    ) -> reqwest::Result<reqwest::blocking::Response> {
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

        builder.send()
    }
}

impl ReleaseApiClient for HttpReleaseApiClient {
    fn create_release(&self, request: &CreateReleaseRequest) -> Result<Release, ApiClientError> {
        let mut attempt = 0;
        loop {
            attempt += 1;

            match self.send_once(request) {
                Ok(response) if response.status().is_success() => {
                    return Ok(response.json::<Release>()?);
                }
                Ok(response) => {
                    let status = response.status();
                    if let Some(delay) = retry_delay(attempt, is_retryable_status(status)) {
                        std::thread::sleep(delay);
                        continue;
                    }
                    let body = response.text().unwrap_or_default();
                    return Err(ApiClientError::Rejected {
                        status: status.as_u16(),
                        body,
                    });
                }
                Err(err) => {
                    let network_errors_are_always_retryable = true;
                    if let Some(delay) = retry_delay(attempt, network_errors_are_always_retryable) {
                        std::thread::sleep(delay);
                        continue;
                    }
                    return Err(ApiClientError::Network(err));
                }
            }
        }
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

            Ok(self.respond_with.clone().unwrap_or_else(|| Release {
                id: "rel_fake".to_string(),
                channel_id: request.payload.channel_id.clone(),
                version: request.payload.version.clone(),
                release_notes: request.payload.release_notes.clone(),
                created_at: "2026-01-01T00:00:00.000Z".to_string(),
            }))
        }
    }
}

#[cfg(test)]
mod retry_tests {
    use super::*;

    #[test]
    fn retries_a_retryable_outcome_with_doubling_backoff_up_to_the_attempt_cap() {
        assert_eq!(retry_delay(1, true), Some(Duration::from_secs(1)));
        assert_eq!(retry_delay(2, true), Some(Duration::from_secs(2)));
        assert_eq!(retry_delay(MAX_ATTEMPTS, true), None);
    }

    #[test]
    fn never_retries_a_non_retryable_outcome_regardless_of_attempt() {
        assert_eq!(retry_delay(1, false), None);
    }

    #[test]
    fn classifies_5xx_as_retryable_and_4xx_as_not() {
        assert!(is_retryable_status(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR
        ));
        assert!(is_retryable_status(
            reqwest::StatusCode::SERVICE_UNAVAILABLE
        ));
        assert!(!is_retryable_status(reqwest::StatusCode::BAD_REQUEST));
        assert!(!is_retryable_status(reqwest::StatusCode::UNAUTHORIZED));
        assert!(!is_retryable_status(reqwest::StatusCode::NOT_FOUND));
    }
}

#[cfg(test)]
mod http_retry_tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;

    /// A minimal HTTP/1.1 server: replies to exactly `responses.len()`
    /// connections with each given status in turn, and reports the
    /// `idempotency-key` header it received on each one.
    fn spawn_fake_server(responses: Vec<u16>) -> (String, mpsc::Receiver<Option<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            for status in responses {
                let (stream, _) = listener.accept().unwrap();
                tx.send(handle_one_request(stream, status)).unwrap();
            }
        });

        (format!("http://{addr}"), rx)
    }

    fn handle_one_request(stream: TcpStream, status: u16) -> Option<String> {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut idempotency_key = None;
        let mut content_length = 0usize;

        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(value) = line.strip_prefix("idempotency-key: ") {
                idempotency_key = Some(value.to_string());
            }
            if let Some(value) = line.strip_prefix("content-length: ") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }

        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body).unwrap();

        let payload = r#"{"id":"rel_1","channelId":"chn","version":"1.0.0","releaseNotes":"","createdAt":"2026-01-01T00:00:00.000Z"}"#;
        let response = format!(
            "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
            payload.len()
        );
        (&mut &stream).write_all(response.as_bytes()).unwrap();

        idempotency_key
    }

    fn sample_request() -> CreateReleaseRequest {
        CreateReleaseRequest {
            project_id: "proj_123".to_string(),
            credential: "brl_app_test".to_string(),
            idempotency_key: Some("idem_abc".to_string()),
            payload: CreateReleasePayload {
                channel_id: "chn_stable".to_string(),
                version: "1.0.0".to_string(),
                release_notes: String::new(),
                artifacts: vec![],
            },
        }
    }

    #[test]
    fn retries_a_5xx_and_succeeds_on_the_next_attempt_with_the_same_idempotency_key() {
        let (base_url, rx) = spawn_fake_server(vec![500, 200]);
        let client = HttpReleaseApiClient::new(base_url);

        let release = client.create_release(&sample_request()).unwrap();
        assert_eq!(release.id, "rel_1");

        assert_eq!(rx.recv().unwrap(), Some("idem_abc".to_string()));
        assert_eq!(rx.recv().unwrap(), Some("idem_abc".to_string()));
    }

    #[test]
    fn never_retries_a_4xx() {
        let (base_url, rx) = spawn_fake_server(vec![400]);
        let client = HttpReleaseApiClient::new(base_url);

        let result = client.create_release(&sample_request());
        assert!(matches!(
            result,
            Err(ApiClientError::Rejected { status: 400, .. })
        ));

        assert_eq!(rx.recv().unwrap(), Some("idem_abc".to_string()));
        assert!(rx.recv().is_err(), "a second request was made for a 4xx");
    }
}
