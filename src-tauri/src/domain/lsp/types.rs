use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

pub const RESTART_BACKOFF_LIMIT: u32 = 3;
pub const LSP_MANIFEST_SOURCE: &str = include_str!("../../../resources/lsp-servers.json");

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Type)]
#[serde(transparent)]
pub struct LspServerId(pub String);

impl LspServerId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LspServerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for LspServerId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for LspServerId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl PartialEq<str> for LspServerId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for LspServerId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LspSessionStatus {
    Starting,
    Running,
    Crashed,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LspRootStrategy {
    NearestMarker,
    JsTsDenoAware,
    CargoWorkspace,
    VenvFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LspCommandSpec {
    Path {
        bin: String,
        #[serde(default)]
        args: Vec<String>,
    },
    Managed {
        bin: String,
        #[serde(default)]
        args: Vec<String>,
    },
}

impl LspCommandSpec {
    pub fn bin(&self) -> &str {
        match self {
            LspCommandSpec::Path { bin, .. } | LspCommandSpec::Managed { bin, .. } => bin,
        }
    }

    pub fn args(&self) -> &[String] {
        match self {
            LspCommandSpec::Path { args, .. } | LspCommandSpec::Managed { args, .. } => args,
        }
    }

    pub fn is_managed(&self) -> bool {
        matches!(self, LspCommandSpec::Managed { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum LspInstallStrategy {
    Download,
    Toolchain,
    SdkDetect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LspArchiveKind {
    TarGz,
    TarXz,
    Zip,
    Binary,
    Gz,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspDownloadInstall {
    pub version: String,
    pub urls: BTreeMap<String, String>,
    #[serde(default)]
    pub sha256: BTreeMap<String, Option<String>>,
    pub archive: LspArchiveKind,
    #[serde(default)]
    pub bin_path_in_archive: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LspToolchainTool {
    Go,
    Gem,
    Coursier,
    Ghcup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspToolDetect {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspToolchainInstall {
    pub tool: LspToolchainTool,
    #[serde(default)]
    pub install_args: Vec<String>,
    pub tool_detect: LspToolDetect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LspSdkProbe {
    PathCommand { command: String },
    FixedPath { path: String },
    Xcrun { tool: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspSdkDetectInstall {
    pub probes: Vec<LspSdkProbe>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspInstallSpec {
    pub strategy: LspInstallStrategy,
    #[serde(default)]
    pub hint: Option<String>,
    #[serde(default)]
    pub download: Option<LspDownloadInstall>,
    #[serde(default)]
    pub toolchain: Option<LspToolchainInstall>,
    #[serde(default)]
    pub sdk_detect: Option<LspSdkDetectInstall>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageServerSpec {
    pub id: LspServerId,
    pub name: String,
    pub language_ids: Vec<String>,
    #[serde(default)]
    pub shares_sessions: bool,
    pub command: LspCommandSpec,
    #[serde(default)]
    pub root_markers: Vec<String>,
    #[serde(default = "default_root_strategy")]
    pub root_strategy: LspRootStrategy,
    #[serde(default)]
    pub initialization_options: Option<serde_json::Value>,
    pub install: LspInstallSpec,
}

fn default_root_strategy() -> LspRootStrategy {
    LspRootStrategy::NearestMarker
}

#[derive(Debug, Clone, Deserialize)]
pub struct LspManifest {
    pub servers: Vec<LanguageServerSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LspServerDetection {
    pub id: LspServerId,
    pub name: String,
    pub language_ids: Vec<String>,
    #[serde(default)]
    pub resolved_path: Option<String>,
    pub available: bool,
    #[serde(default)]
    pub install_hint: Option<String>,
    pub install_strategy: LspInstallStrategy,
    #[serde(default)]
    pub installed_version: Option<String>,
    #[serde(default)]
    pub toolchain_available: Option<bool>,
    #[serde(default)]
    pub sdk_available: Option<bool>,
    #[serde(default)]
    pub toolchain_tool: Option<String>,
    #[serde(default)]
    pub download_available: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LspSessionInfo {
    pub session_id: String,
    pub project_id: ProjectId,
    pub server_id: LspServerId,
    pub root: String,
    pub status: LspSessionStatus,
    #[serde(default)]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LspInstallPhase {
    Downloading,
    Verifying,
    Extracting,
    Done,
    Failed,
}
