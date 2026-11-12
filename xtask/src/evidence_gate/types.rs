use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Required metadata fields for benchmark evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkEvidence {
    pub name: String,
    pub baseline: Option<String>,
    pub result: Option<String>,
    pub environment: Option<String>,
    pub command: Option<String>,
}

impl BenchmarkEvidence {
    pub fn has_baseline(&self) -> bool {
        self.baseline.as_ref().is_some_and(|b| !b.is_empty())
    }

    pub fn has_result(&self) -> bool {
        self.result.as_ref().is_some_and(|r| !r.is_empty())
    }

    pub fn has_environment(&self) -> bool {
        self.environment.as_ref().is_some_and(|e| !e.is_empty())
    }

    pub fn has_command(&self) -> bool {
        self.command.as_ref().is_some_and(|c| !c.is_empty())
    }

    pub fn is_complete(&self) -> bool {
        self.has_baseline() && self.has_result() && self.has_environment() && self.has_command()
    }
}

/// Supply-chain audit result from a single tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditResult {
    pub tool: String,
    pub exit_code: Option<i32>,
    pub output_path: Option<PathBuf>,
    pub passed: bool,
    pub notes: String,
}

/// API surface record for a single crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiSurfaceRecord {
    pub crate_name: String,
    pub version: String,
    pub public_item_count: usize,
}

/// Semver stability record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemverRecord {
    pub crate_name: String,
    pub current_version: String,
    pub previous_version: Option<String>,
    pub breaking_changes: Vec<String>,
}

/// Binary bloat analysis record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BloatRecord {
    pub binary_path: String,
    pub total_size_bytes: u64,
    pub top_contributors: Vec<BloatContributor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BloatContributor {
    pub name: String,
    pub size_bytes: u64,
    pub percentage: f64,
}
