use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SchemaVersion;

impl Serialize for SchemaVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(SCHEMA_VERSION)
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match u8::deserialize(deserializer)? {
            SCHEMA_VERSION => Ok(Self),
            version => Err(D::Error::custom(format!(
                "unsupported schema version: {version}"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofOutcome {
    Pass,
    Fail,
    Inconclusive,
    Skipped,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum ProofId {
    #[serde(rename = "WINT-01-authority")]
    Wint01Authority,
    #[serde(rename = "WINT-02-authorization")]
    Wint02Authorization,
    #[serde(rename = "WINT-03-input-validation")]
    Wint03InputValidation,
    #[serde(rename = "WINT-04-response-redaction")]
    Wint04ResponseRedaction,
    #[serde(rename = "WINT-05-sensitive-lifecycle")]
    Wint05SensitiveLifecycle,
    #[serde(rename = "WINT-06-page-lock")]
    Wint06PageLock,
    #[serde(rename = "WINT-07-session-lock")]
    Wint07SessionLock,
    #[serde(rename = "WINT-08-fail-closed")]
    Wint08FailClosed,
    #[serde(rename = "WINT-09-diagnostics")]
    Wint09Diagnostics,
    #[serde(rename = "WINT-10-panic-output")]
    Wint10PanicOutput,
    #[serde(rename = "WINT-11-release-scan")]
    Wint11ReleaseScan,
    #[serde(rename = "WINT-12-storage-observation")]
    Wint12StorageObservation,
    #[serde(rename = "WINT-13-dpapi")]
    Wint13Dpapi,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum RequirementId {
    #[serde(rename = "WINT-01")]
    Wint01,
    #[serde(rename = "WINT-02")]
    Wint02,
    #[serde(rename = "WINT-03")]
    Wint03,
    #[serde(rename = "WINT-04")]
    Wint04,
    #[serde(rename = "WINT-05")]
    Wint05,
    #[serde(rename = "WINT-06")]
    Wint06,
    #[serde(rename = "WINT-07")]
    Wint07,
    #[serde(rename = "WINT-08")]
    Wint08,
    #[serde(rename = "WINT-09")]
    Wint09,
    #[serde(rename = "WINT-10")]
    Wint10,
    #[serde(rename = "WINT-11")]
    Wint11,
    #[serde(rename = "WINT-12")]
    Wint12,
    #[serde(rename = "WINT-13")]
    Wint13,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    X86,
    X86_64,
    Aarch64,
    Unknown,
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("invalid semantic version")]
    InvalidAppVersion,
    #[error("invalid git SHA")]
    InvalidCommit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppVersion(String);

impl AppVersion {
    pub fn parse(value: &str) -> Result<Self, MetadataError> {
        let parts: Vec<_> = value.split('.').collect();
        let is_semver = parts.len() == 3
            && parts.iter().all(|part| {
                !part.is_empty()
                    && (part == &"0" || !part.starts_with('0'))
                    && part.parse::<u32>().is_ok()
            });

        is_semver
            .then(|| Self(value.to_owned()))
            .ok_or(MetadataError::InvalidAppVersion)
    }
}

impl Serialize for AppVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AppVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(&String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit(String);

impl Commit {
    pub fn parse(value: &str) -> Result<Self, MetadataError> {
        (value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .then(|| Self(value.to_owned()))
            .ok_or(MetadataError::InvalidCommit)
    }
}

impl Serialize for Commit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Commit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(&String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildProfile {
    Debug,
    Release,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BitLockerStatus {
    On,
    Off,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceEnvironment {
    pub windows_build: u32,
    pub architecture: Architecture,
    pub app_version: AppVersion,
    pub commit: Commit,
    pub build_profile: BuildProfile,
    pub bitlocker: BitLockerStatus,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ObservationCode {
    StateLocked,
    StateUnlocked,
    ListenerStarted,
    ListenerStopped,
    PageLockActive,
    PageLockDegraded,
    ProofPassed,
    ProofFailed,
    ProofInconclusive,
    ProofSkipped,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub code: ObservationCode,
    pub count: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitationCode {
    NoKernelMemoryInspection,
    AdministrativeAccessUnavailable,
    PlatformFeatureUnavailable,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ArtifactHash(pub [u8; 32]);

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProofResult {
    schema_version: SchemaVersion,
    pub proof_id: ProofId,
    pub requirement_ids: Vec<RequirementId>,
    pub result: ProofOutcome,
    pub environment: EvidenceEnvironment,
    pub observations: Vec<Observation>,
    pub artifact_hashes: Vec<ArtifactHash>,
    pub limitations: Vec<LimitationCode>,
}

impl ProofResult {
    pub fn new(
        proof_id: ProofId,
        requirement_ids: Vec<RequirementId>,
        result: ProofOutcome,
        environment: EvidenceEnvironment,
        observations: Vec<Observation>,
        artifact_hashes: Vec<ArtifactHash>,
        limitations: Vec<LimitationCode>,
    ) -> Self {
        Self {
            schema_version: SchemaVersion,
            proof_id,
            requirement_ids,
            result,
            environment,
            observations,
            artifact_hashes,
            limitations,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityState {
    Locked,
    Unlocked,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LockReasonCode {
    Startup,
    SessionLocked,
    Suspending,
    Resumed,
    ShuttingDown,
    Exiting,
    Manual,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ListenerComponent {
    EventPump,
    SessionNotification,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ListenerStatus {
    Started,
    Retrying,
    Stopped,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PageLockStatus {
    Active,
    Degraded,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiagnosticEvent {
    StateChanged {
        state: SecurityState,
        epoch: u64,
        reason: LockReasonCode,
    },
    WindowsListener {
        component: ListenerComponent,
        status: ListenerStatus,
        platform_code: Option<u32>,
    },
    PageLock {
        status: PageLockStatus,
        platform_code: Option<u32>,
    },
    ProofResult {
        proof_id: ProofId,
        result: ProofOutcome,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_result() -> ProofResult {
        ProofResult::new(
            ProofId::Wint07SessionLock,
            vec![RequirementId::Wint07, RequirementId::Wint08],
            ProofOutcome::Pass,
            EvidenceEnvironment {
                windows_build: 22631,
                architecture: Architecture::X86_64,
                app_version: AppVersion::parse("0.1.7").unwrap(),
                commit: Commit::parse("0123456789abcdef0123456789abcdef01234567").unwrap(),
                build_profile: BuildProfile::Release,
                bitlocker: BitLockerStatus::On,
            },
            vec![Observation {
                code: ObservationCode::StateLocked,
                count: 1,
            }],
            Vec::new(),
            vec![LimitationCode::NoKernelMemoryInspection],
        )
    }

    #[test]
    fn serializes_only_the_allowlisted_pass_outcome() {
        let value = serde_json::to_value(ProofOutcome::Pass).unwrap();

        assert_eq!(value, serde_json::json!("pass"));
    }

    #[test]
    fn rejects_an_unknown_outcome() {
        let result = serde_json::from_str::<ProofOutcome>("\"approved\"");

        assert!(result.is_err());
    }

    #[test]
    fn serializes_schema_version_one() {
        let value = serde_json::to_value(sample_result()).unwrap();

        assert_eq!(value["schema_version"], 1);
    }

    #[test]
    fn rejects_an_unknown_schema_version() {
        let mut value = serde_json::to_value(sample_result()).unwrap();
        value["schema_version"] = serde_json::json!(2);

        assert!(serde_json::from_value::<ProofResult>(value).is_err());
    }

    #[test]
    fn rejects_an_unknown_proof_result_field() {
        let mut value = serde_json::to_value(sample_result()).unwrap();
        value["path"] = serde_json::json!("C:/sensitive");

        assert!(serde_json::from_value::<ProofResult>(value).is_err());
    }

    #[test]
    fn rejects_an_unknown_observation_code() {
        let mut value = serde_json::to_value(sample_result()).unwrap();
        value["observations"][0]["code"] = serde_json::json!("RAW_ERROR");

        assert!(serde_json::from_value::<ProofResult>(value).is_err());
    }

    #[test]
    fn rejects_an_unknown_limitation_code() {
        let mut value = serde_json::to_value(sample_result()).unwrap();
        value["limitations"][0] = serde_json::json!("opaque_media");

        assert!(serde_json::from_value::<ProofResult>(value).is_err());
    }

    #[test]
    fn result_schema_has_no_path_user_or_canary_fields() {
        let value = serde_json::to_value(sample_result()).unwrap();
        let object = value.as_object().unwrap();

        assert!(!object.contains_key("path"));
        assert!(!object.contains_key("user"));
        assert!(!object.contains_key("canary"));
    }

    #[test]
    fn rejects_an_unknown_diagnostic_event_field() {
        let event = serde_json::json!({
            "event": "state_changed",
            "state": "locked",
            "epoch": 1,
            "reason": "startup",
            "user": "sensitive"
        });

        assert!(serde_json::from_value::<DiagnosticEvent>(event).is_err());
    }

    #[test]
    fn accepts_only_semver_app_versions() {
        assert!(AppVersion::parse("1.2.3").is_ok());
        assert!(AppVersion::parse("1.2").is_err());
        assert!(serde_json::from_str::<AppVersion>("\"1.2.3\"").is_ok());
        assert!(serde_json::from_str::<AppVersion>("\"1.2.3-canary\"").is_err());
    }

    #[test]
    fn accepts_only_forty_digit_hex_git_shas() {
        assert!(Commit::parse("0123456789abcdef0123456789abcdef01234567").is_ok());
        assert!(Commit::parse("0123456789abcdef").is_err());
        assert!(
            serde_json::from_str::<Commit>("\"0123456789abcdef0123456789abcdef01234567\"").is_ok()
        );
        assert!(serde_json::from_str::<Commit>("\"not-a-git-sha\"").is_err());
    }
}
