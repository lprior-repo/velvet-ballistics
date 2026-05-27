# Proof Strategy — vb-om21

State 4 proof-planner artifacts only. No proof code, production code, tests, or review dispositions were written.

## Scope
Plan defense-in-depth proof obligations for journal tail scan fallback: prefix-bounded Fjall run_event key scanning, big-endian max sequence selection, checked tail reconstruction, metadata validation, missing journal classification, replay parity, parser safety, and O(1) scan state.

## Risk Classification
- temporal/state-machine: scan lifecycle, metadata-before-recovery ordering, MissingJournal/TailMismatch outcomes.
- Rust-local invariant: prefix classification, max fold, checked addition, typed outcomes.
- bounded state: two-run/key-order models, edge sequences 0/1/MAX-1/MAX.
- refinement/type-state: RunEventPrefix, RunEventKey, TailMetadata, TailScanMode, TailScanResult illegal states.
- concurrency: no async/spawned task lane; snapshot consistency is a trusted storage-shell assumption for this bead.
- unsafe/UB: unsafe forbidden; Miri scoped only to key parser panic/index boundary.
- untrusted input: Fjall storage keys at parser boundary; cargo-fuzz required for key bytes.
- performance/resource: O(1) accumulator and prefix-bounded scan; no full event collection for tail query.

## Lane Summary
- tla-plus: 6 required, 5 not_applicable, 0 blocked_tooling
- verus: 11 required, 0 not_applicable, 0 blocked_tooling
- kani: 11 required, 0 not_applicable, 0 blocked_tooling
- flux-rs: 11 required, 0 not_applicable, 0 blocked_tooling
- loom: 0 required, 11 not_applicable, 0 blocked_tooling
- miri: 1 required, 10 not_applicable, 0 blocked_tooling
- proptest: 11 required, 0 not_applicable, 0 blocked_tooling
- cargo-fuzz: 1 required, 10 not_applicable, 0 blocked_tooling

## Required command families
Exact per-obligation commands are in `proof-obligations.planned.jsonl`. Proof-writer/formal-verifier must not substitute whole-workspace proof blasts for scoped commands without updating obligations and evidence.

## Non-applicable lane rationale
Loom is not applicable for all current seeds because this contract has no async/task-spawn/cancellation/lock-free behavior; storage read consistency is bounded by the Fjall snapshot boundary. Miri is not applicable except `ps-vb-om21-key-parse` because unsafe/FFI/provenance risks are absent. cargo-fuzz is not applicable except key parser hostile-byte input because other seeds are typed arithmetic/state/refinement properties rather than parser/codec surfaces.

## Blockers
No tooling blocker is asserted by the planner. Downstream states may report blocked_tooling only with raw command evidence.
