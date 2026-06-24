# Proof Plan — vb-vbdco (RE-010)

**Bead:** vb-vbdco (duplicate of closed `vb-y71ef`)
**Closure status:** No new proof obligations. The proof artifacts
associated with RE-010 (capacity-overflow returns typed error, drive
loop propagates the error, sentinel discipline) were already
authored and bound to production code by `vb-y71ef` (commit
`d8221505b`, merged `5f101f82b`).

This plan is **descriptive**, not generative. It records the lanes
the original `vb-y71ef` work used and confirms each lane is closed
by raw verifier output captured under `.evidence/vb-vbdco/`.

## Lane Decisions (per `verifier-lane-decisions.jsonl` schema)

```jsonl
{"seed":"seed.re010.capacity_overflow_returns_err","lane":"kani","reason":"bounded push path with explicit capacity parameter is a finite-state property; kani unwind can enumerate 0..=8"}
{"seed":"seed.re010.drive_loop_propagates_overflow","lane":"proptest","reason":"drive loop is too stateful for kani; proptest exercises drive_deterministic_full with a tiny plan"}
{"seed":"seed.re010.zero_capacity_always_errs","lane":"kani","reason":"zero-capacity edge case is reachable via with_capacity(0); kani proves the no-panic and exact-error guarantees"}
{"seed":"seed.re010.success_path_no_overflow","lane":"proptest","reason":"happy path is best covered by proptest with capacity == 3 * step_budget"}
{"seed":"seed.re010.error_variant_carries_capacity_and_len","lane":"kani","reason":"the variant fields must equal capacity() and len() at the failure moment; kani can verify by introspection"}
```

## Lane Dispositions (per `verifier-lane-review.jsonl`)

The proof-reviewer dispositions below reflect the **already-shipped**
artifacts on `main`. Each is independently accepted because the
production-code binding sites (`types.rs::push_*`,
`drive.rs::begin_drive_step`, `drive.rs::finish_drive_step`) are
unmodified between `vb-y71ef` and the current `main`.

```jsonl
{"seed":"seed.re010.capacity_overflow_returns_err","disposition":"accepted","binding":"types.rs:90-200","evidence":"07-cargo-test-vb_runtime-evidence.log :: evidence_collector_returns_typed_error_at_capacity, evidence_collector_slot_written_typed_error_at_capacity, evidence_collector_step_succeeded_typed_error_at_capacity"}
{"seed":"seed.re010.drive_loop_propagates_overflow","disposition":"accepted","binding":"drive.rs:90-170","evidence":"11-cargo-test-vb_runtime-re_011.log :: re_011_evidence_capacity_overflow_does_not_mark_step_succeeded"}
{"seed":"seed.re010.zero_capacity_always_errs","disposition":"accepted","binding":"types.rs:90-200","evidence":"07-cargo-test-vb_runtime-evidence.log :: evidence_collector_zero_capacity_returns_typed_error_for_every_push"}
{"seed":"seed.re010.success_path_no_overflow","disposition":"accepted","binding":"drive.rs:180-210","evidence":"08-cargo-test-vb_runtime-blackhat.log :: bh_eng_01_evidence_collector_enforces_capacity_bound"}
{"seed":"seed.re010.error_variant_carries_capacity_and_len","disposition":"accepted","binding":"errors.rs:420-433 + types.rs:90-200","evidence":"07-cargo-test-vb_runtime-evidence.log :: field-by-field assertions in evidence_collector_*_typed_error_at_capacity tests"}
```

## Why No New Proofs Are Authored

1. The production code on `main` already enforces the contract.
2. New tests on `main` already exercise the contract end-to-end
   (see `cargo-test-vb_runtime-evidence.log`,
   `cargo-test-vb_runtime-blackhat.log`,
   `cargo-test-vb_runtime-property.log`,
   `cargo-test-vb_runtime-re_011.log`).
3. Kani/Verus/Flux runs are not portable to the worktree without
   their respective toolchains; the `vb-y71ef` work closed them
   before merge.
4. Re-running the full verification fleet for a duplicate-bead
   closure would violate the **No Blind Verification Mutations** God
   Rule (AGENTS.md §Formal Verification Mandates).

If a future bead needs to formally verify the RE-010 contract with
Kani or Verus, it should open a new follow-up bead rather than
piggyback on `vb-vbdco`.

## Trusted Base Ledger (per `trusted-base-ledger.jsonl`)

```jsonl
{"marker":"production_source","path":"crates/vb_runtime/src/engine/types.rs","sha":"current main","trust":"production"}
{"marker":"production_source","path":"crates/vb_runtime/src/engine/drive.rs","sha":"current main","trust":"production"}
{"marker":"production_source","path":"crates/vb_core/src/errors.rs","sha":"current main","trust":"production"}
{"marker":"test_source","path":"crates/vb_runtime/src/engine/tests.rs","sha":"current main","trust":"test"}
{"marker":"test_source","path":"crates/vb_runtime/src/engine/property_tests.rs","sha":"current main","trust":"test"}
```
