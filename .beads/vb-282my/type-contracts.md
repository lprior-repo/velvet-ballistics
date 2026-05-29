# Type Contracts — vb-282my

**Bead:** vb-282my (P1)
**Domain:** TLA bridge refinement harness types
**Date:** 2026-05-29

## Type Contract Checklist (from type-contract-checklist.md)

| Rule | Status | Notes |
|------|--------|-------|
| Replace stringly IDs with newtypes | ✅ Required | RroId, WaiverId, HarnessId are newtyped |
| Replace boolean behavior flags with enums | ✅ Required | BindingStatus, MappingStatus, ReviewerDisposition are enums |
| Replace `Option` lifecycle state with explicit variants | ✅ Required | RroLifecycle uses explicit `HarnessPending`, `HarnessReady`, `Closed` |
| Parse external input once at boundary | ✅ Required | RRO JSONL parsing at file boundary |
| Represent domain failures with semantic error variants | ✅ Required | BridgeError taxonomy per RRO |
| Keep pure core free of I/O, time, network, storage | ✅ Required | Bridge validation is I/O-free |

## Newtypes

```rust
/// Uniquely identifies a Rust refinement obligation row.
/// Format: RRO-TLA-{MODEL}-{NNN}, e.g., RRO-TLA-CHOOSE-LOWERING-001.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RroId(Box<str>);  // invariant: matches ^RRO-TLA-[A-Z]+-\d{3}$

impl RroId {
    /// Creates an RroId with format validation.
    /// Returns Err(RroIdError::InvalidFormat) on malformed input.
    pub fn new(raw: &str) -> Result<Self, RroIdError> { ... }

    /// Returns the model portion of the ID, e.g., "CHOOSE-LOWERING".
    pub fn model_name(&self) -> &str { ... }
}

/// Uniquely identifies a refinement harness artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HarnessId(Box<str>);

/// Uniquely identifies a proportional waiver.
/// Format: WAIVER-{RRO_ID}, e.g., WAIVER-RRO-TLA-RETRY-FSM-001.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WaiverId(Box<str>);

/// A production Rust source reference: crate-qualified path with symbol name.
/// Invariant: MUST contain a `::symbol_name` suffix, not just a file path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceRef {
    pub file: FilePath,
    pub line_range: LineRange,
    pub symbol: SymbolName,
}

/// A range of lines in a source file, 1-indexed inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
    pub start: u32,  // 1-indexed
    pub end: u32,    // 1-indexed
}

/// A Rust symbol name: function, method, type, or module-qualified.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolName(Box<str>);

/// A file path relative to workspace root.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FilePath(Box<str>);
```

## Enums — No Booleans, No Option Lifecycle

```rust
/// The kind of refinement harness artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessKind {
    /// A Kani proof harness (`#[kani::proof]`).
    KaniProof,
    /// A Flux refinement type (`#[sig]`, `#[refined_by]`).
    FluxRefinement,
    /// A Verus spec binding (`proof fn` + `exec fn` with `requires`/`ensures`).
    VerusSpec,
    /// A proptest property (`proptest!`).
    ProptestProperty,
}

/// The binding status of a refinement harness to a TLA+ claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingStatus {
    /// Harness exists but reviewer has not confirmed the binding.
    Unconfirmed,
    /// Harness covers a subset of the claim; gaps exist.
    Partial,
    /// Reviewer has confirmed the full claim is covered.
    Confirmed,
}

/// The per-RRO mapping status. Replaces raw string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingStatus {
    /// Obligation planned but no code written.
    Planned,
    /// Code/harness exists but not yet reviewer-approved.
    Materialized,
    /// Reviewer has approved the mapping.
    Verified,
    /// Incomplete; missing harness or waiver.
    Partial,
}

/// The reviewer's disposition on a harness or waiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewerDisposition {
    /// Not yet reviewed.
    Pending,
    /// Reviewer accepted the evidence.
    Accepted,
    /// Reviewer rejected; fixes required.
    Rejected,
}

/// The lifecycle state of a refinement obligation row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RroLifecycle {
    /// Row exists but no refinement harness or waiver is registered.
    HarnessPending,
    /// A harness is registered and the binding is confirmed, but not yet reviewer-approved.
    HarnessReady,
    /// Reviewer has accepted the bridge; row is closed.
    Closed { approved_by: InvocationRef },
}

/// The overall bridge verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeVerdict {
    /// All rows closed with harness/waiver and reviewer approval.
    Pass,
    /// One or more rows lack harness/waiver or reviewer approval.
    Rejected,
    /// Some rows closed, some pending.
    Partial,
}

/// A risk tag categorizing the correctness property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiskTag {
    Temporal,
    Concurrency,
    Persistence,
    PublicApi,
    UserVisibleBehavior,
}

/// The result of a TLC model check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TlcResult {
    pub states_generated: u64,
    pub distinct_states: u64,
    pub depth: u32,
    pub exit_code: i32,
}

impl TlcResult {
    /// TLC evidence is valid only when exit code is 0.
    pub fn is_pass(&self) -> bool {
        self.exit_code == 0
    }
}
```

## Smart Constructors — Parsers at Boundaries

```rust
/// RroId parser. Rejects malformed IDs at the boundary.
impl RroId {
    pub fn parse(raw: &str) -> Result<Self, RroIdError> {
        if !RRO_ID_RE.is_match(raw) {
            return Err(RroIdError::InvalidFormat { raw: raw.into() });
        }
        if raw.len() > RRO_ID_MAX_LEN {
            return Err(RroIdError::TooLong { len: raw.len(), max: RRO_ID_MAX_LEN });
        }
        Ok(Self(raw.into()))
    }
}

/// SourceRef parser. Rejects references that don't name a symbol.
impl SourceRef {
    pub fn parse(raw: &str) -> Result<Self, SourceRefError> {
        // Format: path:lines::symbol
        // Rejects file-only references without ::symbol
        let (path, rest) = split_file_path(raw)?;
        let (line_range, symbol) = split_line_symbol(rest)?;
        if symbol.as_ref().is_empty() {
            return Err(SourceRefError::MissingSymbol { raw: raw.into() });
        }
        Ok(Self { file: FilePath(path), line_range: line_range.parse()?, symbol })
    }
}

/// Verifies that a harness ref is distinct from a behavior test ref.
pub fn assert_distinct_harness(
    harness: &HarnessRef,
    behavior_tests: &[TestRef],
) -> Result<(), BridgeError> {
    for test_ref in behavior_tests {
        if harness.is_same_file_and_range(test_ref) {
            return Err(BridgeError::HarnessTestCollision {
                harness: harness.clone(),
                test: test_ref.clone(),
            });
        }
    }
    Ok(())
}
```

## Railway Error Taxonomy — Harness-Level

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeError {
    // --- RRO-level errors ---
    /// RRO ID does not match the expected format.
    RroIdInvalid { raw: String, reason: RroIdError },
    /// Source reference does not name a symbol.
    SourceRefMissingSymbol { raw: String },
    /// A behavior test ref is being used as a refinement harness.
    HarnessTestCollision { harness: HarnessRef, test: TestRef },
    /// A waiver covers a behavior-affecting claim.
    BehaviorAffectingWaiver { waiver_id: WaiverId, claim: String },
    /// An RRO row is missing required evidence.
    MissingEvidence { rro: RroId, missing: EvidenceKind },
    /// Self-stamped reviewer disposition.
    SelfApprovedReview { rro: RroId, reviewer: InvocationRef },
    /// Harness binding only covers a subset of the claim.
    IncompleteBinding { rro: RroId, covered: Vec<String>, missing: Vec<String> },
    /// Empty refinement_harness_refs with mapping_status: Verified.
    UnverifiedClosure { rro: RroId },
    /// mapping_status: Planned at State 12 (closure gate).
    PlannedAtClosure { rro: RroId },
    /// TLC evidence is the only implementation proof cited.
    TlaOnlyClosure { rro: RroId },

    // --- Harness-level errors ---
    /// Harness file not found or unreadable.
    HarnessNotFound { rro: RroId, file: String },
    /// Harness compilation failed.
    HarnessCompileFailure { rro: RroId, file: String, diagnostics: String },
    /// Harness verification returned counterexample.
    HarnessCounterexample { rro: RroId, file: String, trace: String },

    // --- Waiver-level errors ---
    /// Waiver missing required compensating evidence.
    WaiverMissingEvidence { waiver_id: WaiverId },
    /// Waiver expired.
    WaiverExpired { waiver_id: WaiverId, expiry: String },
    /// Waiver not reviewer-approved.
    WaiverNotApproved { waiver_id: WaiverId },

    // --- Transactional errors ---
    /// Attempt to close an RRO without bridging all source refs.
    UnbridgedSourceRef { rro: RroId, refs: Vec<SourceRef> },
    /// Mixed closure: some rows have harness, some have waiver, but closure is attempted as batch.
    MixedClosureStrategy { rro_ids: Vec<RroId> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    TlcPass,
    BehaviorTestPass,
    RefinementHarness,
    ProportionalWaiver,
    ReviewerApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RroIdError {
    InvalidFormat { raw: String },
    TooLong { len: usize, max: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceRefError {
    MissingSymbol { raw: String },
    InvalidLineRange { raw: String },
    InvalidFilePath { raw: String },
}
```

## Typestates — Harness Lifecycle

```rust
/// A refinement harness in its type-state machine.
mod harness_state {
    use super::*;

    /// Harness is registered but not yet compiled.
    pub struct HarnessRegistered {
        pub id: HarnessId,
        pub rro: RroId,
        pub kind: HarnessKind,
        pub file: FilePath,
        pub line_range: LineRange,
        pub symbol: SymbolName,
    }

    /// Harness compiles and passes verification.
    pub struct HarnessVerified {
        pub id: HarnessId,
        pub rro: RroId,
        pub kind: HarnessKind,
        pub file: FilePath,
        pub line_range: LineRange,
        pub symbol: SymbolName,
        pub evidence_command: String,
        pub exit_status: i32,
    }

    /// Harness binding is confirmed by independent reviewer.
    pub struct HarnessApproved {
        pub id: HarnessId,
        pub rro: RroId,
        pub kind: HarnessKind,
        pub file: FilePath,
        pub line_range: LineRange,
        pub symbol: SymbolName,
        pub evidence_command: String,
        pub reviewer: InvocationRef,
    }
}
```

## Key Type Guards

```rust
/// GUARD: A waiver must not cover behavior-affecting claims.
/// Returns Err(BridgeError::BehaviorAffectingWaiver) if behavior_affecting is true.
pub fn validate_waiver_behavior_scope(
    waiver: &ProportionalWaiver,
) -> Result<(), BridgeError> {
    if waiver.behavior_affecting {
        return Err(BridgeError::BehaviorAffectingWaiver {
            waiver_id: waiver.id.clone(),
            claim: waiver.claim.clone(),
        });
    }
    Ok(())
}

/// GUARD: An RRO must have a refinement harness or waiver before closing.
pub fn validate_rro_closure_prerequisites(
    rro: &RefinementObligation,
) -> Result<(), BridgeError> {
    if rro.mapping_status == MappingStatus::Planned {
        return Err(BridgeError::PlannedAtClosure { rro: rro.id.clone() });
    }
    let has_harness = !rro.refinement_harness_refs.is_empty();
    let has_waiver = rro.approved_waiver.is_some();
    if !has_harness && !has_waiver {
        return Err(BridgeError::MissingEvidence {
            rro: rro.id.clone(),
            missing: if rro.behavior_affecting {
                EvidenceKind::RefinementHarness
            } else {
                EvidenceKind::ProportionalWaiver
            },
        });
    }
    Ok(())
}

/// GUARD: Harness must cover the full claim. Partial bindings are errors.
pub fn validate_harness_claim_coverage(
    rro: &RefinementObligation,
    harness: &RefinementHarness,
) -> Result<(), BridgeError> {
    match harness.binding_status {
        BindingStatus::Confirmed => Ok(()),
        BindingStatus::Partial => Err(BridgeError::IncompleteBinding {
            rro: rro.id.clone(),
            covered: harness.covered_subclaims.clone(),
            missing: harness.missing_subclaims.clone(),
        }),
        BindingStatus::Unconfirmed => Err(BridgeError::IncompleteBinding {
            rro: rro.id.clone(),
            covered: vec![],
            missing: vec![rro.claim.clone()],
        }),
    }
}
```

## Canonical Harness Type Profiles

| RRO Row | Preferred Harness Kind | Alternative | Minimal Claim Coverage |
|---------|----------------------|-------------|----------------------|
| CHOOSE-LOWERING-001 | KaniProof | ProptestProperty | Exhaustive fanout (≤ 64 branches) and empty-branch rejection. |
| CHOOSE-REPLAY-001 | KaniProof | ProptestProperty | First-true-branch selection, otherwise fallback, no-match error. |
| ASK-ANSWER-001 | KaniProof | FluxRefinement | Journal monotonicity: AskScheduled must precede pending_timer; SlotWritten must precede AskAnswered. |
| RETRY-FSM-001 | KaniProof (extend existing) | — | Monotonicity (existing) + exhaustion under fairness + terminal typing. |
| RETRY-JOURNAL-001 | KaniProof | ProptestProperty | Key injectivity: (run, seq) maps to unique key; idempotent duplicate equality. |
| RESUME-001 | KaniProof | FluxRefinement | RuntimeState transitions: Resumed journaled before drive; append failure → Resumable; drive failure → preserves journal. |
| ADMISSION-001 | KaniProof | ProptestProperty | Append-before-insert: RunSubmitted/RunAdmission journaled before RunState::insert; append failure → AdmissionHeaderPersistenceFailed. |

## Illegal Type-Level States

- `RefinementObligation { mapping_status: Verified, refinement_harness_refs: [], approved_waiver: None }` — INV-4 violation
- `ProportionalWaiver { behavior_affecting: true }` — INV-3 violation (waiver MUST be rejected)
- `SourceRef { symbol: "" }` — INV-1 violation (parser must reject)
- `HarnessRef { file: "..." }` pointing to a file under `tests/` — INV-2 violation (harness-test collision guard)
- `BindingStatus::Confirmed` on a RetryFSM harness that only checks monotonicity — INV-8 violation (incomplete claim coverage)
- Self-referential reviewer invocation on `HarnessApproved` — INV-5 violation
