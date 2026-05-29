# Proof Plan Repair Guide — vb-t6hx Scope Reduction

reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: proof-plan-reviewer-vb-t6hx-state4-002-scope-reduction
planner_invocation_id: proof-planner-vb-t6hx-state4-001
repair_target_state: 4
rerun_from: 4 (proof-planner replan, then proof-plan-reviewer acceptance)

## Root Cause

The prior plan (`proof-planner-vb-t6hx-state4-001`, approved by `proof-plan-reviewer-vb-t6hx-state4-001`) treated a CLI test-first bead as if it were a new storage engine or distributed protocol. The plan mandated Verus, Flux, TLA+, Loom, and Miri for CLI glue that wraps existing `vb_storage` APIs. This over-scoping caused:

- 56 lane decisions across 8 verifiers (should be ~28 across 4 active verifiers)
- 37 proof obligations with contract-clause mismatches
- 7 proof-writer repair attempts, all failing
- 5 proof-reviewer rejections at State 6
- Wasted verification effort on tools inappropriate for CLI test-first work

## Reduced Scope

The approved reduced scope is:

| Verifier | Obligations (est.) | Purpose |
|---|---|---|
| **proptest** | 6 | CLI argument parsing, scan limits, hex validation, envelope error classes, preview bounds, skip-decode projection |
| **Kani** | 6 | Bounded scan enumeration, hex parser bounded input, decode order over bounded envelopes, preview truncation, skip-decode bounded state, read-only command selection |
| **cargo-fuzz** | 5 | Hostile argv for scan/get, envelope decode bytes, preview adversarial inputs, projection skip-decode, bounded preview |
| **Behavior tests** | Primary | All 10 acceptance-behavior contract seeds from contract.md |

### Explicitly Excluded

| Verifier | Reason |
|---|---|
| **Verus** | CLI glue wraps existing storage APIs; no new Rust-local invariants justify Verus |
| **Flux** | CLI diagnostic output is cold path; refinement types add maintenance burden |
| **TLA+** | Single-invocation linear sequence; no temporal/distributed properties |
| **Loom** | CLI opens handle, reads, closes; no concurrent interleaving inside command |
| **Miri** | Existing storage-level `codec_miri_tests.rs` already covers malformed decode safety |

## Exact Repair Steps

### Step 1: Proof-Planner Rewrites proof-strategy.md

Delete the existing file. Write a new strategy that:
- Risk classification: Bounded state, Untrusted input, Performance/resource only. Remove Temporal/state-machine, Rust-local invariant, Refinement/type-state, Concurrency/interleaving rows.
- Strategy section: proptest for CLI parser/codec/preview properties; Kani for bounded state enumeration; fuzz for hostile CLI argv and decode bytes; behavior tests as primary evidence.
- Required obligation groups: ~4 groups (parser boundary, scan bounds, decode order, preview bounds), ~18 obligations total.

### Step 2: Proof-Planner Rewrites verifier-lane-decisions.jsonl

Delete the existing file. Write new lane decisions:

For each of 7 proof seeds, produce decisions for 8 verifiers (default profile + conditionals):
- kani: required for seeds 1-6 (6 rows)
- verus: not_applicable for all seeds (7 rows) — CLI glue wraps existing APIs
- flux-rs: not_applicable for all seeds (7 rows) — cold diagnostic path
- proptest: required for seeds 1-6; not_applicable for seed 7 (6+1 rows)
- tla-plus: not_applicable for all seeds (7 rows) — single-invocation workflow
- loom: not_applicable for all seeds (7 rows) — no CLI-internal concurrency
- miri: not_applicable for all seeds (7 rows) — no new unsafe; storage-level Miri exists
- cargo-fuzz: required for seeds 2-6 (parser/codec/preview hostile surfaces); not_applicable for seeds 1,7 (2+5 rows)

Total: 56 lane decisions (same count, different applicability)

Use consistent contract clauses. Copy from contract.md exactly.

### Step 3: Proof-Planner Rewrites proof-obligations.planned.jsonl

Delete the existing file. Write ~18 new obligations:

**Bounded Scan Properties (Kani #1):**
- ID: PO-vb-t6hx-R01
- Seed: vb-t6hx-seed-scan-bounded
- Verifier: kani
- Command: `cargo kani -p vb_cli --harness kani_harness_scan_limit_rows_never_exceed_limit`
- Bounds: max_rows=16, max_limit=16

**Bounded Scan Properties (proptest #1):**
- ID: PO-vb-t6hx-R02
- Seed: vb-t6hx-seed-scan-bounded
- Verifier: proptest
- Command: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- proptest_doctor_scan_rows_never_exceed_limit`

**Hostile Scan Args (fuzz #1):**
- ID: PO-vb-t6hx-R03
- Seed: vb-t6hx-seed-scan-bounded
- Verifier: cargo-fuzz
- Command: `cargo +nightly fuzz run vb_t6hx_doctor_scan_args -- -max_total_time=60`

**Hex Key Parser (Kani #2):**
- ID: PO-vb-t6hx-R04
- Seed: vb-t6hx-seed-hex-key-parser
- Verifier: kani
- Command: `cargo kani -p vb_cli --harness kani_harness_hex_key_rejects_invalid_before_open`

**Hex Key Parser (proptest #2):**
- ID: PO-vb-t6hx-R05
- Seed: vb-t6hx-seed-hex-key-parser
- Verifier: proptest

**Hex Key Hostile Args (fuzz #2):**
- ID: PO-vb-t6hx-R06
- Seed: vb-t6hx-seed-hex-key-parser
- Verifier: cargo-fuzz

**Envelope Decode Order (Kani #3):**
- ID: PO-vb-t6hx-R07
- Seed: vb-t6hx-seed-decode-order
- Verifier: kani
- Command: `cargo kani -p vb_storage --harness kani_harness_storage_decode_order`
- Note: may extend or reference existing harness; no hardcoded dummy shapes

**Envelope Decode Order (proptest #3):**
- ID: PO-vb-t6hx-R08
- Seed: vb-t6hx-seed-decode-order
- Verifier: proptest

**Envelope Decode Fuzz (fuzz #3):**
- ID: PO-vb-t6hx-R09
- Seed: vb-t6hx-seed-decode-order
- Verifier: cargo-fuzz

**Envelope Doctor Decode Fuzz (fuzz #4):**
- ID: PO-vb-t6hx-R10
- Seed: vb-t6hx-seed-decode-order
- Verifier: cargo-fuzz
- Command: `cargo +nightly fuzz run vb_t6hx_doctor_decode_cli -- -max_total_time=60`

**Preview Bounded (Kani #4):**
- ID: PO-vb-t6hx-R11
- Seed: vb-t6hx-seed-preview-bounded
- Verifier: kani
- Command: `cargo kani -p vb_cli --harness kani_harness_bounded_preview_never_exceeds_limit`

**Preview Bounded (proptest #4):**
- ID: PO-vb-t6hx-R12
- Seed: vb-t6hx-seed-preview-bounded
- Verifier: proptest

**Preview Hostile Input (fuzz #5):**
- ID: PO-vb-t6hx-R13
- Seed: vb-t6hx-seed-preview-bounded
- Verifier: cargo-fuzz

**Skip-Decode Projection (Kani #5):**
- ID: PO-vb-t6hx-R14
- Seed: vb-t6hx-seed-skip-decode-projection
- Verifier: kani

**Skip-Decode Projection (proptest #5):**
- ID: PO-vb-t6hx-R15
- Seed: vb-t6hx-seed-skip-decode-projection
- Verifier: proptest

**Skip-Decode Hostile Value (fuzz #6):**
- ID: PO-vb-t6hx-R16
- Seed: vb-t6hx-seed-skip-decode-projection
- Verifier: cargo-fuzz

**Read-Only No Mutation (Kani #6):**
- ID: PO-vb-t6hx-R17
- Seed: vb-t6hx-seed-readonly-no-mutation
- Verifier: kani
- Command: `cargo kani -p vb_cli --harness kani_harness_doctor_storage_readonly_no_mutation`

**Read-Only Inventory (proptest #6):**
- ID: PO-vb-t6hx-R18
- Seed: vb-t6hx-seed-readonly-no-mutation
- Verifier: proptest

All obligations must use consistent contract clauses from contract.md.

### Step 4: Proof-Planner Rewrites proof-to-implementation-input.md

Update source targets and claim mapping to reflect reduced scope. Remove Verus/Flux/TLA+/Loom/Miri bridge requirements. Add behavior test file as primary evidence channel.

### Step 5: Proof-Planner Rewrites proof-coverage-matrix.md and verifier-lane-matrix.md

Reduce to 4 active verifiers (proptest, Kani, fuzz, behavior test).

### Step 6: Proof-Planner Updates trusted-base-plan.md

Remove trusted-base entries for rejected Verus/Flux/TLA+/Loom/Miri obligations. Keep Kani bound entries.

### Step 7: Proof-Plan-Reviewer Validates Replan

Run State 4 validator after replan. If PASS, dispatch State 5 proof-writer with reduced plan.

### Step 8: Archive Old Artifacts

Move old over-scoped artifacts to `.beads/vb-t6hx/archive/scope-reduction-20260527T000000Z/`:
- `proof-plan-review.md` (hash `88472cf...`)
- `verifier-lane-review.jsonl` (hash `39eb836...`)
- `proof-strategy.md` (hash `76a696a...`)
- `verifier-lane-decisions.jsonl` (hash `ae496e9...`)
- `proof-obligations.planned.jsonl` (hash `18573a1...`)
- `trusted-base-plan.md` (hash `d6023fc...`)
- `proof-to-implementation-input.md` (hash `cb8a275...`)
- `proof-coverage-matrix.md` (hash `3755b7f...`)
- `verifier-lane-matrix.md` (hash `33e3914...`)
- `proof-plan-findings.jsonl` (hash `e3b0c44...` — empty)

### Step 9: Archive State 6 Rejected Artifacts

Move to `archive/scope-reduction-20260527T000000Z/state6-rejected/`:
- `proof-review.md` (hash `e4d3bd6...`)
- `proof-findings.jsonl` (hash `5ca1a88...`)

## Expected Outcome

After replan: ~18 proof obligations across proptest + Kani + fuzz, plus behavior test coverage for all 10 acceptance-behavior contract seeds. The reduced plan is achievable: proptest/Kani/fuzz/nextest are all tools that the fleet runs regularly, and the obligations target CLI-appropriate properties rather than storage-engine invariants.

## Minimum State to Rerun

State 4 (proof-planner replan → proof-plan-reviewer acceptance → State 4 validation PASS).

Do not rerun State 5 or State 6 with the old over-scoped plan. After State 4 passes with reduced scope, dispatch fresh State 5 proof-writer.
