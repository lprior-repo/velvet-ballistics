# Proof Strategy — vb-t6hx (Reduced Scope, Scope Reduction Replan)

State 4 proof-planner replan output for bead `vb-t6hx`. This plan reflects the approved reduced scope from proof-plan-reviewer-vb-t6hx-state4-002-scope-reduction: proptest, Kani, cargo-fuzz, and behavior tests only. Verus, Flux, TLA+, Loom, and Miri are excluded as inappropriate for a CLI test-first bead wrapping existing storage APIs. This plan is planning only: it does not claim proof success, write verifier artifacts, write tests, or approve review.

## Inputs Read

- `.beads/vb-t6hx/contract.md`
- `.beads/vb-t6hx/proof-seeds.jsonl`
- `.beads/vb-t6hx/traceability-matrix.jsonl`
- `.beads/vb-t6hx/domain-model.md`
- `.beads/vb-t6hx/type-contracts.md`
- `.beads/vb-t6hx/workflow-model.md`
- `.beads/vb-t6hx/error-taxonomy.md`
- `.beads/vb-t6hx/boundary-map.md`
- `.beads/vb-t6hx/hazard-analysis.md`
- `.beads/vb-t6hx/delivery-scope.jsonl`
- `.beads/vb-t6hx/codebase-map.md`
- `.beads/vb-t6hx/proof-plan-repair-guide.md`
- `.beads/vb-t6hx/proof-plan-review.md` (scope-reduction APPROVED)

State 3 validator status: `PASS`. State 4 scope-reduction review: `APPROVED`.

## Bead Reality Check

Bead vb-t6hx is a **CLI test-first bead** with three components:
1. Behavioral tests in `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs`
2. Minimal CLI implementation for read-only storage scan/get, envelope decode, and bounded preview
3. No new storage engine logic, no new unsafe code, no new concurrency primitives, no new protocol

The CLI wraps existing `vb_storage` APIs and adds a diagnostic shell. A read-only CLI subcommand calling existing storage APIs does not demand Verus, Flux, TLA+, Loom, or Miri.

## Risk Classification

| Risk class | Present | Evidence | Proof response |
|---|---|---|---|
| Bounded state | Yes | scan/preview limits, finite parser classes, bounded record envelope | Kani required where honest bounded state enumeration applies |
| Untrusted input | Yes | argv, hex, numeric limits, malformed envelopes, large values | proptest and cargo-fuzz required for parser/codec/preview hostile surfaces |
| Performance/resource | Yes | unbounded scan/preview hazards | bounded-resource obligations via Kani/proptest/fuzz |
| Release-critical gates | Yes | `moon ci` canonical final gate | included as downstream evidence expectation, not a State 4 proof claim |
| Temporal/state-machine | No | single-invocation linear workflow (parse → open → scan/get → print) | TLA+ excluded: no retries, timeouts, leases, cancellation, or distributed state |
| Rust-local invariant | No | CLI glue wraps existing `vb_storage` APIs; no new storage invariants | Verus excluded: no safety-critical new pure/core invariants justifying Verus cost |
| Refinement/type-state | No | CLI diagnostic output is cold format path | Flux excluded: refinement types add maintenance burden with no safety gain for cold output |
| Concurrency/interleaving | No | CLI opens handle, reads, closes; no concurrent interleaving inside CLI command | Loom excluded: storage-level concurrency belongs to storage crate |
| Unsafe/UB | No | no new unsafe; existing `codec_miri_tests.rs` covers storage-level malformed decode safety | Miri excluded: CLI glue does not introduce new unsafe/FFI/provenance risk |
| Dependency/supply-chain | Not directly changed | delivery scope identifies source/test additions, not dependency edits | non-behavior waiver candidate for deep supply-chain attestation only |

## Strategy

1. **proptest**: Cover CLI argument parsing, scan limits, hex validation, preview bounds, envelope decode error classes, and skip-decode projection with generated input properties. Use upstream `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs` as the primary property test file.
2. **Kani**: Bounded scan enumeration properties, hex parser bounded input, decode order over bounded envelopes, preview truncation, skip-decode bounded state, and read-only command selection. Harnesses must use `kani::Arbitrary` or exhaustive `kani::any()` generators per repository GOD RULES; no hardcoded dummy shapes.
3. **cargo-fuzz**: Hostile argv for scan/get, envelope decode bytes, preview adversarial inputs, projection skip-decode, and bounded preview. Fuzz targets in `fuzz/fuzz_targets/` with 60-second smoke minimum. Handle artifacts under `.beads/vb-t6hx/fuzz-artifacts/`.
4. **Behavior tests (nextest)**: Primary evidence channel. Tests cover all 10 acceptance-behavior contract seeds from `contract.md` including scan limit rows, raw get, missing key, invalid hex, large value preview, no-color, read-only inventory, envelope decode, and projection skip-decode.

## Required Planned Obligation Groups

- scan-bounded: `PO-vb-t6hx-R01` (Kani), `PO-vb-t6hx-R02` (proptest), `PO-vb-t6hx-R03` (fuzz)
- hex-key-parser: `PO-vb-t6hx-R04` (Kani), `PO-vb-t6hx-R05` (proptest), `PO-vb-t6hx-R06` (fuzz)
- decode-order: `PO-vb-t6hx-R07` (Kani), `PO-vb-t6hx-R08` (proptest), `PO-vb-t6hx-R09` (fuzz envelope), `PO-vb-t6hx-R10` (fuzz CLI decode)
- preview-bounded: `PO-vb-t6hx-R11` (Kani), `PO-vb-t6hx-R12` (proptest), `PO-vb-t6hx-R13` (fuzz)
- skip-decode-projection: `PO-vb-t6hx-R14` (Kani), `PO-vb-t6hx-R15` (proptest), `PO-vb-t6hx-R16` (fuzz)
- readonly-no-mutation: `PO-vb-t6hx-R17` (Kani), `PO-vb-t6hx-R18` (proptest)

## Non-Applicable Lane Pattern

Verus, Flux, TLA+, Loom, and Miri are `not_applicable` for every seed in this reduced scope. Each non-applicable row in `verifier-lane-decisions.jsonl` cites concrete justification: CLI test-first bead, no new production invariants, single-invocation linear workflow, no new unsafe/FFI/provenance risk. Non-applicable does not mean unimportant; it means the verifier lane does not match the risk class for this bead.

## Blockers

No State 4 planning blocker. Tool availability is intentionally not asserted here; State 5/12 executors must record actual versions and command evidence.
