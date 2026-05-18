# Proof Strategy: vb-engine-yaml

## Scope

- Bead: `vb-engine-yaml`.
- State: 4 proof planning, attempt 3 after repaired State 3.
- Planning inputs: repaired `.beads/vb-engine-yaml/contract.md`, `.beads/vb-engine-yaml/proof-obligations.jsonl`, `.beads/vb-engine-yaml/traceability-matrix.jsonl`, `.beads/vb-engine-yaml/delivery-scope.jsonl`, plus State 6 rejection artifacts and prior proof evidence as context only.
- Write boundary: planning artifacts only under `.beads/vb-engine-yaml/`; no production code, tests, proof files, harnesses, models, specs, dependencies, config, or source checkout files were edited.

## Discovery Evidence

- `pwd -P` returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml`.
- `test -s ".beads/vb-engine-yaml/contract.md" && test -s ".beads/vb-engine-yaml/traceability-matrix.jsonl" && test -s ".beads/vb-engine-yaml/delivery-scope.jsonl"` exited 0.
- Risk discovery command: `rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" crates/vb_yaml crates/vb_validate crates/vb_compile crates/vb_core crates/vb_runtime crates/vb_storage crates/vb_ipc crates/velvet_ballastics fuzz kani verification tests xtask .moon Cargo.toml Cargo.lock velvet-ballistics-MASTER.md`.
- Risk discovery result: 12766 matches in 470 scoped files.
- Proof discovery command: `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" crates/vb_yaml crates/vb_validate crates/vb_compile crates/vb_core crates/vb_runtime crates/vb_storage crates/vb_ipc crates/velvet_ballastics fuzz kani verification tests xtask .moon Cargo.toml Cargo.lock velvet-ballistics-MASTER.md`.
- Proof discovery result: 1750 matches in 385 scoped files.
- Discovery was not blocked.

## State 6 Rejection Carry-Forward

- TLA+ admission, lifecycle, recovery, and ingress obligations must require real eventuality/liveness evidence under explicit fairness where the clause claims progress; safety restatements are not enough.
- `TLA-INGRESS-001` must remain model-level evidence for PRE-006/POST-007 ingress/backpressure/typed diagnostics; Loom is implementation interleaving evidence, not a substitute.
- `LOOM-IPC-001` remains required and must use the reviewed command `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime bounded_queue` after the existing Loom model compile repair.
- Kani evidence must be focused or have a longer completed run; the previous timeout is not acceptable evidence.
- Each `EngineYamlError::*` variant now has repaired traceability; downstream tests and evidence must preserve exact typed diagnostic scenarios.

## Risk Classification

- Temporal protocol risk: strict admission, persist-before-ack, lifecycle progress, recovery fail-closed progress, capability lifecycle, direct/API IPC ingress, bounded backpressure, and typed diagnostics require TLA+.
- Rust-local invariant risk: numeric IDs, checked access, budgets, resource bounds, step-state transitions, recovery completeness, and capability/artifact gates require Verus and focused Kani where finite state is valuable.
- Concurrency risk: direct/IPC ingress and runtime queues require Loom or a faithful equivalent; existing review found a compile blocker, so the row remains required and blocked until repaired.
- Untrusted input risk: YAML authoring, artifact envelopes, Postcard/durable records, blobs, and IPC frames require fuzz/property evidence.
- Unsafe/UB and governance risk: first-party crates forbid unsafe, but Miri, supply, banned-token, and dependency-boundary evidence remain required for release acceptance.
- Operator evidence risk: POST-007 requires exact typed diagnostics without creating a text protocol.
- Performance/resource risk: INV-006 needs bounded resource evidence only; this bead cannot claim generated Rust or maxperf parity.

## Verifier Lanes

- TLA+: required for admission, lifecycle, recovery, capability, and ingress/backpressure/operator temporal behavior.
- Verus: required for Rust-local pure/core invariants and abstract models, excluding I/O shells.
- Kani: required for bounded accessor/bytecode/constant/slot/node/idempotency/budget/expression/admission state spaces.
- Loom: required for bounded direct/IPC backpressure concurrency evidence after compile repair.
- Fuzz/proptest: required for hostile input, decode, parity, and broad input-space evidence.
- Miri: required for represented pure/core no-UB evidence through canonical CI.
- CI/release governance: required through `moon ci` for mutation, coverage, supply, dependency-boundary, banned-token, nightly, and performance/resource reports.
- Lean/Aeneas/Hax: waived for this planning pass; reopen only if Verus/Kani cannot cover a tiny pure digest/artifact/lattice kernel.
- Flux: not applicable because no distinct Flux artifact is identified beyond Verus/Kani-owned predicates.

## Execution Order For State 5+

1. Repair or confirm TLA+ models encode non-vacuous progress and ingress backpressure behavior before claiming TLC evidence.
2. Rerun TLC for admission, lifecycle, recovery, ingress, and capability with the exact commands in the obligation rows.
3. Rerun Verus scoped files and preserve trusted-boundary notes.
4. Replace the broad timed-out Kani evidence with the focused harness commands or a completed longer CI run.
5. Repair Loom compile blockers, rerun the exact Loom command, and record raw output.
6. Later execution states collect `moon ci`, fuzz, proptest, Miri, mutation/coverage, supply, and performance evidence without claiming results in this plan.

## Blockers

- No State 4 discovery command was blocked.
- Known downstream blocker: Loom compile failure from State 6 remains a required State 5/implementation repair target, not waived here.
- Known downstream blocker: Kani workspace timeout remains incomplete evidence; focused commands are planned.
