# Architectural Drift Report: `vb_storage::admission`

**File:** `crates/vb_storage/src/admission.rs`
**Line Count:** 1216 (LIMIT: 300)
**Status:** VIOLATION — MUST SPLIT

---

## Executive Summary

This file is **4× over budget** (1216 / 300 lines). It violates:
1. The `<300` line rule (SCOTTVLASCHIN / arch-drift mandate)
2. Single Responsibility Principle (workflow lumped with types)
3. Primitive Obsession (unguarded `u8`/`u32` domain scalars)

---

## Line Count Breakdown

| Section | Lines | Type | Within Limit? |
|---------|-------|------|----------------|
| Module docs + imports | 1–12 | Config | ✅ |
| `VerificationWarning` + impl | 13–44 | Value Object | ✅ |
| `ProofFlag` enum | 47–58 | Value Object | ✅ |
| `VerificationProof` struct | 61–91 | Entity | ✅ |
| `VerificationProofCore` | 94–112 | Internal | ✅ |
| `verification_proof_core` const fn | 114–129 | Factory | ✅ |
| `VerificationProof::new()` | 131–155 | Factory | ✅ |
| `AcceptedArtifact` | 157–196 | Entity | ✅ |
| `compute_policy_digest` | 198–218 | Service | ✅ |
| Constants + `submit_artifact` | 220–241 | Entry | ✅ |
| `submit_artifact_with_contracts` | 243–340 | **WORKFLOW (97 lines)** | ❌ |
| Helper fns (caps, idempotency) | 342–413 | Services | ✅ |
| `admit_compiled_artifact` | 415–454 | Workflow | ❌ |
| `mod tests` | 456–1216 | Tests | ❌ **760 lines** |

**Production code:** ~454 lines (admission + helpers)
**Test code:** ~760 lines (62% of file)

---

## 1. LINE COUNT VIOLATION

### Required Splitting

```
admission/
├── lib.rs                    # re-exports
├── types.rs                 # VerificationWarning, ProofFlag, VerificationProof*, AcceptedArtifact (≈200L)
├── policy_digest.rs         # compute_policy_digest (≈20L)
├── admission_workflow.rs    # submit_artifact, submit_artifact_with_contracts, admit_compiled_artifact (≈150L)
├── idempotency.rs           # idempotency evidence helpers (≈50L)
└── test_helpers.rs          # TestJournal, temp_journal, minimal_workflow (≈100L)
```

The **760-line test module** MUST be extracted into `tests/admission_tests.rs` or similar.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### Violation 1: `u8` for Gate Numbers
```rust
// Line 24
pub gate: u8,

// Line 29-31
pub const MIN_GATE: u8 = 1;
pub const MAX_GATE: u8 = 15;
```
**Problem:** Gate is a domain concept (1–15 validation gate ID). Using raw `u8` means any `u8` value is accepted; validation is manual via `is_valid()`.

**Fix:** NewType wrapper
```rust
pub struct VerificationGate(u8);
impl VerificationGate {
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 15;
    pub fn new(v: u8) -> Option<Self> { ... }
}
```

### Violation 2: `u32` for Warning Codes
```rust
// Line 20
pub code: u32,
```
**Problem:** Numeric error/warning codes are primitive. Should be an explicit `WarningCode` type.

### Violation 3: `u8` for Gate Count
```rust
// Line 72
pub gate_count: u8,

// Line 222
const ADMISSION_GATE_COUNT: u8 = 15;
```
**Problem:** Gate count is validated against a constant but passed as raw `u8`. Could be `GateCount(u8)` with bounded constructor.

### Violation 4: `bool` for Policy Flags
```rust
// Lines 74, 76-84
pub durable: bool,
pub bounded_claimed: bool,
pub taint_safe_claimed: bool,
// ...
```
**Problem:** Multiple `bool` fields named with `_claimed` suffix indicate unverified claims. The naming convention is a code smell — these should be a proper `ProofStatus` enum or structured type.

---

## 3. WORKFLOW / STATE MACHINE VIOLATIONS

### Violation: `submit_artifact_with_contracts` Handles 3 Policies

**Lines 253–339** — A single function with a `match` on 3 policies (`Relaxed`, `Journaled`, `Strict`) contains 3 divergent code paths with different validation requirements.

| Policy | Gates | Durable | Structure Check | Checksum Check |
|--------|-------|---------|-----------------|----------------|
| Relaxed | 0 | No | ❌ | ❌ |
| Journaled | 15 | No | ✅ | ✅ |
| Strict | 15 | Yes | ✅ | ✅ |

**Problem:** This is an implicit state machine. The DDD prescription is explicit transitions.

**Fix:** Extract 3 distinct workflow functions:
- `admit_relaxed()`
- `admit_journaled()`
- `admit_strict()`

### Violation: `_claimed` Suffix on VerificationProof Fields

Lines 76–84 contain:
```rust
pub bounded_claimed: bool,
pub taint_safe_claimed: bool,
pub retry_safe_claimed: bool,
pub idempotency_verified_claimed: bool,
pub replayable_claimed: bool,
```

**Problem:** The `_claimed` suffix is an explicit admission that these fields are **unverified claims not proven facts**. The struct comment on lines 62–66 confirms this is a known GAP (GAP-001). This is not DDD — it's a placeholder.

**DDD Prescription:** Either:
1. Remove `_claimed` fields entirely until real verification exists
2. Or model them as `UnverifiedProofFlags` / `ProofClaims` newtype

---

## 4. PARSE, DON'T VALIDATE

### Partial Compliance

`admit_compiled_artifact` (lines 423–454) and `submit_artifact_with_contracts` both use:
```rust
vb_core::CompiledWorkflow::try_from_parts(parts.clone())
    .map_err(|_| JournalError::ArtifactMalformed)?;
```

This is correct Parse-don't-Validate: re-construct the workflow from parts and fail if reconstruction fails.

**However:** The checksum validation (lines 291–293, 436–443) is redundant if `try_from_parts` already validates structure. Consider whether one gate suffices.

---

## 5. TEST MODULE INFRINGEMENT

**760 lines** of tests inside the production module is unacceptable.

Tests must be:
- In `tests/admission_tests.rs` (integration level)
- Or in `crates/vb_storage/tests/admission_tests.rs`

### Test Helper Blob (Lines 607–691)
`TestJournal`, `temp_journal()`, and `minimal_workflow()` are **test infrastructure** that should not live in production source.

---

## 6. GAP ANALYSIS (from code comments)

| GAP | Reference | Severity | Description |
|-----|-----------|----------|-------------|
| GAP-001 | Lines 62–66, 76–84 | **CRITICAL** | All proof flags unconditionally set to `true` — unverified claims |
| GAP-002 | Lines 159–165 | LOW | `source_digest` / `policy_digest` added for Backend DoD |
| GAP-003 | Lines 199–202 | LOW | Policy digest computation exists but derived field |
| GAP-004 | Lines 166–170 | INFO | Per-action digests intentionally NOT added |
| GAP-007 | Lines 187–193 | LOW | `accepted_at_seq` always 0 — placeholder |

---

## 7. ACTION ITEMS (Priority Order)

| Priority | Action | Complexity |
|----------|--------|------------|
| P0 | **Extract tests** → `tests/admission_tests.rs` | Low |
| P0 | **Split production code** into `types.rs`, `workflow.rs`, `idempotency.rs` | Medium |
| P1 | NewType `VerificationGate(u8)` replacing raw `u8` | Medium |
| P1 | NewType `WarningCode(u32)` replacing raw `u32` | Medium |
| P1 | Split `submit_artifact_with_contracts` into 3 explicit policy functions | Medium |
| P2 | Remove or properly model `_claimed` fields | High |
| P2 | Extract `VerificationProofCore` / `IdempotencyEvidence` to separate files | Low |

---

## 8. DDD BOUNDED CONTEXT MAP

```
vb_storage::admission
├── Value Objects:    VerificationWarning, ProofFlag
├── Entities:        VerificationProof, AcceptedArtifact
├── Services:        compute_policy_digest, idempotency evidence helpers
├── Workflows:       submit_artifact, submit_artifact_with_contracts, admit_compiled_artifact
└── Excluded:        Tests (760 lines — must move)
```

---

## 9. VERDICT

| Rule | Status |
|------|--------|
| `<300` line limit | **VIOLATION** (1216 lines, 4× over) |
| Primitive obsession | **VIOLATION** (`u8` gate, `u32` code, `u8` gate_count) |
| Single Responsibility | **VIOLATION** (test blob + 3 workflow policies in 1 fn) |
| State machine as functions | **PARTIAL** (implicit state via match, not explicit) |
| Parse, don't validate | **PARTIAL** (correct pattern, but redundant gates exist) |
| No `unsafe` | ✅ Clean |

**Required Resolution:** Split file into 4+ modules. Extract tests. Create NewTypes for domain scalars.
