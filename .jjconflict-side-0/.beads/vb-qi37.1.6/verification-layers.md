# Verification Layers

## Boundary
- Verus-owned kernel: `RecoveryFrameSeed` construction, `RecoveryHydration` to `RunFrame` pre/postconditions, dimension bounds, taint exactness, fail-closed classification.
- TLA+ temporal model: ordered durable event lifecycle across crash and restart cuts.
- Theorem projection: waived unless State4 discovers a tiny non-Verus algebraic kernel.
- Runtime shell: Fjall reopen integration, runtime recovery boundary, wait/ask/action/collect primitive evidence.
- External systems excluded from formal proof: OS crash semantics beyond drop/reopen simulation, Fjall internal proof, external action side effects.

## Layer Assignment
- PRE-001 -> integration + static-scan.
- PRE-002 -> tla-plus + integration + proptest.
- PRE-003 -> tla-plus + integration + proptest.
- PRE-004 -> verus + proptest + integration.
- PRE-005 -> integration + manual review.
- PRE-006 -> verus + integration + mutation.
- POST-001 -> verus + integration.
- POST-002 -> tla-plus + integration + proptest.
- POST-003 -> tla-plus + integration + proptest.
- POST-004 -> tla-plus + integration.
- POST-005 -> tla-plus + integration.
- POST-006 -> tla-plus + integration + proptest.
- POST-007 -> tla-plus + integration + proptest.
- POST-008 -> verus + integration + mutation.
- INV-001 -> verus + integration.
- INV-002 -> tla-plus + proptest.
- INV-003 -> tla-plus + integration.
- INV-004 -> tla-plus + verus + proptest.
- INV-005 -> verus + proptest + integration.
- INV-006 -> verus + integration + mutation.
- INV-007 -> tla-plus + static-scan/manual review.

## Verus Scope
- Rust targets: `crates/vb_storage/src/recovery/replay/summary.rs`, `crates/vb_storage/src/recovery/hydrate.rs`, `crates/vb_storage/src/recovery/hydrate_support.rs`, `crates/vb_storage/src/recovery/types.rs`, and `crates/vb_core/src/frame.rs` abstractions.
- Spec/proof function names are to be authored in State4; do not treat existing `verification/verus/*` files as covering this bead unless State4 maps them explicitly.
- Invariants: no success without durable state, exact taint, monotonic snapshot-tail application, bounded dimensions, typed fail-closed errors.
- Trusted boundary: validated journal event order, decoded postcard values, validated snapshot metadata, verified digest bundle.
- Shell exclusions: Fjall I/O, OS crash/reopen, external actions, runtime scheduling, wall-clock time.
- Evidence command: `moon run :verify-proof`.

## TLA+ Scope
- Module/model path: planned `verification/tla/RecoveryCrashRestart.tla` and bounded configs.
- Variables: headers, events, snapshots, attempts, slots, taints, waits, asks, actions, collects, crash/recovery status, errors.
- Actions: persist header/snapshot, append journal facts, crash, recover, reject.
- Safety invariants: no stale attempt mixing, snapshot-tail order, exact taint, no duplicated action ticket, collect identity exact, typed rejection for corrupt/unsupported inputs.
- Temporal properties: sufficient durable data eventually recovers; insufficient/corrupt/unsupported data eventually rejects.
- Fairness/deadlock stance: weak fairness on recovery/rejection actions; deadlock only allowed at terminal recovered/rejected states.
- Refinement boundary: Rust journal events and recovery API outcomes refine abstract TLA actions and terminal states.
- Evidence command: `moon run :verify-proof`.

## Evidence Commands
- Focused storage integration: `rustup run nightly-2026-04-28 cargo nextest run --cargo-quiet -p vb_storage --test recovery_integration --all-features`
- Focused runtime unit/collect evidence: `rustup run nightly-2026-04-28 cargo nextest run --cargo-quiet -p vb_runtime --all-features collect --no-capture`
- Workspace recovery contract evidence: `rustup run nightly-2026-04-28 cargo nextest run --cargo-quiet -p velvet-ballistics-workspace-tests --all-features vb_qi37_1_1_red_recovery_contract_test --no-capture`
- Proof gate: `moon run :verify-proof`
- Source gate: `moon run :lint-src`
- Standard gate: `moon run :test`

## Waivers
- Lean/Aeneas/Hax waived by `THM-WAIVE-001` in `lean-contract.md`.
- Loom waived unless State5 implementation introduces new concurrent recovery code; current bead acceptance is crash/restart temporal ordering plus integration evidence, not a new shared-memory concurrent algorithm.
- Performance improvement evidence is a non-goal; no speed claim is made.
