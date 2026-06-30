# Proof Strategy: vb-qi37.1

State 4 attempt 5 repair after State 3 schema repair. Planning only; no production code, tests, proof/model/harness/spec, dependency, or config files were edited.

## Inputs Read

- Repaired State 3 artifacts: `contract.md`, `tla-spec.md`, `lean-contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `delivery-scope.jsonl`, `codebase-map.md`.
- State 6 rejection context: `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, `contract-verification-review.md`.
- Prior State 5 proof evidence as invalidated context only: `proof-writer-report.md`, `proof-evidence.md`.

## Discovery Evidence

- `pwd -P` -> `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1`.
- `test -s .beads/vb-qi37.1/contract.md && test -s .beads/vb-qi37.1/traceability-matrix.jsonl && test -s .beads/vb-qi37.1/delivery-scope.jsonl` -> exit 0.
- `rtk grep -n "unsafe|unwrap\\(|expect\\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" crates/vb_storage/src/recovery crates/vb_storage/src/events.rs crates/vb_runtime/src/recovery.rs verification` -> exit 0, 465 matches.
- `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" crates/vb_storage/src/recovery crates/vb_storage/src/events.rs crates/vb_runtime/src/recovery.rs verification` -> exit 0, 292 matches.

## Risk Assignment

- TLA+ owns temporal recovery lifecycle, mixed-run rejection, snapshot-tail ordering, crash-before-ack and crash-after-ack restart semantics, durable fact preservation or fail-closed state, no YAML/JSON/HTTP recovery input, and terminal consistency.
- Verus owns Rust-local unsupported-state rejection, no successful empty live-frame hydration, no fabricated slot/taint state, non-vacuous typed error propagation/refinement through the pure boundary, required workflow-source and compiled-IR digest mismatch checks, and dimension overflow decision logic.
- Integration/fault-injection owns Fjall-backed restart evidence for crash cuts, corrupt journal/snapshot, and snapshot+journal replay.
- Proptest/property-style cargo tests own broad corrupt event/slot/taint input coverage.
- `moon ci` owns source governance/static-scan evidence for no silent discard or forbidden constructs in touched source.

## State 6 Repair Reflected

- Required digest proof scope is now limited to production-linked workflow-source and compiled-IR mismatch detection: `PO-017`, `PO-019`, and `PO-020`.
- Action ABI and policy digest mismatch lanes are explicit optional downstream waivers: `PO-021` and `PO-022`; they are not State 5 proof blockers for vb-qi37.1.
- `PO-016` still requires a non-vacuous typed-error propagation/refinement proof. The prior tautological Verus proof remains invalidated and must be repaired in State 5.
- Optional omitted lanes are mechanically reviewable waiver rows: `PO-033` Kani, `PO-034` Flux, `PO-035` Loom/Miri, and `PO-036` fuzz/theorem/dependency.

## State 3 Schema Repair Reflected

- `PRE-004` now has direct contract-time and proof-planning coverage: `VERUS-PRE-004` in `proof-obligations.jsonl` and `PO-003A` in `proof-obligations.planned.jsonl`.
- `PO-003A` is a required Verus proof-writing obligation for digest-input preconditions. It covers only production-visible workflow-source and compiled-IR digest inputs for `DigestCheck::WorkflowAndIr` and `DigestCheck::Full`.
- Optional downstream digest families remain excluded from `PRE-004`: action ABI and policy digest surfaces have no production input/lookup/check path in this bead and remain waiver rows `PO-021` and `PO-022`.
- All waiver rows in `proof-obligations.planned.jsonl` now retain `status: "planned"` with `required:false` and explicit waiver metadata. Execution outcomes such as waived/pass/fail remain reserved for later verifier/review states.
- `PO-036` keeps explicit limitation metadata for the omitted fuzz, theorem-kernel, and dependency-specific lanes.

## Blockers And Waivers

- No discovery command was blocked.
- Kani: waived unless implementation adds numeric/indexing state transitions not covered by Verus or TLA+.
- Flux: waived; Verus is the selected refinement lane unless Verus cannot express the needed refinement or Flux annotations are introduced.
- Loom/Miri: waived for current scoped risk; recovery files forbid unsafe and no concurrency primitive risk was introduced by planning.
- Fuzz/theorem/dependency lanes: waived/not applicable with compensating TLA+/Verus/proptest/integration/CI evidence; promote if byte-level parser/security risk, theorem-owned proof gaps, or dependency file changes appear.

## State 4 Attempt 5 Completion Evidence

- Isolation command rerun from `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1`; workspace remained outside `/home/lewis/src/velvet-ballistics`.
- Refreshed planning artifacts consume the repaired State 3 schema and preserve planning-only scope.
- JSONL validation required for completion: `jq -c . .beads/vb-qi37.1/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-qi37.1/proof-obligations.planned.jsonl >/dev/null && jq -c . .beads/vb-qi37.1/traceability-matrix.jsonl >/dev/null`.
