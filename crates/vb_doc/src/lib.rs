#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]

use std::path::PathBuf;

pub mod evidence;
pub mod reconcile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterDocSnapshot {
    pub path: PathBuf,
    pub text: String,
}

impl MasterDocSnapshot {
    pub fn for_workspace_text(path: PathBuf, text: &str) -> Self {
        Self {
            path,
            text: text.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidencePolicy {
    pub workspace_root: PathBuf,
    pub required: RequiredEvidence,
}

impl EvidencePolicy {
    pub fn strict_bounded(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocPatchPlan {
    pub target: PatchTarget,
    pub edits: Vec<PatchEdit>,
    pub stale_text_removed: Vec<StalePhrase>,
    pub evidence_actions: RequiredEvidence,
    pub preserved_non_goals: Vec<PreservedNonGoal>,
    pub forbidden_actions: Vec<String>,
    pub contradiction_count: usize,
    pub status: PatchPlanStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PatchTarget {
    MasterDoc(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PatchEdit {
    EvalExprJoin,
    BuildObjectJoin,
    BuildListJoin,
    FinishCarriesTaint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PatchPlanStatus {
    NeedsReconciliation,
    AlreadyConsistent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PreservedNonGoal {
    ControlFlowTaintV1NonGoal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RequiredEvidence {
    ConcreteArtifactOrPendingMarker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DocReconcileError {
    WrongWorkspace {
        path: PathBuf,
    },
    OutOfScopeChange {
        change_kind: String,
        path_or_operation: String,
    },
    StaleCleanOnlyTaintText {
        node: ResolvedNode,
        phrase: String,
    },
    UnsupportedEvidenceClaim {
        sentence: String,
        claim_kind: ClaimKind,
        required: RequiredEvidence,
    },
    TaintVocabularyConflict {
        conflict: ConflictKind,
        sentence: String,
        term: Option<String>,
    },
    ControlFlowTaintConflation {
        sentence: String,
    },
    MissingTraceability {
        clause: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResolvedNode {
    EvalExpr,
    BuildObject,
    BuildList,
    Finish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StalePhrase {
    EvalExprAlwaysClean,
    EvalExprNoOperandJoin,
    BuildObjectAlwaysClean,
    BuildObjectNoFieldJoin,
    BuildListAlwaysClean,
    BuildListNoItemJoin,
    WriteSlotOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClaimKind {
    TestEvidence,
    FormalEvidence,
    ReleaseReadiness,
    GeneratedParity,
    ImplementationEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConflictKind {
    WrongOrder,
    UnknownTerm,
    Downgrade,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContradictionReport {
    pub stale_clean_only: Vec<StalePhrase>,
    pub no_join_claims: Vec<StalePhrase>,
    pub write_slot_only_claims: Vec<StalePhrase>,
    pub scanned_nodes: Vec<ResolvedNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBoundedReport {
    pub unsupported_claims: Vec<String>,
    pub cited_claims: usize,
    pub pending_claims: usize,
    pub forbidden_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaintVocabularyReport {
    pub lattice: Vec<String>,
    pub propagation_rule: TaintVocabularyRule,
    pub conflicts: Vec<ConflictKind>,
    pub control_flow_scope: PreservedNonGoal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TaintVocabularyRule {
    JoinedDataFlowTaint,
}
