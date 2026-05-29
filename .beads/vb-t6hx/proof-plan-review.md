# Proof Plan Review — vb-t6hx (Reduced Scope Replan Acceptance)

reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: proof-plan-reviewer-vb-t6hx-state4-replan-002
planner_invocation_id: proof-planner-vb-t6hx-state4-replan-001
review_state: State 4 proof-plan-reviewer (replan acceptance)
bead: vb-t6hx
workspace: /home/lewis/isolated/femdation-velvet-ballistics/vb-t6hx
source_checkout: /home/lewis/src/velvet-ballistics

## Reviewed Artifacts

| Artifact | SHA-256 |
|---|---|
| `proof-strategy.md` | `948b8c3048bbbb88243ddcc19290ae9e8cbb62dbaeb23a2e8168aae30053ac30` |
| `verifier-lane-decisions.jsonl` | `c5eadf1b5e1ca6142d9313f8c72841f3bf48a0b775d577f5480a96787be41cb6` |
| `proof-obligations.planned.jsonl` | `12bb9ad62bd6444727c82a2b160a0c3eeb657162173a2401e21352f1a51833ea` |
| `trusted-base-plan.md` | `687c8efef378160c5622101b696110fb212f372f456071b5d5179037aa9bc47f` |
| `proof-coverage-matrix.md` | `207e44f0c24672ff964bac17322597f08393f43327d1d503422c50da18990e89` |
| `proof-to-implementation-input.md` | `e11bfda92a2ef3f1e922e8f201b9826f5112fd475d5b2a3561ba0fd5f1a3583a` |
| `verifier-lane-matrix.md` | `2c186cb10fee225f850ce6d2793ad41434ac0e8023a50a9a58769aa3a53fb1c9` |
| `contract.md` | `dc1c4e022aa1e65dd67c80d29e4ebbcfc6ee9a1cf3a57d597b677acde69adb3e` |
| `proof-seeds.jsonl` | `988c0b8496fe22b6a40bbe96a3e2b99986c5fe0598493c0596ba23e3efb954d9` |
| `traceability-matrix.jsonl` | `6349dfcb755c0bd3bf27270a5a4f0baf76da9a14c724f72d8a1c313c89498c21` |
| `delivery-scope.jsonl` | `4ebbe8803357fd08bafb2c66515f0cac30467ea919a49c2fa36c634ee12661e6` |
| `waiver-candidates.jsonl` | `fce98203728ae82e91aa6252867e681340245c468ecc1ea8f255e6054b633301` |
| `proof-plan-review.md` (scope-reduction APPROVED) | `8ae178fab6d5b268c4f0f83ebfc766330dff5a6205f12750b1fac2604f5fc22e` |
| `proof-plan-repair-guide.md` | `271fd1edaa5e62063af0a1934a29e340b8d61871fc11c0e9bb7139b80d03ef9b` |
| `agent-invocation-ledger.jsonl` | read, entries 19-20 |

## Executive Summary

This review accepts the reduced-scope replan produced by `proof-planner-vb-t6hx-state4-replan-001`. The replan faithfully implements the scope reduction approved by `proof-plan-reviewer-vb-t6hx-state4-002-scope-reduction`: 18 proof obligations across Kani (6), proptest (6), and cargo-fuzz (6), with Verus/Flux/TLA+/Loom/Miri excluded as not applicable for this CLI test-first bead. All 56 verifier-lane decisions across 7 proof seeds and 8 verifier lanes are correctly classified and consistently linked to their obligations.

### Provenance Check

- Planner invocation: `proof-planner-vb-t6hx-state4-replan-001` (ledger sequence 20, parent: `proof-plan-reviewer-vb-t6hx-state4-002-scope-reduction`)
- Reviewer invocation: `proof-plan-reviewer-vb-t6hx-state4-replan-002` (this review)
- Invocations are independent. No self-stamped reviewer fields exist in planner artifacts.

## Lane Decision Review

### Coverage Summary

| Verifier | required | not_applicable | Total |
|---|---|---|---|
| kani | 6 | 1 (seed 7: doctor-boundary) | 7 |
| verus | 0 | 7 | 7 |
| flux-rs | 0 | 7 | 7 |
| tla-plus | 0 | 7 | 7 |
| loom | 0 | 7 | 7 |
| miri | 0 | 7 | 7 |
| proptest | 6 | 1 (seed 7: doctor-boundary) | 7 |
| cargo-fuzz | 5 | 2 (seeds 1, 7) | 7 |
| **Total** | **17** | **39** | **56** |

Note: `cargo-fuzz` has 2 obligations for seed `decode-order` (R09 envelope bytes, R10 CLI doctor decode), hence 5 required decisions produce 6 obligations.

### Exclusion Justification Quality

All 39 `not_applicable` lane decisions cite concrete evidence:

| Excluded verifier | All seeds | Justification pattern |
|---|---|---|
| Verus | 1-7 | CLI test-first bead wraps existing `vb_storage` APIs; no new Rust-local invariants justify deductive proof cost. |
| Flux | 1-7 | CLI diagnostic output is cold formatting path; refinement types add maintenance burden with no safety gain. |
| TLA+ | 1-7 | CLI workflow is single-invocation linear sequence (parse → open → scan/get → print); no retries, leases, cancellation, distributed state, or temporal interleaving. |
| Loom | 1-7 | CLI opens handle, reads, closes; no concurrent interleaving inside the CLI subcommand. Storage-level concurrency is the storage crate's responsibility. |
| Miri | 1-7 | Existing `crates/vb_storage/src/codec_miri_tests.rs` already covers storage-level malformed decode safety. CLI glue introduces no new unsafe/FFI/provenance risk. |

Each `not_applicable` row includes `non_applicability_evidence_refs` pointing to the scope-reduction review (lines 77-81) and supporting artifacts. `limitation_kind` is consistently `risk_absent`. This meets the reviewer rubric requirement for non-weak non-applicability.

### Required Lane Quality

| Verifier | Decisions | Obligations | Quality notes |
|---|---|---|---|
| kani | 6 | R01, R04, R07, R11, R14, R17 | Each harness requires `kani::Arbitrary` or `kani::any()` generators per GOD RULES; explicit bounds documented (max_rows=16, max_input_bytes=64, max_record_bytes=256, max_value_bytes=256, max_rows=8/max_value_bytes=128, max_generated_commands=8/keyspaces=9). No hardcoded dummy shapes permitted. |
| proptest | 6 | R02, R05, R08, R12, R15, R18 | All target `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs` as primary evidence channel; each has unique test target name. proptest_cases configured by test harness. |
| cargo-fuzz | 5 | R03, R06, R09, R10, R13, R16 | All require 60-second minimum smoke (`-max_total_time=60`); artifacts routed to `.beads/vb-t6hx/fuzz-artifacts/`. Each targets distinct hostile input surfaces: scan args, get args, envelope decode, CLI doctor decode, bounded preview, projection skip-decode. |

The `not_applicable` kani decision for seed 7 (doctor-boundary) correctly notes that "Kani explores bounded execution states; this seed concerns crate/module placement and dependency drift, not a bounded runtime transition." Source/dependency inspection is the correct evidence channel for boundary drift.

## Obligation Schema Review

All 18 obligations in `proof-obligations.planned.jsonl` use schema `proof-obligation/v1` with no legacy alias fields. Each obligation includes:
- Unique ID (`PO-vb-t6hx-R01` through `PO-vb-t6hx-R18`)
- `requirement_id` and `contract_clause` matching contract.md
- `domain_claim` with specific property assertion
- `verifier` matching lane decisions
- `artifact` and `target` specifying exact file and harness/test name
- `command` with exact CLI invocation and `workdir`
- `expected_evidence` with concrete pass criteria
- `assumptions` documented
- `model_bounds` specified (Kani: max values; fuzz: max_total_time_seconds)
- `tool_metadata` with tool and lane
- `trusted_base_refs` cross-referencing trusted-base-plan.md
- `required: true`, `behavior_affecting: true`
- `mode: verify-proof`
- `owner_state: 4`, `rerun_from: 4`, `status: planned`

No schema drift detected. No `PO-vb-t6hx-037` legacy-alias IDs survive from the old plan.

## Trusted Base and Waiver Review

`trusted-base-plan.md` contains 18 entries (`TBP-vb-t6hx-R01` through `TBP-vb-t6hx-R18`), one per obligation. Each documents the trusted surface/bound, reason, and closure expectation. Key observations:

- Kani entries explicitly cover bound declarations (max_rows/max_limit=16, max_input_bytes=64, max_record_bytes=256/unwind=8, max_value_bytes=256/max_preview=64, max_rows=8/max_value_bytes=128, max_generated_commands=8/keyspaces=9)
- Proptest entries note output parser counting and assertion reliability
- Fuzz entries note 60-second smoke bound is planning minimum, not exhaustive
- Entry R07 notes that existing `kani_postcard_envelope_wire.rs` may need extension; CRC/BLAKE3 treated as external equality oracles
- No behavior-affecting proof obligation is waived

**Waiver candidate WC-vb-t6hx-001**: Non-behavior-affecting candidate for deep dependency/supply-chain attestation. Valid only if no Cargo.toml/Cargo.lock/dependency changes. Invalidated by any manifest change or boundary drift. `review_status: pending`. This is a non-behavior waiver; it does not waive any proof obligation, `moon ci` gate, or runtime/core boundary behavior.

## Bridge Planning Review

`proof-to-implementation-input.md` names concrete source targets for all 7 proof seeds:
- CLI parser/dispatch: `crates/vb_cli/src/args.rs`, `crates/vb_cli/src/app_impl.rs`
- Storage read boundary: `crates/vb_storage/src/journal/core.rs`
- Codec spine: `crates/vb_storage/src/codec/mod.rs`, `crates/vb_storage/src/error/mod.rs`
- Behavior tests: `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs`
- Existing proof artifacts: `crates/vb_storage/src/kani_postcard_envelope_wire.rs`, `crates/vb_storage/src/codec_miri_tests.rs`
- Fuzz targets: `fuzz/fuzz_targets/`

Required bridge checks for State 7 are specified: Kani arbitrary generators, fuzz production-function targeting, Postcard decoder as external dependency, behavior test independence, and waiver invalidation conditions. This is sufficient for the reduced scope.

## Coverage Verification Against Repair Guide

Checking the replan against `proof-plan-repair-guide.md`:

| Repair step | Status |
|---|---|
| Step 1: Rewrite proof-strategy.md | ✅ Done. Risk classification matches CLI-only risks. Strategy documents proptest/Kani/fuzz/behavior tests. |
| Step 2: Rewrite verifier-lane-decisions.jsonl | ✅ Done. 56 decisions: 6 kani required, 6 proptest required, 5 cargo-fuzz required, all non-applicable rows have evidence refs. |
| Step 3: Rewrite proof-obligations.planned.jsonl | ✅ Done. 18 obligations with consistent contract clauses, exact commands, model_bounds. |
| Step 4: Rewrite proof-to-implementation-input.md | ✅ Done. Source targets updated, claim mapping covers all seeds. |
| Step 5: Rewrite proof-coverage-matrix.md | ✅ Done. 4 active verifier layers. |
| Step 6: Update trusted-base-plan.md | ✅ Done. 18 TBP entries, no legacy entries from rejected Verus/Flux/TLA+/Loom/Miri obligations. |

The replan precisely matches the repair guide specifications. No contract clause mismatches (the old `E_LANE_OBLIGATION_MISMATCH` finding does not reproduce in the reduced-scope plan — the 7 proof seeds × reduced verifier profile produce a tractably consistent matrix).

## Non-Applicability of Verus/Flux/TLA+/Loom/Miri

Each excluded verifier lane decision cites `proof-plan-review.md` lines 77-81 and supporting artifacts (`contract.md:44-47`, `delivery-scope.jsonl:8-9`, `workflow-model.md`, `boundary-map.md`, `type-contracts.md`, `hazard-analysis.md`). The justifications are:

1. **Verus**: Wrapping existing storage APIs does not introduce new Rust-local invariants; deductive proof cost is unjustified for CLI glue.
2. **Flux**: CLI diagnostic output is a cold formatting path; refinement types add maintenance burden without safety gain.
3. **TLA+**: CLI workflow is a deterministic single-invocation sequence with no temporal/distributed properties.
4. **Loom**: CLI opens-a-handle-reads-closes with no internal concurrent interleaving; storage-level concurrency is the storage crate's concern.
5. **Miri**: Storage-level `codec_miri_tests.rs` already covers malformed decode safety; CLI glue introduces no new unsafe/FFI/provenance surface.

These justifications are concrete, evidence-backed, and correctly map the bead's risk profile (CLI test-first, no new storage invariants, no concurrency, no unsafe) to `not_applicable` decisions. The Go-skill verification-lane policy defaults Rust-local behavior to Verus/Kani/Flux/proptest but allows conditional exclusion where the risk profile does not demand it. The justifications provided are specific enough to survive adversarial review.

## Verifier Lane Review Rows

Wrote 56 `verifier-lane-review/v1` rows in `verifier-lane-review.jsonl` — one per planner `verifier-lane-decision/v1` row (VLD-vb-t6hx-R01 through VLD-vb-t6hx-R56). All 56 rows have `reviewer_disposition: accepted`. Planner invocation: `proof-planner-vb-t6hx-state4-replan-001`. Reviewer invocation: `proof-plan-reviewer-vb-t6hx-state4-replan-002`.

## Findings

No findings. The replan correctly implements the approved scope reduction. All 56 lane decisions are internally consistent, all 18 obligations are schema-compliant with exact commands and explicit model bounds, all trusted-base entries cover declared bounds, and no behavior-affecting proof obligation is waived.

## Final Disposition

The reduced-scope replan (planner invocation `proof-planner-vb-t6hx-state4-replan-001`) is **APPROVED**. The plan is precise enough for State 5 proof-writer and State 7 proof-to-implementation: it names exact artifact files, target harness/test names, exact verifier commands with workdirs, explicit Kani bounds, fuzz smoke minimums, and trusted-base entries for every obligation.

Next actions after this approval:
1. State 4 validator must pass on these artifacts.
2. State 5 proof-writer must produce proptest properties, Kani harnesses (using `kani::Arbitrary`/`kani::any()` per GOD RULES), and fuzz targets (60-second minimum smoke) matching the 18 planned obligations.
3. State 6 proof-reviewer must verify the written artifacts against these planned obligations.
4. State 7 proof-to-implementation must bridge all approved proof claims to Rust source refs.
5. The non-behavior waiver WC-vb-t6hx-001 remains pending; it is invalidated by any dependency manifest change.

STATUS: APPROVED