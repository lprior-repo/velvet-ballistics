# ARCHITECTURAL DRIFT HAMMER REPORT

**Target:** `vb_proof_kernels/src/vb_kyyf_normalization.rs`
**Total Lines:** 653
**Line Limit:** 300
**Violation Ratio:** 2.18x

**Status:** REFACTOR REQUIRED

---

## EXECUTIVE SUMMARY

This file is a **proof kernel** that handles cross-run and replay comparison normalization. It contains Verus formal specs alongside runtime Rust code in a dual-block structure. Despite the proof-kernel framing, it commits severe architectural violations: primitive obsession on 9 `u64` signature fields, massive duplication between verus/non-verus blocks, and macro-overuse that obscures domain logic.

---

## 1. LINE COUNT VIOLATION

| Metric | Value |
|--------|-------|
| Total lines | 653 |
| Limit | 300 |
| Overage | 353 lines (118%) |

**Breakdown by concern:**
- Macro definitions: ~140 lines (lines 9-146)
- Verus spec block: ~280 lines (lines 152-432)
- Cargo-kernel runtime block: ~140 lines (lines 435-574)
- Tests: ~73 lines (lines 579-653)

**The duplication is the primary size driver.** The verus block and cargo_kernel block are nearly identical copies of the same types and functions with different syntax.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Signature Field Proliferation

`PublicObservation` and `NormalizedObservation` use raw `u64` for **9 distinct semantic concepts**:

```rust
pub struct PublicObservation {
    // ... other fields ...
    pub event_signature: u64,              // ❌ EventSignature(u64)
    pub event_payload_signature: u64,      // ❌ EventPayloadSignature(u64)
    pub semantic_slot_signature: u64,       // ❌ SemanticSlotSignature(u64)
    pub semantic_action_signature: u64,     // ❌ SemanticActionSignature(u64)
    pub semantic_taint_signature: u64,      // ❌ SemanticTaintSignature(u64)
    pub temp_path_signature: u64,           // ❌ TempPathSignature(u64)
    pub process_id_signature: u64,          // ❌ ProcessIdSignature(u64)
    pub wall_clock_signature: u64,          // ❌ WallClockSignature(u64)
    pub generated_run_signature: u64,       // ❌ GeneratedRunSignature(u64)
}
```

**Scott Wlaschin DDD Violation:** These are semantically distinct concepts (events, slots, actions, taint, paths, process ID, time, generated runs) all collapsed into undifferentiated `u64`. This allows mixing:
- `observation.event_signature = my_process_id_signature` — LEGAL in current code, ILLEGAL in domain

### 2.2 Boolean Primitive Obsession in DigestStatus

```rust
pub struct DigestStatus {
    pub workflow_source_matches: bool,  // ❌ Should be WorkflowSourceMatch(bool)
    pub compiled_ir_matches: bool,      // ❌ Should be CompiledIrMatch(bool)
    pub action_abi_matches: bool,       // ❌ Should be ActionAbiMatch(bool)
    pub policy_matches: bool,           // ❌ Should be PolicyMatch(bool)
}
```

While `bool` is technically a NewType here, bundling 4 booleans into a struct without a domain-specific wrapper means callers can construct `DigestStatus { workflow_source_matches: true, compiled_ir_matches: false, ... }` without semantic intent. A proper `DigestStatus` would use a `MatchStatus` enum or similar.

---

## 3. DDD STRUCTURAL VIOLATIONS

### 3.1 Value Objects Without Validation

`TerminalResult` and `TaintStatus` are proper enums (good), but the **comparison functions** are implemented manually instead of deriving equality:

```rust
fn terminal_results_equal(left: TerminalResult, right: TerminalResult) -> bool {
    terminal_results_equal_body!(left, right)
}

fn taint_statuses_equal(left: TaintStatus, right: TaintStatus) -> bool {
    taint_statuses_equal_body!(left, right)
}
```

**DDD Principle Violation:** For value objects with finite variants, `derive(Eq)` would be simpler and safer. The manual match arms add no semantic value.

### 3.2 Workflow Logic Buried in Macros

`compare_replay` implements a **state machine**:

```
1. Check digest_status.all_match() → Err(ReplayDigestMismatch) if false
2. Check replay_policy_blocked → Err(ReplayPolicyBlocked) if true
3. Check event_signature equality → Err(ReplaySequenceViolation) if false
4. Compare normalized observations → Err(NondeterministicObservation) if false
5. Ok(())
```

This is buried inside a macro (`compare_replay_body!`) with no explicit state representation. A proper DDD approach:

```rust
enum ReplayCheckState {
    CheckingDigest,
    CheckingPolicy,
    CheckingSequence,
    CheckingSemantics,
    Passed,
}
```

---

## 4. DUPLICATION ANALYSIS

### 4.1 Verus Block vs Cargo Kernel Block

The file contains **identical type definitions in two separate blocks:**

| Element | Verus Block (L152-432) | Cargo Kernel (L435-574) |
|---------|------------------------|-------------------------|
| `TerminalResult` | L154-160 | L437-443 |
| `TaintStatus` | L162-167 | L446-451 |
| `DeterminismError` | L169-177 | L454-462 |
| `DigestStatus` | L179-203 | L465-478 |
| `PublicObservation` | L205-222 | L481-498 |
| `NormalizedObservation` | L224-237 | L501-514 |
| All functions | L334-430 | L516-573 |

**Estimated duplication: ~280 lines of identical code**

### 4.2 Macro Duplication

```rust
// These two macros are 90% identical
normalized_observations_equal_body!  // L38-56
generated_ir_observations_equal_body! // L118-135
```

Difference: `generated_ir_observations_equal_body!` omits `semantic_taint_signature` comparison.

---

## 5. NORMALIZATION RESPONSIBILITY MAP

| Responsibility | Current Location | Issue |
|----------------|------------------|-------|
| Filter runtime noise (temp_path, process_id, wall_clock) | `normalize_observation()` | OK - correctly strips `PublicObservation` to `NormalizedObservation` |
| Compare terminal results | `terminal_results_equal()` | Manual match, should derive |
| Compare taint status | `taint_statuses_equal()` | Manual match, should derive |
| Compare event sequences | `compare_replay()` | Buried in macro, state machine implicit |
| Compare cross-run semantics | `compare_cross_run()` | Good abstraction |
| Compare generated IR | `compare_generated_ir()` | Good abstraction |

---

## 6. MACRO OVERUSE ASSESSMENT

| Macro | Purpose | Assessment |
|-------|---------|------------|
| `digest_all_match_body!` | Extract `all_match` boolean | Simple enough to inline |
| `normalize_observation_body!` | Field copy | Simple enough to inline |
| `normalized_observations_equal_body!` | 14-field comparison | Too complex for macro |
| `terminal_results_equal_body!` | Enum variant comparison | Should use `derive(Eq)` |
| `taint_statuses_equal_body!` | Enum variant comparison | Should use `derive(Eq)` |
| `compare_replay_body!` | 4-step state machine | Should be explicit function |
| `compare_generated_ir_body!` | 3-step check | Should be explicit function |
| `generated_ir_observations_equal_body!` | 13-field comparison | Too complex for macro |
| `compare_normalized_observations_body!` | Simple wrapper | Delete, inline |

**Total: 9 macros for a 653-line file. Average: 1 macro per 72 lines.**

---

## 7. RECOMMENDED REFACTORING

### 7.1 NewType Signatures (9 types)

```rust
pub struct EventSignature(u64);
pub struct EventPayloadSignature(u64);
pub struct SemanticSlotSignature(u64);
pub struct SemanticActionSignature(u64);
pub struct SemanticTaintSignature(u64);
pub struct TempPathSignature(u64);
pub struct ProcessIdSignature(u64);
pub struct WallClockSignature(u64);
pub struct GeneratedRunSignature(u64);
```

### 7.2 File Splitting (Target: 5 files)

1. **types.rs** (~100 lines): All enum/type definitions with newtypes
2. **normalization.rs** (~80 lines): `normalize_observation()` only
3. **comparison.rs** (~150 lines): All comparison functions, no macros
4. **verus_spec.rs** (~200 lines): Verus specs only
5. **tests.rs** (~73 lines): Existing tests

### 7.3 Replace Macros with Functions

- `terminal_results_equal` → derive `Eq` on `TerminalResult`
- `taint_statuses_equal` → derive `Eq` on `TaintStatus`
- `compare_replay_body!` → explicit `fn check_replay_sequence()` state machine

---

## 8. VERDICT

| Violation | Severity | Fix Effort |
|-----------|----------|------------|
| Line count 2.18x limit | CRITICAL | High (split + dedupe) |
| 9 primitive `u64` signatures | CRITICAL | Medium (newtypes) |
| 280 lines duplicated | HIGH | High (extract to shared) |
| Macro overuse (9 macros) | MEDIUM | Low (inline/derive) |
| Implicit state machine | MEDIUM | Low (extract states) |

**Overall: REFACTOR REQUIRED**

This file requires decomposition before it can pass architectural review. The proof kernel duality (verus + runtime) is architecturally sound in principle but needs shared types extracted to eliminate duplication.

---

*Generated by arch-drift-hammer on 2026-05-29*
