# Proof Strategy — vb-2bzz

**Bead**: `vb-2bzz` — storage: expose action ABI and policy digest recovery mismatch checks
**Phase**: 3 | **Attempt**: 1-of-7
**Risk tags**: api-surface, recovery, release-blocker
**Date planned**: 2026-05-19

---

## 1. Current State Analysis

### What exists (pre-wiring)

| Component | Location | Status |
|---|---|---|
| `RecoveryError::ActionAbiMismatch { action_id }` | `types.rs:41-44` | Variant exists |
| `RecoveryError::PolicyDigestMismatch { step }` | `types.rs:46-50` | Variant exists |
| `DigestCheck::Full` | `types.rs:374` | Variant exists |
| `check_action_abi_digests` | `recover.rs:109-120` | Function exists, tested in isolation |
| `check_policy_digests` | `recover.rs:128-137` | Function exists, tested in isolation |
| `check_action_abi_digest` | `recover.rs:54-64` | Single-entry function exists |
| `check_policy_digest` | `recover.rs:67-77` | Single-entry function exists |
| `verify_digests` | `recover.rs:83-101` | Does NOT call `check_action_abi_digests` or `check_policy_digests` |
| `recover_full_journal` | `core.rs:150-173` | Accepts `_expected_action_abi_digests` — parameter discarded (underscore prefix) |
| `replay_events` | `core.rs:34-119` | Accepts `_expected_action_abi_digests` — parameter discarded |
| GAP-3 BDD tests | `recovery_bdd_tests.rs:1751-1839` | Exist, un-ignored, but test standalone functions only |
| `verify_digests` at `DigestCheck::Full` | `recovery_bdd_tests.rs` | No behavioral test exists |

### Key Gaps

1. **`verify_digests` does not call `check_action_abi_digests` or `check_policy_digests` at any level** — Even `DigestCheck::Full` only checks workflow source and compiled IR. The `Full` variant's intent (verify "all digests including action ABI and policy" per `types.rs:374-375`) is not implemented.

2. **`recover_full_journal` discards `_expected_action_abi_digests`** — The underscore prefix signals the parameter is accepted but unused. The ABI digest check never reaches `replay_events`.

3. **No integration test wires the new error paths through the public recovery API** — BDD tests cover the standalone `check_*_digests` functions but not the call chain: public API → `verify_digests` (Full) → `check_action_abi_digests` / `check_policy_digests`.

4. **Contract says "no formal proof required"** — The contract classifies this as an API surface change with unit tests as sufficient evidence. However, the wiring gap is a correctness bug (silently discarding verifier inputs), which elevates the risk beyond a simple surface change.

---

## 2. Risk Classification

| # | Risk | Severity | Trigger |
|---|---|---|---|
| R1 | **Reachability gap** — `ActionAbiMismatch` and `PolicyDigestMismatch` not reachable through `verify_digests` or `recover_full_journal` | HIGH | `release-blocker` tag; `DigestCheck::Full` documented but not wired |
| R2 | **Parameter discarding** — `_expected_action_abi_digests` silently dropped in `recover_full_journal` | HIGH | Underscore-prefixed parameter; silent correctness failure |
| R3 | **False sense of security** — `DigestCheck::Full` exists but does not check ABI/policy digests | MEDIUM | Consumer may assume Full = all digests verified |
| R4 | **ABI digest parameter not passed through `replay_events`** | MEDIUM | `replay_events` also discards `_expected_action_abi_digests` |

**No concurrency risk** — recovery functions are single-threaded, no `Mutex`, `RwLock`, or async.
**No unsafe risk** — all files have `#![forbid(unsafe_code)]`.
**No panic risk in production code** — panics exist only in test code.

---

## 3. Selected Verifier Lanes

### Lane A: Unit Tests (Extended Integration)

**Scope**: Wire `verify_digests` → `check_action_abi_digests` / `check_policy_digests` at `DigestCheck::Full`. Wire `recover_full_journal` → pass ABI digests through to `replay_events`. Test error paths through the full recovery API.

**Why**: This is the primary verification for wiring correctness. The contract already mandates unit tests.

**Planned tests**:
- `verify_digests_at_full_level_checks_abis_and_policies` — verify `verify_digests` with `DigestCheck::Full` calls both `check_*_digests` functions and returns correct error types on mismatch
- `verify_digests_full_no_false_positives_on_match` — matching ABIs and policies return Ok at Full level
- `verify_digests_empty_abis_policy_returns_ok` — empty expected inputs at Full level do not guess
- `recover_full_journal_passes_abis_to_replay` — verify `_expected_action_abi_digests` is now used (not discarded)
- `recover_full_journal_abi_mismatch_propagates` — ABI mismatch in entries propagates as `ActionAbiMismatch`
- GAP-3 scenarios: un-ignore and verify full integration

### Lane B: Kani (Panic-Freedom on New Paths)

**Scope**: Prove `check_action_abi_digests` and `check_policy_digests` panic-free for all `WorkflowDigest` inputs. Prove `verify_digests` with `DigestCheck::Full` does not panic on any digest combination.

**Why**: New error paths must be proven panic-free before release. Kani's bounded model checking is the right tool for exhaustively exploring digest comparison paths.

**Artifacts**: `crates/vb_storage/src/kani_digest_checks.rs` (new) or append to existing `kani_codec.rs`.

**Harnesses**:
- `kani_check_action_abi_digests_panic_free` — generate arbitrary digest tuples, prove no panic
- `kani_check_policy_digests_panic_free` — generate arbitrary digest tuples, prove no panic
- `kani_verify_digests_full_panic_free` — generate arbitrary inputs at Full level, prove no panic

**Bounds**: `WorkflowDigest` is 32 bytes — Kani must model the full byte representation. No loop unwind limits needed (functions iterate over caller-provided slices, Kani uses `kani::any_vec` for slice generation).

### Lane C: Proptest (Error Taxonomy Exhaustive)

**Scope**: Property-based tests covering all error taxonomy combinations.

**Why**: Ensures the error taxonomy is closed — every valid input combination maps to exactly one error outcome, and no unexpected errors are produced.

**Properties**:
- Empty input → Ok for both functions
- Single match → Ok
- Single mismatch → correct error variant
- Multiple entries, first matches, second mismatches → error on second
- Multiple entries, all match → Ok
- Duplicate action_ids with mixed results → error on first mismatch
- Policy digests with zero step → correct error
- Large input vectors → no panic, correct result
- Cross-type interference: passing ABI entries to `check_policy_digests` (type error at compile time — no need to test)

### Lane D: TLA+ (Verification Ordering Invariant)

**Scope**: Model the `DigestCheck` enum and verify that `verify_digests` respects the level hierarchy: `WorkflowSourceOnly ⊂ WorkflowAndIr ⊂ Full`. Prove that a higher level always includes checks from lower levels.

**Why**: The `DigestCheck` enum defines a strict hierarchy. Consumers may rely on `Full` being a superset. A TLA+ model proves this invariant holds structurally.

**Artifact**: `.beads/vb-2bzz/specs/verify_digests_ordering.tla`

**Invariant**: `FullChecks ⊇ WorkflowAndIrChecks ⊇ WorkflowSourceOnlyChecks`

---

## 4. Waivers (Not-Applicable Lanes)

| Lane | Reason |
|---|---|
| Verus | No formal specification required; contract says unit tests suffice |
| Flux | No refinement types or type-state constraints in scope |
| Loom | No concurrency in recovery functions |
| Miri | No `unsafe` code; `#![forbid(unsafe_code)]` everywhere |
| Fuzz | Functions are pure comparisons over small slices; proptest covers the same space more effectively |
| Property-based mutation testing | Too low signal for pure digest comparison functions |

---

## 5. Obligation Summary

| ID | Requirement | Verifier | Status |
|---|---|---|---|
| PO-001 | EARS-1 + EARS-2: mismatch error paths reachable through `verify_digests` | unit-test | planned |
| PO-002 | EARS-3: `verify_digests` Full level calls ABI/policy checks | unit-test | planned |
| PO-003 | INV-1+INV-2: empty inputs return Ok | unit-test | planned |
| PO-004 | INV-4: error carries exact action_id/step | unit-test | planned |
| PO-005 | Gap: `recover_full_journal` passes ABI digests (not discarded) | unit-test | planned |
| PO-006 | R1: `verify_digests` Full-level ABI/policy checks produce correct errors | kani | planned |
| PO-007 | R1: `verify_digests` Full-level panic-freedom | kani | planned |
| PO-008 | R3: `DigestCheck` hierarchy invariant | tla-plus | planned |
| PO-009 | Error taxonomy completeness | proptest | planned |

Total planned obligations: **9** (5 unit-test, 2 Kani, 1 TLA+, 1 proptest)
