# Proof Strategy: vb-qi37.12

## Scope

- Bead: `vb-qi37.12`.
- State: 4 proof planning repair after State 3 schema repair.
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12`.
- Source checkout write boundary: `/home/lewis/src/velvet-ballistics` is forbidden for writes.
- Planning outputs only: `.beads/vb-qi37.12/proof-strategy.md`, `.beads/vb-qi37.12/proof-plan-review-input.md`, and `.beads/vb-qi37.12/proof-obligations.planned.jsonl`.

## Inputs Read

- Repaired State 3: `contract.md`, `tla-spec.md`, `verification-layers.md`, `lean-contract.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`.
- Scope: `codebase-map.md`, `delivery-scope.jsonl`.
- Rejection context: approved `proof-review.md`, rejected `contract-verification-review.md`, `proof-findings.jsonl`, and `proof-repair-guide.md`.
- Prior evidence context only: `proof-evidence.md`, `proof-writer-report.md`.

## Discovery Evidence

- `pwd -P` returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-12`.
- Isolation guard against `/home/lewis/src/velvet-ballistics` exited 0.
- `test -s` passed for `contract.md`, `traceability-matrix.jsonl`, and `delivery-scope.jsonl`.
- `jq -c .` passed for `proof-obligations.jsonl`, `traceability-matrix.jsonl`, and `delivery-scope.jsonl`.
- Scoped risk scan over delivery files found storage/runtime state transitions, recovery, retry/cancel, serialization/deserialization, `Mutex`, and production/test assertion signals.
- Scoped verifier scan over delivery files found first-party `#![forbid(unsafe_code)]` markers and no existing in-production TLA/Verus/Kani/Loom/proptest/fuzz annotations for this bead surface.
- No discovery command was blocked; State 4 repair reused the scoped discovery evidence and refreshed the plan against repaired State 3 schema.

## State 4 Repair Reflected

- Active planned rows remain reviewable plans with `status:"planned"`; execution evidence stays in `proof-execution-ledger.jsonl`, `proof-evidence.md`, and review artifacts.
- TLA+ rows use exact bead-local artifact paths, `TMPDIR=target/tmp` TLC commands, and the repaired State 3 TLA metadata fields: module, model, config, variables, actions, invariants, temporal properties, fairness, state constraints, and refinement.
- `TLA-DEADLOCK-011` is an explicit required TLA row with deadlock-freedom metadata, no `CHECK_DEADLOCK FALSE` waiver, and no explicit unconditional `Stutter` assumption.
- `TEST-JOURNAL-007` and `TEST-RUNTIME-008` use the focused package/test-selector commands from repaired State 3; only `GATE-RELEASE-010` owns `moon ci`.
- `SCAN-DISCARD-006` and `FUZZ-DECODE-009` remain planned obligations, not completed rows, even though prior State 5/6 evidence exists for reviewer context.
- Prior Kani/proptest lanes are not kept as required proof obligations because repaired State 3 moved active coverage to Verus, TLA+, focused static scan, fuzz, tests, and `moon ci`. They are listed as `not_applicable` unless later implementation introduces a new bounded harness/property-test requirement.

## Required Lanes

- TLA+: persistence-before-ack, recovery fail-closed lifecycle, diagnostic-cause temporal preservation, and separate deadlock evidence with mandatory TLA metadata.
- Verus: discard classification lattice, diagnostic envelope preservation, and recovery decode classification as abstract Rust-local kernels.
- Focused static scan: classify every scoped silent-discard candidate and prove zero unclassified release-critical silent discards.
- Fuzz: malformed persisted payloads must not panic or hydrate as empty success; the planned command names the repaired wired target and local TMPDIR/CARGO_TARGET_DIR workaround.
- Focused tests and `moon ci`: journal/storage and runtime focused commands are explicit; release-critical repository evidence remains the separate State 11 `moon ci` gate.

## Waivers And Non-Applicable Lanes

- Lean/Aeneas/Hax: waived because repaired State 3 identifies no theorem-only kernel beyond Verus.
- Kani: not applicable to the repaired active contract unless State 5/8 introduces a bounded harness requirement; previous rejected stale `PO-007` is not an active repaired obligation.
- Proptest: not applicable to the repaired active contract unless State 8 adds a property target; static classification plus focused tests own the current inventory evidence.
- Loom: not applicable unless implementation adds concurrent behavior or scheduler interleavings.
- Miri: not applicable while first-party scope remains `#![forbid(unsafe_code)]` and no unsafe/FFI/raw-pointer risk is introduced.
- Flux: not applicable because Verus owns the selected Rust-local proof kernels.
- Dependency audit: not applicable because all delivery-scope rows set `dependencies_changed=false`.

## Review Gate

- `proof-plan-review-input.md` is the human review packet.
- `proof-obligations.planned.jsonl` is the machine-readable obligation matrix.
- No State 4 repair row claims verifier PASS or implementation evidence.
