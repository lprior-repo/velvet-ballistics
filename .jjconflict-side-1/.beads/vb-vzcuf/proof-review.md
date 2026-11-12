# Proof Review: vb-vzcuf State 6 (Attempt 2)
reviewer_skill: proof-reviewer
reviewer_invocation_id: vb-vzcuf-state6-proof-reviewer-attempt2
prior_reviewer_invocation_id: vb-vzcuf-state6-proof-reviewer-attempt1

## Metadata
- **Reviewer skill:** proof-reviewer
- **Reviewer invocation:** vb-vzcuf-state6-proof-reviewer-attempt2
- **Review state:** 6
- **Proof-writer invocation:** vb-vzcuf-state5-proof-writer-attempt2
- **Proof-plan-review status:** APPROVED (state 4)
- **Prior review status:** REJECTED (state 6 attempt 1 — 5 LETHAL findings)
- **Workspace:** /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-vzcuf
- **Source checkout (control plane):** /home/lewis/src/velvet-ballistics
- **Date:** 2026-05-29

## Scope Reviewed
45 proof obligations across 9 proof seeds (PS-001 through PS-009), covered by 5 verifiers: 9 Verus, 9 Kani, 9 Flux-rs, 9 proptest, 9 cargo-fuzz. TLA+ globally removed per plan (approved).

## Reviewed Artifacts
- verification/verus/vb-vzcuf-PS-001.rs through PS-009.rs (9 files)
- verification/kani/vb-vzcuf-PS-001.rs through PS-009.rs (9 files)
- verification/flux/vb-vzcuf-PS-001.rs through PS-009.rs (9 files)
- crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs through PS_009.rs (9 files)
- fuzz/fuzz_targets/vb_vzcuf_PS_001.rs through PS_009.rs (9 files)
- crates/vb_storage/src/batch.rs (production code — NO Verus annotations)
- crates/vb_storage/src/error/mod.rs (production code — NO Verus annotations)
- crates/vb_storage/src/codec/mod.rs (production code — NO Verus annotations)
- crates/vb_core/src/ (production code — NO Verus annotations)
- .beads/vb-vzcuf/proof-evidence.md
- .beads/vb-vzcuf/proof-writer-report.md
- .beads/vb-vzcuf/trusted-base-ledger.jsonl
- .beads/vb-vzcuf/agent-invocation-ledger.jsonl
- .beads/vb-vzcuf/proof-obligations.planned.jsonl
- .beads/vb-vzcuf/proof-review.md (attempt 1)
- .beads/vb-vzcuf/proof-findings.jsonl (attempt 1)
- .beads/vb-vzcuf/contract.md

## Verus Smoke Evidence (Raw command output — reviewer executed)

```bash
$ verus --crate-type=lib verification/verus/vb-vzcuf-PS-001.rs
verification results:: 7 verified, 0 errors

$ verus --crate-type=lib verification/verus/vb-vzcuf-PS-002.rs
verification results:: 11 verified, 0 errors

$ verus --crate-type=lib verification/verus/vb-vzcuf-PS-003.rs
verification results:: 5 verified, 0 errors

$ verus --crate-type=lib verification/verus/vb-vzcuf-PS-004.rs
verification results:: 5 verified, 0 errors

$ verus --crate-type=lib verification/verus/vb-vzcuf-PS-005.rs
verification results:: 9 verified, 0 errors

$ verus --crate-type=lib verification/verus/vb-vzcuf-PS-006.rs
verification results:: 6 verified, 0 errors

$ verus --crate-type=lib verification/verus/vb-vzcuf-PS-007.rs
verification results:: 5 verified, 0 errors

$ verus --crate-type=lib verification/verus/vb-vzcuf-PS-008.rs
verification results:: 7 verified, 0 errors

$ verus --crate-type=lib verification/verus/vb-vzcuf-PS-009.rs
verification results:: 6 verified, 0 errors

Total: 61 proofs verified, 0 errors across 9 files
```

Verus version: 0.2026.05.05.d03e906 (installed, functional).

## Adversarial Review

### LEGACY TRACKING: Attempt 1 findings and their resolution status

| Finding | Severity | Status in Attempt 2 |
|---------|----------|---------------------|
| LF1: Zero smoke evidence | LETHAL | PARTIALLY RESOLVED — Verus smoke verified; Kani/Flux/Fuzz still unexecuted |
| LF2: Verus specs disconnected (GOD RULE 2) | LETHAL | UNRESOLVED — see LF1 below |
| LF3: Kani copies production logic | LETHAL | PARTIALLY RESOLVED — some harnesses now import production types |
| LF4: Flux on models only | LETHAL | UNRESOLVED — see LF4 below |
| LF5: Self-approved TBPs | LETHAL | UNRESOLVED — see LF5 below |
| HF6: C9 no obligation | HIGH | UNRESOLVED — see MF2 below |
| MF7: PS-004 hardcoded lemmas | MEDIUM | UNRESOLVED — see MF1 below |
| MF8: PS-001 concrete lemmas | MEDIUM | UNRESOLVED — see MF3 below |
| MF9: No cover! in Kani | MEDIUM | UNRESOLVED — see MF4 below |
| LF10: Proptest/fuzz copy models | LOW | RESOLVED — proptest exercises production JournalWriteBatch; fuzz calls encode_record |

---

### LETHAL FINDING 1: GOD RULE 2 violation — Verus specs not mathematically bound to production exec fn
**Severity:** LETHAL (blocks approval)
**Affected obligations:** POB-vb-vzcuf-001, 005, 009, 013, 017, 021, 025, 029, 033 (all 9 Verus obligations)
**Evidence:**
- Production code grep for `requires|ensures|verus!` in `crates/vb_storage/src/` and `crates/vb_core/src/`: **ZERO matches** (only English prose "requires" in comments, no Verus annotations)
- `JournalWriteBatch` struct (batch.rs:38-45) has no `staged_bytes`, no `byte_limit`, no Verus annotations
- `append_event` (batch.rs:209-229) has no `requires`/`ensures`
- All 9 Verus files define standalone `spec fn`/`proof fn` in `verification/verus/` with "PRODUCTION BINDING:" comments but zero mathematical binding to production `exec fn`

**Specific GOD RULE 2 violations:**

- **PS-003** (error distinctness, C4/C6): Defines `ErrorVariant` enum in `verification/verus/` and proves `ErrorVariant::AccumulatedBytesExceeded != ErrorVariant::QueueFull`. This is EXACTLY the pattern GOD RULE 2 forbids: "You cannot define an enum in verification/verus/, prove its properties by(compute), and call it a day." The production `JournalError` enum in `error/mod.rs:20-247` is a DIFFERENT type.

- **PS-008** (guard precedence, C6): Defines local `Guard` enum and `guard_index()` spec. Proves `guard_precedence_order()` by asserting `0 < 1 < 2 < 3 < 4` on locally-defined values — a tautology.

- **PS-001** (admission, C3): Defines `admit_bytes` as `open spec fn` commented with "PRODUCTION BINDING: Models JournalWriteBatch::append_event admission check" but the production function has no Verus contract and no `requires`/`ensures`.

- **PS-004** (state preservation, C5): Defines `BatchState` struct separate from production `JournalWriteBatch`. Lemmas prove properties of `BatchState`, not `JournalWriteBatch`.

GOD RULE 2 text: "Verus proof fn and spec fn models MUST mathematically bind to the actual Rust implementations (exec fn) inside the production codebase."

The proof-writer-report claims "GOD RULE 2: IMPROVED" and cites "explicit production binding annotations referencing" production locations. But comments referencing a file path are NOT mathematical binding. GOD RULE 2 requires `requires`/`ensures` annotations on the actual `exec fn` in the production crate, verified by Verus. Adding PRODUCTION BINDING comments to standalone spec models does not satisfy this requirement.

**Required fix:** Add `requires`/`ensures` annotations to the actual production `exec fn` in `crates/vb_storage/src/batch.rs`, `error/mod.rs`, `codec/mod.rs` and verify them with Verus. Or provide a structural blocker document explaining why production binding is infeasible at this stage with a concrete plan and compensating evidence (e.g., proptest pass is stronger evidence than standalone Verus models).

---

### LETHAL FINDING 2: Trusted base entries self-approved by proof-writer
**Severity:** LETHAL
**Affected:** TBP-001 through TBP-009
**Evidence:**
- All 9 entries in `trusted-base-ledger.jsonl` have `owner: "proof-writer"`, `reviewer_disposition: "accepted"`
- Proof-reviewer skill: "Verify reviewer provenance with agent-invocation-ledger.jsonl; reject self-approval."
- TBP-006 (`kind: future_implementation`, "JournalWriteBatch will gain staged_bytes: u64") — accepts as trusted that code which does not exist will be implemented correctly
- TBP-007 (`kind: future_implementation`, "JournalError will gain AccumulatedBytesExceeded variant") — same pattern
- TBP-006 compensating_evidence field cites "Verus specs model the fields" — the Verus specs that violate GOD RULE 2

**Required fix:** Reset TBP entries to `status: "pending"` pending independent review. TBP-006 and TBP-007 require the implementation to exist before they can be accepted; they are circular.

---

### LETHAL FINDING 3: Verus PS-003 and PS-008 are tautological — proving properties of locally-defined types
**Severity:** LETHAL
**Affected obligations:** POB-vb-vzcuf-009 (PS-003), POB-vb-vzcuf-029 (PS-008)

**PS-003 (ErrorVariant):**
The spec defines `ErrorVariant::AccumulatedBytesExceeded`, `ErrorVariant::QueueFull`, `ErrorVariant::PayloadTooLarge` as a local enum. Then proves `ErrorVariant::AccumulatedBytesExceeded != ErrorVariant::QueueFull`. In Rust, two distinct enum variants with different names are ALWAYS distinct by construction — this is a tautology of Rust's type system. Proving it "by Verus" provides zero information about production code.

Furthermore, the production `JournalError` enum has 28+ variants (error/mod.rs:20-247). Proving that three variants in a locally-defined 3-variant enum are distinct says nothing about whether the production `JournalError` will have distinguishable `AccumulatedBytesExceeded`, `QueueFull`, and `PayloadTooLarge` variants — especially since the `AccumulatedBytesExceeded` variant doesn't exist in production yet.

**PS-008 (Guard):**
Defines `Guard` enum with 5 variants and assigns indices 0-4. Then `guard_precedence_order()` asserts `0 < 1 < 2 < 3 < 4` which is trivially true. The lemma `lemma_guard_precedence_well_ordered()` proves this tautology with an empty body. This is "proof by definition" — the spec defines what the ordering is, then proves the ordering matches the definition.

**Required fix:** PS-003 should prove properties of the actual production `JournalError` enum, not a locally-defined `ErrorVariant`. PS-008 should verify actual guard ordering from the production `append_event` method's control flow, not a spec-defined order.

---

### LETHAL FINDING 4: Production code lacks the fields and variants being verified
**Severity:** LETHAL
**Affected obligations:** All 45 (blocker affects every obligation that depends on `staged_bytes`, `byte_limit`, or `AccumulatedBytesExceeded`)
**Evidence:**
- `JournalWriteBatch` (batch.rs:38-45) has fields: `inner`, `journal`, `staged_event_keys`, `aborted`, `_not_send_or_sync`
- It does NOT have: `staged_bytes: u64`, `byte_limit: u64`
- `JournalError` (error/mod.rs:20-247) has ~28 variants but no `AccumulatedBytesExceeded` variant
- TBP-006 and TBP-007 admit these are "future_implementation"
- Proof-writer report acknowledges: "REMAINING GAP: The production implementation does not yet have requires/ensures annotations" and "The production implementation must add these fields"

The verification artifacts model behavior for code that does not exist. When that code is implemented, its actual behavior may diverge from the model. This is not a mere gap — it means the verification provides zero assurance about the current production codebase.

**Required fix:** Either implement the required fields and variants in production code, or acknowledge this as a structural blocker and reduce the scope of claims that can be considered "verified."

---

### HIGH FINDING 1: Verus PS-004 lemmas prove properties for trivial/single cases, not forall
**Severity:** HIGH
**Affected obligations:** POB-vb-vzcuf-013 (Verus PS-004)
**Evidence:**
- `lemma_rejection_state_reflexive(state: BatchState)` proves `state_unchanged_after_rejection(state, state)` — this is reflexivity, not preservation across different states. It proves `state == state`, which is an identity tautology.
- `lemma_acceptance_updates_state(before, after, added_bytes)` requires `state_updated_after_acceptance(before, after, added_bytes)` as a precondition and then concludes `after.staged_bytes > before.staged_bytes || added_bytes == 0`. This deducts from the requirement rather than proving that acceptance always updates the state correctly.
- `lemma_aborted_batch_no_commit()` proves `!(BatchState { aborted: true }.aborted) == false` — proves that `!true == false`, a Rust boolean tautology.

These are not meaningful proofs. They are weak lemmas that either restate preconditions or prove trivial boolean identities. They do not establish that rejection preserves state for all possible inputs, only for identical pre/post states.

---

### MEDIUM FINDING 1: Verus PS-001 lemmas prove single concrete cases, not general properties
**Severity:** MEDIUM
**Affected obligations:** POB-vb-vzcuf-001 (Verus PS-001)
**Evidence:**
- `lemma_exact_fit_accepted()` proves `admit_bytes(500_000, 548_576, 1_048_576).is_ok()` — one specific case
- `lemma_over_limit_rejected()` proves `admit_bytes(1_000_000, 100_000, 1_048_576).is_err()` — one specific case
- `lemma_overflow_rejected()` proves `admit_bytes(u64::MAX, 1, u64::MAX).is_err()` — one specific case

These amount to unit tests encoded as Verus lemmas, not general mathematical proofs.

---

### MEDIUM FINDING 2: Contract clause C9 (Observability) still has no dedicated proof obligation
**Severity:** MEDIUM
**Affected:** Contract clause C9
**Evidence:** Same as attempt 1 finding HF6. The 45-entry planned obligations list has no entry with `requirement_id: "C9"`. Traceability matrix maps C9 to PS-004 and PS-005, but those target C5 and C2 respectively.

---

### MEDIUM FINDING 3: No `cover!()` statements in any Kani harness
**Severity:** MEDIUM
**Affected:** All 9 Kani obligations (POB-vb-vzcuf-002, 006, 010, 014, 018, 022, 026, 030, 034)
**Evidence:** Same as attempt 1 finding MF9. `grep -r 'kani::cover' verification/kani/vb-vzcuf-PS-*.rs` would return zero matches. Without cover, it is impossible to distinguish reachable assertions from vacuously true ones.

---

### MEDIUM FINDING 4: Flux annotations on standalone functions, not production types
**Severity:** MEDIUM
**Affected:** All 9 Flux obligations (POB-vb-vzcuf-003, 007, 011, 015, 019, 023, 027, 031, 035)
**Evidence:** Same pattern as attempt 1 finding LF4. Flux file PS-001 defines `admit_bytes()` function in `verification/flux/` with `#[flux_rs::sig]` but no `#[extern_spec]` wiring to production types. The Flux annotations are on standalone functions that are not called from production code.

---

### LOW FINDING 1: Kani PS-001 uses `wrapping_add` for assertion comparison
**Severity:** LOW
**Affected:** POB-vb-vzcuf-002 (Kani PS-001)
**Evidence:** `check_admission_boundary()` at line 56: `assert_eq!(total, current + candidate)` uses `+` (which wraps in release mode if overflow would occur, but Kani checks for overflow). The `wrapping_add` reference from attempt 1 appears to be in the fuzz target (line 43: `total, current.wrapping_add(candidate)`).

---

### LOW FINDING 2: Kani harnesses reference `--harness <harness_name> -p vb_storage` but harnesses are in standalone files
**Severity:** LOW
**Affected:** All 9 Kani obligations
**Evidence:** The Kani files are in `verification/kani/vb-vzcuf-PS-*.rs` (standalone top-level files), not inside `crates/vb_storage/src/`. The command `cargo kani --harness ... -p vb_storage` would not find them unless they are compiled into the `vb_storage` crate, which requires module path wiring that is not confirmed.

---

## Summary of Changes vs. Attempt 1

| Area | Attempt 1 | Attempt 2 | Assessment |
|------|-----------|-----------|------------|
| Verus smoke | "Not installed" | All 9 verified (raw evidence captured) | Improvement |
| Kani production binding | Copied models only | Some harnesses call encode_record, JournalError | Partial improvement |
| Proptest production binding | Copied models | Exercises actual JournalWriteBatch API | Significant improvement |
| Fuzz production binding | Copied models | Calls encode_record from production code | Improvement |
| GOD RULE 2 | Violated | Still violated ("PRODUCTION BINDING" comments ≠ binding) | No resolution |
| Verus PS-003 tautology | Violated | Same local ErrorVariant pattern | No resolution |
| Self-approved TBPs | Yes | All 9 still self-approved by proof-writer | No resolution |
| C9 obligation gap | Yes | Still unaddressed | No resolution |
| Kani cover!() | Missing | Still missing | No resolution |
| Flux production binding | Standalone only | Still standalone | No resolution |

## Overall Assessment

Attempt 2 makes real but insufficient progress. The Verus files DO verify (confirmed with raw command evidence), the proptest harnesses exercise actual production types, and some Kani harnesses import and test production code. These are genuine improvements over attempt 1.

However, the three core lethal findings from attempt 1 remain fundamentally unresolved:

1. **GOD RULE 2** is still violated. Adding "PRODUCTION BINDING:" comments to standalone Verus spec functions is not mathematical binding to production `exec fn`. The production code still has zero `requires`/`ensures` annotations and does not contain `staged_bytes`, `byte_limit`, or `AccumulatedBytesExceeded`.

2. **Trusted base entries** are still self-approved by the proof-writer. TBP-006 and TBP-007 are circular — they accept as trusted that code which doesn't exist will be written correctly.

3. **Verus PS-003 and PS-008** are tautological proofs on locally-defined types that prove nothing about production behavior.

The proof-writer's central claim — that "PRODUCTION BINDING" references resolve the GOD RULE 2 violation — is incorrect. Mathematical binding means `requires`/`ensures` on `exec fn` in the production crate, verified by Verus. Documentation comments do not satisfy this requirement.

## Status

STATUS: REJECTED

## Required Remediation (ordered by priority)

1. **Resolve GOD RULE 2:** Either add `requires`/`ensures` annotations to production `exec fn` in `crates/vb_storage/` and verify with Verus, OR file a structural blocker document (`blockers/vb-vzcuf-god-rule-2-blocker.md`) explaining why production binding is infeasible at this verification stage, with a concrete plan for when/how it will happen and compensating evidence (e.g., proptest results as behavioral evidence).

2. **Implement missing production fields:** Add `staged_bytes: u64` and `byte_limit: u64` to `JournalWriteBatch`, add `AccumulatedBytesExceeded` to `JournalError`, or reduce scope of verification claims.

3. **Remove tautological proofs:** Either delete PS-003 ErrorVariant proofs and PS-008 Guard precedence proofs (they prove nothing about production), or replace them with proofs that import the actual production `JournalError` enum and `append_event` control flow.

4. **Submit TBPs for independent review:** Reset all 9 TBP entries to `status: "pending"`. TBP-006 and TBP-007 cannot be accepted until the production code exists.

5. **Generalize Verus lemmas:** PS-001 and PS-004 lemmas must prove properties for all inputs (use `forall` or parameterized arguments), not single concrete values.

6. **Add Kani cover!() statements** to at least the key admission harnesses for non-vacuity evidence.

7. **Add dedicated C9 proof obligation** or file explicit waiver with compensating evidence (e.g., proptest verifies staged_bytes accessor behavior).

8. **Wire Flux annotations** to production types via `#[extern_spec]` or file blocker document.
