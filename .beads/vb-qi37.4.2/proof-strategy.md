# Proof Strategy

Bead: `vb-qi37.4.2`
State: 4 proof planning repair attempt 4
Scope: isolated workspace `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2`

## Inputs Read

- Repaired State 3 artifacts after contract status repair: `contract.md`, `verification-layers.md`, `martin-fowler-tests.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `tla-spec.md`, `lean-contract.md`, and `delivery-scope.jsonl`.
- State 6 rejection artifacts: `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, and `contract-verification-review.md`.
- Prior proof evidence used only as context: `proof-evidence.md` and `proof-writer-report.md`.

## Discovery Evidence

- `pwd -P` returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2`.
- `test -s ".beads/vb-qi37.4.2/contract.md" && test -s ".beads/vb-qi37.4.2/traceability-matrix.jsonl" && test -s ".beads/vb-qi37.4.2/delivery-scope.jsonl"` exited 0.
- Scoped risk scan over `delivery-scope.jsonl` paths found admission state/allocation, serialization/deserialization, queue/retry/cancel terms, and `#![forbid(unsafe_code)]` markers. It also found test-only `assert!` in scoped source files, which affects later lint/static review but does not create a proof-planner edit.
- Scoped verifier scan found existing Verus proof functions in `verification/verus/capability_artifact_model.rs` and `verification/verus/accepted_envelope_model.rs`; it found no `kani::`, `loom::`, `proptest!`, `fuzz_target`, or Flux proof hooks in the scoped runtime/storage/CLI source files.
- Discovery blockers: none for the required proof-planner discovery commands.

## Rejection Repair Deltas Reflected

- `VERUS-ENV-006` is now an executable Verus planning row targeting `verification/verus/accepted_envelope_model.rs` with command `verus verification/verus/accepted_envelope_model.rs`.
- TLA+/Verus evidence expectations now reference existing `proof-evidence.md` sections rather than missing `tla-report.md` or `verus-report.md` files.
- `PO-007`, `PO-008`, `PO-009`, `PO-011`, and `PO-012` are planned downstream evidence-policy rows with owner, reason, expiry, limitation, and compensating evidence in `waiver_policy`. They remain `status:"planned"` and do not claim pass or waiver approval evidence at contract/planning time.
- `TEST-STRICT-009` expected evidence now enumerates exact ERR-001 through ERR-008 diagnostic scenarios.
- `PO-010` remains required and executable for later static/lint verification because it has concrete scoped source targets and an exact command.

## Risk Classification

- Temporal admission lifecycle: strict/journaled denial must occur before frame allocation, run insertion, `drive_run`, and `RunAccepted`. Primary lane: TLA+ safety model.
- Gate-count mismatch: runtime canonical gate `15` versus existing storage gate `2` must fail closed until reconciled. Primary lanes: TLA+ and Verus decoded accepted-envelope predicate.
- Capability exactness: capability grants must be exact by name/action and cardinality. Primary lanes: Verus and TLA+.
- Dummy-store bypass: strict/journaled production paths must not admit through `AlwaysPresentArtifactStore` or existence-only APIs. Primary lanes: TLA+ abstraction plus later static scan and integration tests.
- Parser/codec hostile input: raw `WorkflowParts`, YAML/JSON bytes, malformed postcard, unknown schema, missing fields, and random bytes must deny without allocation and preserve diagnostics. Current executable proof lane is decoded-value Verus; byte-level fuzz remains a downstream evidence-policy row until the exact target exists or a later WAIVED/DEFERRED evidence record is approved.
- Digest mismatch: requested digest, persisted record digest, and envelope digest disagreement is a hard denial. Kani remains a downstream evidence-policy row until a harness exists or a later WAIVED/DEFERRED evidence record is approved; integration/domain scenarios remain required.
- Diagnostics: ERR-001 through ERR-008 must preserve category, rejected/requested digest where present, and semantic cause. Mutation remains a downstream evidence-policy row until diagnostic tests exist or a later WAIVED/DEFERRED evidence record is approved.
- Unsafe/UB: no unsafe/FFI/raw-pointer scope trigger was found; Miri is not applicable as a bead-specific lane.
- Concurrency: no thread/atomic/lock/channel/async interleaving scope trigger was found; Loom is not applicable.
- Dependency/supply chain: no dependency manifest or policy file is in delivery scope; cargo audit/deny/geiger is not applicable for this bead.

## Verifier Lane Plan

- TLA+ required: run existing `verification/tla/CapabilityLifecycle.tla` focused configs for no allocation on denial, gate mismatch, exact/excess capability grants, and legacy/dummy bypass.
- Verus required: run existing `verification/verus/capability_artifact_model.rs` and `verification/verus/accepted_envelope_model.rs` for decoded capability and accepted-envelope predicates.
- Runtime/tests required later: run strict admission scenarios covering ERR-001 through ERR-008 and no-allocation behavior.
- Static scan/lint required later: audit strict/journaled production paths for dummy-store and YAML/JSON runtime parse bypasses, then run `moon run :lint-src`.
- Kani/fuzz/proptest/mutation/CI: planned downstream evidence-policy rows where exact targets or state permission are absent; each policy has an expiry requiring raw evidence or downstream WAIVED/DEFERRED evidence before anyone claims that lane.
- Lean/Aeneas/Hax, TLA+ liveness, Loom, Miri, Flux, dependency audit/geiger: not applicable with explicit rationale in the obligation ledger.

## Handoff Rules

- Do not treat planned rows as pass evidence. Later execution states must run commands and record raw outputs.
- Do not create proof/model/harness/test/source/dependency/config files from State 4.
- Replace downstream evidence-policy rows only when an exact artifact and executable command exist, or when formal-verifier/landing records a WAIVED/DEFERRED evidence decision with owner, reason, expiry, limitation, and compensating evidence.
- Preserve decoded-value versus byte-level boundaries: Verus owns decoded predicates; fuzz/integration own hostile bytes; Kani remains optional only until a bounded digest harness exists.
