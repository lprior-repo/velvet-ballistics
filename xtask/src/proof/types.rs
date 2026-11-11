use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofObligation {
    pub id: String,
    pub section: Vec<usize>,
    pub statement: String,
    pub proof_level: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub files: Vec<String>,
    pub required: RequiredProof,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredProof {
    #[serde(default)]
    pub verus: Option<String>,
    #[serde(default)]
    pub kani: Option<String>,
    #[serde(default)]
    pub tests: Vec<String>,
    #[serde(default)]
    pub fuzz: Option<FuzzField>,
    #[serde(default)]
    pub tla: Option<String>,
    #[serde(default)]
    pub loom: Option<String>,
    #[serde(default)]
    pub property_tests: Vec<String>,
    #[serde(default)]
    pub differential_tests: Vec<String>,
    #[serde(default)]
    pub crash_lab: Vec<String>,
    #[serde(default)]
    pub diff_test: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FuzzField {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProofLevel {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProofMatrix {
    pub crate_name: String,
    pub proof_strategy: String,
    pub primary_level: String,
    pub secondary_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProofTarget {
    pub obligation_id: String,
    pub file: PathBuf,
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObligationStatus {
    pub id: String,
    pub status: String,
    pub level: String,
    pub commands: Vec<String>,
    pub log: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEvidence {
    pub kind: String,
    pub bead: String,
    pub commit: String,
    pub obligations: Vec<ObligationStatus>,
    pub remaining_assumptions: Vec<String>,
    pub verified: HashMap<String, String>,
}
