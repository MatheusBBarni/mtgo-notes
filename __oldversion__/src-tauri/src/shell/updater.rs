use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(windows)]
use std::io::Write;

use crate::ipc::{AppError, ErrorCode};

pub const RELEASE_TARGET: &str = "windows";
pub const RELEASE_ARCHITECTURE: &str = "x86_64";
pub const RELEASE_PUBLIC_KEY: [u8; 32] = [
    215, 90, 152, 1, 130, 177, 10, 183, 213, 75, 254, 211, 201, 100, 7, 58, 14, 225, 114, 243, 218,
    166, 35, 37, 175, 2, 26, 104, 247, 7, 81, 26,
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckRequest {
    pub target: String,
    pub architecture: String,
    pub current_version: String,
}

impl UpdateCheckRequest {
    pub fn current() -> Self {
        Self {
            target: RELEASE_TARGET.into(),
            architecture: RELEASE_ARCHITECTURE.into(),
            current_version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedRelease {
    pub version: String,
    pub release_notes: String,
    pub classifier_change_summary: String,
    pub artifact_digest: String,
    pub metadata_signature: String,
    pub artifact_signature: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStage {
    Idle,
    Checking,
    Available,
    Downloading,
    Verifying,
    AwaitingConfirmation,
    Installing,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub stage: UpdateStage,
    pub version: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallUpdateRequest {
    pub version: String,
    pub confirmed: bool,
}

pub trait ReleaseEndpoint: Send + Sync {
    fn check(&self, request: &UpdateCheckRequest) -> Result<Option<SignedRelease>, AppError>;
    fn download(&self, version: &str) -> Result<Vec<u8>, AppError>;
}

pub trait PassiveInstaller: Send + Sync {
    fn install(&self, artifact: &[u8]) -> Result<(), AppError>;
}

pub struct HttpReleaseEndpoint {
    client: reqwest::blocking::Client,
    metadata_url: &'static str,
    artifact_url: &'static str,
}

impl HttpReleaseEndpoint {
    fn configured() -> Option<Self> {
        let metadata_url = option_env!("MTGO_NOTES_UPDATE_ENDPOINT")?;
        let artifact_url = option_env!("MTGO_NOTES_UPDATE_ARTIFACT_ENDPOINT")?;
        if !metadata_url.starts_with("https://") || !artifact_url.starts_with("https://") {
            return None;
        }
        let client = reqwest::blocking::Client::builder()
            .https_only(true)
            .timeout(Duration::from_secs(30))
            .build()
            .ok()?;
        Some(Self {
            client,
            metadata_url,
            artifact_url,
        })
    }
}

impl ReleaseEndpoint for HttpReleaseEndpoint {
    fn check(&self, request: &UpdateCheckRequest) -> Result<Option<SignedRelease>, AppError> {
        let response = self
            .client
            .get(self.metadata_url)
            .query(request)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|_| update_unavailable())?;
        let bytes = read_capped(response, 1024 * 1024)?;
        serde_json::from_slice::<Option<SignedRelease>>(&bytes).map_err(|_| update_unavailable())
    }

    fn download(&self, version: &str) -> Result<Vec<u8>, AppError> {
        const MAX_INSTALLER_BYTES: u64 = 256 * 1024 * 1024;
        let response = self
            .client
            .get(self.artifact_url)
            .query(&[("version", version)])
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|_| update_unavailable())?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_INSTALLER_BYTES)
        {
            return Err(update_unavailable());
        }
        read_capped(response, MAX_INSTALLER_BYTES)
    }
}

#[derive(Default)]
pub struct NoReleaseEndpoint;

impl ReleaseEndpoint for NoReleaseEndpoint {
    fn check(&self, _request: &UpdateCheckRequest) -> Result<Option<SignedRelease>, AppError> {
        Ok(None)
    }

    fn download(&self, _version: &str) -> Result<Vec<u8>, AppError> {
        Err(update_unavailable())
    }
}

#[derive(Default)]
pub struct PassiveWindowsInstaller;

impl PassiveInstaller for PassiveWindowsInstaller {
    fn install(&self, artifact: &[u8]) -> Result<(), AppError> {
        #[cfg(windows)]
        {
            let artifact_path = std::env::temp_dir().join(format!(
                "mtgo-notes-update-{}.exe",
                crate::domain::EntityId::new()
            ));
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&artifact_path)
                .map_err(|_| install_failed())?;
            file.write_all(artifact).map_err(|_| install_failed())?;
            file.sync_all().map_err(|_| install_failed())?;
            std::process::Command::new(&artifact_path)
                .arg("/S")
                .spawn()
                .map_err(|_| install_failed())?;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = artifact;
            Err(AppError::new(
                ErrorCode::UpdateUnavailable,
                "Passive installation is available only in the packaged Windows application.",
                false,
            ))
        }
    }
}

#[derive(Clone)]
pub struct UpdaterService {
    endpoint: Arc<dyn ReleaseEndpoint>,
    installer: Arc<dyn PassiveInstaller>,
    verifying_key: VerifyingKey,
    pending: Arc<Mutex<Option<SignedRelease>>>,
    status: Arc<Mutex<UpdateStatus>>,
}

impl Default for UpdaterService {
    fn default() -> Self {
        let endpoint: Arc<dyn ReleaseEndpoint> = HttpReleaseEndpoint::configured()
            .map(|endpoint| Arc::new(endpoint) as Arc<dyn ReleaseEndpoint>)
            .unwrap_or_else(|| Arc::new(NoReleaseEndpoint));
        Self::new(
            endpoint,
            Arc::new(PassiveWindowsInstaller),
            VerifyingKey::from_bytes(&RELEASE_PUBLIC_KEY).expect("pinned update key"),
        )
    }
}

impl UpdaterService {
    pub fn new(
        endpoint: Arc<dyn ReleaseEndpoint>,
        installer: Arc<dyn PassiveInstaller>,
        verifying_key: VerifyingKey,
    ) -> Self {
        Self {
            endpoint,
            installer,
            verifying_key,
            pending: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(UpdateStatus {
                stage: UpdateStage::Idle,
                version: None,
                error_code: None,
            })),
        }
    }

    pub fn check(
        &self,
        enabled: bool,
        request: UpdateCheckRequest,
    ) -> Result<SignedRelease, AppError> {
        if !enabled {
            return Err(update_unavailable());
        }
        self.set_status(UpdateStage::Checking, None, None)?;
        let Some(release) = self.endpoint.check(&request)? else {
            self.set_status(UpdateStage::Idle, None, Some("update_unavailable".into()))?;
            return Err(update_unavailable());
        };
        verify_release_metadata(&release, &self.verifying_key)?;
        if !is_newer_version(&release.version, &request.current_version) {
            self.set_status(UpdateStage::Idle, None, Some("update_unavailable".into()))?;
            return Err(update_unavailable());
        }
        *self.pending.lock().map_err(|_| updater_state_error())? = Some(release.clone());
        self.set_status(
            UpdateStage::AwaitingConfirmation,
            Some(release.version.clone()),
            None,
        )?;
        Ok(release)
    }

    pub fn launch_check(
        &self,
        enabled: bool,
        request: UpdateCheckRequest,
    ) -> Result<Option<SignedRelease>, AppError> {
        if !enabled {
            return Ok(None);
        }
        self.check(enabled, request).map(Some)
    }

    pub fn install(&self, request: InstallUpdateRequest) -> Result<UpdateStatus, AppError> {
        if !request.confirmed {
            return Err(AppError::new(
                ErrorCode::InvalidRequest,
                "Confirm installation before downloading an update.",
                false,
            ));
        }
        let release = self
            .pending
            .lock()
            .map_err(|_| updater_state_error())?
            .clone()
            .filter(|release| release.version == request.version)
            .ok_or_else(update_unavailable)?;
        self.set_status(
            UpdateStage::Downloading,
            Some(release.version.clone()),
            None,
        )?;
        let artifact = self.endpoint.download(&release.version)?;
        self.set_status(UpdateStage::Verifying, Some(release.version.clone()), None)?;
        verify_release_artifact(&release, &artifact, &self.verifying_key)?;
        self.set_status(UpdateStage::Installing, Some(release.version.clone()), None)?;
        if let Err(error) = self.installer.install(&artifact) {
            self.set_status(
                UpdateStage::Failed,
                Some(release.version),
                Some(error_code(&error).into()),
            )?;
            return Err(error);
        }
        *self.pending.lock().map_err(|_| updater_state_error())? = None;
        self.set_status(UpdateStage::Completed, Some(release.version), None)?;
        self.status()
    }

    pub fn status(&self) -> Result<UpdateStatus, AppError> {
        self.status
            .lock()
            .map_err(|_| updater_state_error())
            .map(|status| status.clone())
    }

    fn set_status(
        &self,
        stage: UpdateStage,
        version: Option<String>,
        error_code: Option<String>,
    ) -> Result<(), AppError> {
        *self.status.lock().map_err(|_| updater_state_error())? = UpdateStatus {
            stage,
            version,
            error_code,
        };
        Ok(())
    }
}

pub fn verify_release_metadata(
    release: &SignedRelease,
    key: &VerifyingKey,
) -> Result<(), AppError> {
    if release.version.trim().is_empty()
        || release.release_notes.trim().is_empty()
        || !release.artifact_digest.starts_with("sha256:")
    {
        return Err(signature_invalid());
    }
    verify_signature(key, &release.metadata_signature, &metadata_payload(release))
}

pub fn verify_release_artifact(
    release: &SignedRelease,
    artifact: &[u8],
    key: &VerifyingKey,
) -> Result<(), AppError> {
    let digest = format!("sha256:{}", sha256_hex(artifact));
    if digest != release.artifact_digest {
        return Err(signature_invalid());
    }
    verify_signature(key, &release.artifact_signature, artifact)
}

pub fn metadata_payload(release: &SignedRelease) -> Vec<u8> {
    [
        release.version.as_str(),
        release.release_notes.as_str(),
        release.classifier_change_summary.as_str(),
        release.artifact_digest.as_str(),
    ]
    .join("\n")
    .into_bytes()
}

fn verify_signature(key: &VerifyingKey, encoded: &str, payload: &[u8]) -> Result<(), AppError> {
    let bytes = STANDARD
        .decode(encoded.strip_prefix("ed25519:").unwrap_or(encoded))
        .map_err(|_| signature_invalid())?;
    let signature = Signature::from_slice(&bytes).map_err(|_| signature_invalid())?;
    key.verify(payload, &signature)
        .map_err(|_| signature_invalid())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn signature_invalid() -> AppError {
    AppError::new(
        ErrorCode::SignatureInvalid,
        "The update signature is invalid. The installed application is unchanged.",
        false,
    )
}

fn update_unavailable() -> AppError {
    AppError::new(
        ErrorCode::UpdateUnavailable,
        "No newer signed release is available.",
        false,
    )
}

fn updater_state_error() -> AppError {
    AppError::internal("updater-state-lock")
}

fn read_capped(response: reqwest::blocking::Response, max_bytes: u64) -> Result<Vec<u8>, AppError> {
    let mut bytes = Vec::new();
    response
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| update_unavailable())?;
    if bytes.len() as u64 > max_bytes {
        return Err(update_unavailable());
    }
    Ok(bytes)
}

#[cfg(windows)]
fn install_failed() -> AppError {
    AppError::new(
        ErrorCode::SaveFailed,
        "The verified update installer could not be launched.",
        true,
    )
}

fn is_newer_version(candidate: &str, current: &str) -> bool {
    fn parts(version: &str) -> Option<Vec<u64>> {
        let core = version.trim().trim_start_matches('v').split('-').next()?;
        let parsed = core
            .split('.')
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        (!parsed.is_empty()).then_some(parsed)
    }
    match (parts(candidate), parts(current)) {
        (Some(mut candidate), Some(mut current)) => {
            let width = candidate.len().max(current.len());
            candidate.resize(width, 0);
            current.resize(width, 0);
            candidate > current
        }
        _ => false,
    }
}

fn error_code(error: &AppError) -> &'static str {
    match error.code {
        ErrorCode::SignatureInvalid => "signature_invalid",
        ErrorCode::UpdateUnavailable => "update_unavailable",
        _ => "install_failed",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use ed25519_dalek::{Signer, SigningKey};

    use super::*;

    const TEST_SIGNING_KEY: [u8; 32] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
    ];

    struct FixtureEndpoint {
        checks: Arc<AtomicUsize>,
        request: Arc<Mutex<Option<UpdateCheckRequest>>>,
        release: SignedRelease,
        artifact: Vec<u8>,
    }

    impl ReleaseEndpoint for FixtureEndpoint {
        fn check(&self, request: &UpdateCheckRequest) -> Result<Option<SignedRelease>, AppError> {
            self.checks.fetch_add(1, Ordering::SeqCst);
            *self.request.lock().expect("request") = Some(request.clone());
            Ok(Some(self.release.clone()))
        }

        fn download(&self, _version: &str) -> Result<Vec<u8>, AppError> {
            Ok(self.artifact.clone())
        }
    }

    #[derive(Default)]
    struct FixtureInstaller {
        calls: AtomicUsize,
    }

    impl PassiveInstaller for FixtureInstaller {
        fn install(&self, _artifact: &[u8]) -> Result<(), AppError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    type Fixture = (
        UpdaterService,
        Arc<AtomicUsize>,
        Arc<Mutex<Option<UpdateCheckRequest>>>,
        Arc<FixtureInstaller>,
        SignedRelease,
    );

    fn fixture() -> Fixture {
        let signing = SigningKey::from_bytes(&TEST_SIGNING_KEY);
        let artifact = b"signed passive installer".to_vec();
        let mut release = SignedRelease {
            version: "0.2.0".into(),
            release_notes: "Privacy and resilience improvements.".into(),
            classifier_change_summary: "Modern corpus refresh.".into(),
            artifact_digest: format!("sha256:{}", sha256_hex(&artifact)),
            metadata_signature: String::new(),
            artifact_signature: format!(
                "ed25519:{}",
                STANDARD.encode(signing.sign(&artifact).to_bytes())
            ),
        };
        release.metadata_signature = format!(
            "ed25519:{}",
            STANDARD.encode(signing.sign(&metadata_payload(&release)).to_bytes())
        );
        let checks = Arc::new(AtomicUsize::new(0));
        let request = Arc::new(Mutex::new(None));
        let installer = Arc::new(FixtureInstaller::default());
        let endpoint = FixtureEndpoint {
            checks: Arc::clone(&checks),
            request: Arc::clone(&request),
            release: release.clone(),
            artifact,
        };
        (
            UpdaterService::new(
                Arc::new(endpoint),
                installer.clone(),
                signing.verifying_key(),
            ),
            checks,
            request,
            installer,
            release,
        )
    }

    #[test]
    fn ut_092_update_check_sends_only_documented_metadata() {
        let (service, _, request, _, _) = fixture();
        service
            .check(true, UpdateCheckRequest::current())
            .expect("release");
        assert_eq!(
            request.lock().expect("request").clone().expect("captured"),
            UpdateCheckRequest {
                target: "windows".into(),
                architecture: "x86_64".into(),
                current_version: env!("CARGO_PKG_VERSION").into(),
            }
        );
    }

    #[test]
    fn ut_093_invalid_signature_never_invokes_installation() {
        let (service, _, _, installer, release) = fixture();
        let mut tampered = release;
        tampered.release_notes = "tampered".into();
        assert_eq!(
            verify_release_metadata(
                &tampered,
                &SigningKey::from_bytes(&TEST_SIGNING_KEY).verifying_key()
            )
            .expect_err("tampered")
            .code,
            ErrorCode::SignatureInvalid
        );
        assert_eq!(installer.calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            service
                .install(InstallUpdateRequest {
                    version: "0.2.0".into(),
                    confirmed: true,
                })
                .expect_err("not pending")
                .code,
            ErrorCode::UpdateUnavailable
        );
    }

    #[test]
    fn ut_094_disabled_launch_check_performs_no_endpoint_request() {
        let (service, checks, _, _, _) = fixture();
        assert_eq!(
            service
                .launch_check(false, UpdateCheckRequest::current())
                .expect("disabled"),
            None
        );
        assert_eq!(checks.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn it_281_trusted_fixture_accepts_signed_and_rejects_altered_artifacts() {
        let (service, _, _, installer, release) = fixture();
        service
            .check(true, UpdateCheckRequest::current())
            .expect("check");
        service
            .install(InstallUpdateRequest {
                version: release.version.clone(),
                confirmed: true,
            })
            .expect("install");
        assert_eq!(installer.calls.load(Ordering::SeqCst), 1);

        assert_eq!(
            verify_release_artifact(
                &release,
                b"altered",
                &SigningKey::from_bytes(&TEST_SIGNING_KEY).verifying_key(),
            )
            .expect_err("altered")
            .code,
            ErrorCode::SignatureInvalid
        );
    }

    #[test]
    fn it_271_status_contains_no_device_identifier() {
        let (service, _, _, _, _) = fixture();
        service
            .check(true, UpdateCheckRequest::current())
            .expect("check");
        let json = serde_json::to_string(&service.status().expect("status")).expect("json");
        for prohibited in ["device", "user", "machine", "handle"] {
            assert!(!json.to_ascii_lowercase().contains(prohibited));
        }
    }

    #[test]
    fn it_231_opted_in_check_returns_signed_notes_and_classifier_summary() {
        let (service, _, _, _, release) = fixture();
        let available = service
            .check(true, UpdateCheckRequest::current())
            .expect("available");
        assert_eq!(available.version, release.version);
        assert!(!available.release_notes.is_empty());
        assert!(!available.classifier_change_summary.is_empty());
    }

    #[test]
    fn it_232_confirmed_install_verifies_then_invokes_passive_installer() {
        let (service, _, _, installer, release) = fixture();
        service
            .check(true, UpdateCheckRequest::current())
            .expect("check");
        let status = service
            .install(InstallUpdateRequest {
                version: release.version,
                confirmed: true,
            })
            .expect("install");
        assert_eq!(status.stage, UpdateStage::Completed);
        assert_eq!(installer.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn it_263_no_newer_signed_release_is_update_unavailable() {
        let service = UpdaterService::default();
        assert_eq!(
            service
                .check(true, UpdateCheckRequest::current())
                .expect_err("unavailable")
                .code,
            ErrorCode::UpdateUnavailable
        );
    }

    #[test]
    fn signed_downgrades_are_not_offered() {
        assert!(is_newer_version("0.2.0", "0.1.9"));
        assert!(!is_newer_version("0.1.0", "0.2.0"));
        assert!(!is_newer_version("invalid", "0.2.0"));
    }

    #[test]
    fn it_264_tampered_artifact_cannot_install() {
        let (_, _, _, installer, release) = fixture();
        let signing = SigningKey::from_bytes(&TEST_SIGNING_KEY);
        assert_eq!(
            verify_release_artifact(&release, b"tampered", &signing.verifying_key())
                .expect_err("tampered")
                .code,
            ErrorCode::SignatureInvalid
        );
        assert_eq!(installer.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn interrupted_install_preserves_pending_last_known_good_state() {
        struct FailingInstaller;
        impl PassiveInstaller for FailingInstaller {
            fn install(&self, _artifact: &[u8]) -> Result<(), AppError> {
                Err(AppError::new(
                    ErrorCode::SaveFailed,
                    "Injected interruption.",
                    true,
                ))
            }
        }

        let (fixture_service, _, _, _, release) = fixture();
        let endpoint = Arc::clone(&fixture_service.endpoint);
        let service = UpdaterService::new(
            endpoint,
            Arc::new(FailingInstaller),
            SigningKey::from_bytes(&TEST_SIGNING_KEY).verifying_key(),
        );
        service
            .check(true, UpdateCheckRequest::current())
            .expect("check");
        assert!(
            service
                .install(InstallUpdateRequest {
                    version: release.version.clone(),
                    confirmed: true,
                })
                .is_err()
        );
        assert_eq!(service.status().expect("status").stage, UpdateStage::Failed);
        assert!(
            service
                .install(InstallUpdateRequest {
                    version: release.version,
                    confirmed: true,
                })
                .is_err()
        );
    }
}
