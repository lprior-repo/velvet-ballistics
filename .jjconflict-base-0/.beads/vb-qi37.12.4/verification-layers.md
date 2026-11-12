# Verification Layers: vb-qi37.12.4

## Boundary

- Verus-owned kernel: none for current State 3 because no Rust-local classifier/exception-validator artifact exists; mandatory follow-up if such Rust logic is introduced later.
- TLA+ temporal model: none; temporal waiver in `tla-spec.md`.
- Theorem projection: none; theorem waiver in `lean-contract.md`.
- Runtime shell: shell/Moon/Cargo filesystem execution proven by direct machine-gate evidence.
- External systems excluded from formal proof: Moon scheduler, Cargo/clippy internals, bash, OS filesystem.

## Layer Assignment

- PRE-001 -> direct-new-gate + manual command evidence.
- PRE-002 -> direct-new-gate + path-domain fixture evidence.
- PRE-003 -> direct-new-gate + false-positive/false-negative fixture evidence.
- PRE-004 -> JSONL/static validation of exception artifact if exceptions are implemented.
- POST-001 -> direct-new-gate negative fixture evidence.
- POST-002 -> direct-new-gate clean-tree/exception evidence.
- POST-003 -> `moon run :verify-standard` evidence showing gate is part of canonical verification.
- POST-004 -> `moon run :lint-src` evidence showing hard clippy denies remain effective.
- POST-005 -> fixture evidence for DISCARD-001 through DISCARD-006.
- INV-001 -> direct-new-gate + clippy-hard-deny.
- INV-002 -> deterministic rerun evidence or report comparison.
- INV-003 -> path-domain fixture evidence.
- INV-004 -> exception-validation evidence.
- INV-005 -> fail-closed missing/malformed input evidence.
- INV-006 -> deterministic classifier fixture evidence covering total, mutually exclusive `Violation`/`JustifiedException`/`NonProductionExcluded` outcomes.
- INV-007 -> exception-validation fixture evidence covering malformed syntax, missing owner, missing expiry/follow-up, overbroad path, overbroad class, and production-hiding non-production claims.
- ERR-* -> machine-gate-report entries demonstrating exact error class and non-zero exit.

## Exact Evidence Commands

- Direct gate: `bash scripts/check-ignored-fallible-results.sh`
- Moon direct gate if implemented as a task: `moon run :ignored-fallible-results`
- Canonical path: `moon run :verify-standard`
- Clippy hard-deny path: `moon run :lint-src`
- JSONL validation for State 3 artifacts: `python -m json.tool` per JSON line or equivalent line-by-line parser.

## Verus Scope

- Rust target: none in current State 3; future target must be discovered after implementation if Rust classifier/exception-validator logic exists.
- Spec/proof function: none in current State 3; future contract repair must name concrete Verus spec/proof functions before approval if Rust logic exists.
- Invariants: INV-002, INV-004, INV-006, and INV-007 are currently owned by executable gate evidence; they move to Verus-first proof obligations if Rust-local pure/core logic is introduced.
- Trusted boundary: validated shell/Moon inputs and direct executable evidence.
- Shell exclusions: filesystem traversal, Moon, Cargo/clippy, bash, OS exit status.

## TLA+ Scope

- Module/model path: none.
- Variables/actions/properties: none.
- Fairness/deadlock stance: not applicable.
- Evidence command: none.

## Theorem Scope

- Theorem module: none.
- Rust target: none.
- Abstraction relation: none.
- Shell exclusions: all bead-local behavior is shell/static evidence.

## Waivers

- TLA-WAIVER-001: waived layer `tla-plus`; owner `State 3 rust-contract`; reason `no temporal workflow, queue, retry, lease, lifecycle, scheduler, concurrency, or distributed protocol behavior`; expiry `if gate execution becomes concurrent/distributed or stateful over time`; compensating evidence `GATE-MOON-001` and direct gate exit-status evidence.
- VERUS-WAIVER-001: waived layer `verus`; owner `State 3 rust-contract`; reason `no Rust-local classifier, parser, validator, loop, arithmetic, typestate, or data-structure artifact exists in current scope`; concrete limitation `Verus cannot verify absent Rust code or external shell/Moon/filesystem semantics`; expiry `if State 8/11 introduces Rust-local classifier or exception-validation logic`; compensating evidence `GATE-CLASSIFIER-001`, `GATE-EXC-001`, `GATE-EXC-VALIDATION-001`, `GATE-DETERMINISM-001`, and `GATE-FAIL-CLOSED-001`.
- LEAN-WAIVER-001: waived layer `lean/aeneas/hax`; owner `State 3 rust-contract`; reason `no theorem-critical algebraic kernel beyond executable gate behavior`; expiry `if a theorem-critical classifier lattice is introduced and cannot be expressed in Verus`; compensating evidence `GATE-CLASSIFIER-001` and `GATE-EXC-VALIDATION-001`.

## Required Reports For Later States

- `machine-gate-report.md`: direct gate, negative fixtures, canonical Moon path, clippy hard-deny evidence.
- `formal-verification-report.md`: records planned formal waivers and scoped executable verification results.
