use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub addresses: Vec<String>,
    #[serde(default)]
    pub hostnames: Vec<String>,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub asset_id: String,
    pub port: u16,
    pub protocol: String,
    pub state: String,
    pub service: Option<String>,
    pub title: String,
    pub description: String,
    pub severity: String,
    #[serde(default)]
    pub evidence: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanArtifacts {
    pub tool: String,
    pub target: String,
    pub assets: Vec<Asset>,
    pub findings: Vec<Finding>,
    pub raw_xml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptArtifact {
    pub name: String,
    pub command: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub started_at: String,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGroupArtifact {
    pub name: String,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportArtifact {
    pub target: String,
    pub format: String,
    pub generated_at: String,
    pub includes: BTreeMap<String, Value>,
    pub tables: BTreeMap<String, TableArtifact>,
    #[serde(default)]
    pub output_path: Option<String>,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableArtifact {
    pub columns: Vec<String>,
    pub rows: Vec<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArtifactKind {
    AssetGroup,
    Scan,
    Script,
    Report,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredArtifact {
    pub name: String,
    pub kind: ArtifactKind,
    pub path: Option<String>,
    pub data: Value,
}
