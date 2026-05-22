//! Schema definitions for assurec v1.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleProvenanceKind {
    VcsPreexisting,
    ApprovedReview,
    HistoricalBug,
    ExternalSpec,
    IncidentRegression,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleProvenance {
    pub commit: String,
    pub present_in_merge_base: bool,
    pub signature_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleRecord {
    pub id: String,
    pub source_kind: OracleProvenanceKind,
    pub provenance: OracleProvenance,
    pub claim: String,
    pub generated: bool,
}

impl OracleRecord {
    pub fn is_trusted(&self) -> bool {
        self.provenance.signature_verified && self.provenance.present_in_merge_base
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DomainType {
    pub name: String,
    pub variants: Vec<String>,
}

impl DomainType {
    pub fn new(name: impl Into<String>, variants: Vec<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            variants: variants.into_iter().map(|v| v.into()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WitnessPolicy {
    Debug,
    Clone,
    PartialEq,
    Eq,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", content = "args")]
pub enum TypedExpr {
    Bool(bool),
    Var { name: String },
    Eq { lhs: Box<TypedExpr>, rhs: Box<TypedExpr> },
    Neq { lhs: Box<TypedExpr>, rhs: Box<TypedExpr> },
    And { lhs: Box<TypedExpr>, rhs: Box<TypedExpr> },
    Or { lhs: Box<TypedExpr>, rhs: Box<TypedExpr> },
    Not { inner: Box<TypedExpr> },
}

impl TypedExpr {
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var { name: name.into() }
    }
    pub fn eq(lhs: TypedExpr, rhs: TypedExpr) -> Self {
        Self::Eq { lhs: Box::new(lhs), rhs: Box::new(rhs) }
    }
    pub fn and(lhs: TypedExpr, rhs: TypedExpr) -> Self {
        Self::And { lhs: Box::new(lhs), rhs: Box::new(rhs) }
    }
    pub fn or(lhs: TypedExpr, rhs: TypedExpr) -> Self {
        Self::Or { lhs: Box::new(lhs), rhs: Box::new(rhs) }
    }
    pub fn not(inner: TypedExpr) -> Self {
        Self::Not { inner: Box::new(inner) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractClause {
    pub id: String,
    pub when: Vec<TypedExpr>,
    pub then: EffectOutcome,
}

impl ContractClause {
    pub fn new(id: impl Into<String>, when: Vec<TypedExpr>, then: EffectOutcome) -> Self {
        Self { id: id.into(), when, then }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectOutcome {
    pub err: String,
}

impl EffectOutcome {
    pub fn err(err: impl Into<String>) -> Self {
        Self { err: err.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimCeiling {
    pub id: String,
    pub blocked_claims: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPath {
    pub id: String,
    pub valuation: BTreeMap<String, String>,
    pub outcome: PathOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PathOutcome {
    Grant,
    Error { code: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssumptionLedgerEntry {
    pub id: String,
    pub assumption: String,
    pub source: OracleRecord,
    pub claim_ceiling_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcbLedgerEntry {
    pub id: String,
    pub component: String,
    pub version: String,
    pub artifact_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waiver {
    pub id: String,
    pub violation_type: String,
    pub target: String,
    pub reason: String,
    pub approved_by: OracleRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub id: String,
    pub spec_digest: String,
    pub ir_digest: String,
    pub oracle_bank_digest: String,
    pub source_digest: String,
    pub generator_digest: String,
    pub tool_versions: BTreeMap<String, String>,
    pub cwd_digest: String,
    pub env_digest: String,
    pub stdout_digest: String,
    pub stderr_digest: String,
    pub exit_code: i32,
    pub runner_identity: String,
    pub claim_ceiling_id: String,
    pub trusted: bool,
}

impl EvidenceRecord {
    pub fn is_stale(&self, other: &EvidenceRecord) -> bool {
        self.spec_digest != other.spec_digest
            || self.ir_digest != other.ir_digest
            || self.oracle_bank_digest != other.oracle_bank_digest
            || self.source_digest != other.source_digest
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedArtifact {
    pub path: String,
    pub content_digest: String,
    pub source_digest: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairPacket {
    pub id: String,
    pub violation_type: String,
    pub target: String,
    pub proposed_fix: String,
    pub oracle_records: Vec<OracleRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineFailure {
    pub id: String,
    pub test_name: String,
    pub mutation: String,
    pub baseline_output: String,
}
