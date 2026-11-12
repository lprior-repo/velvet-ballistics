# Lean Contract Projection: vb-nf2u

## Boundary
- Lean-owned kernel: none for this bead.
- Rust/runtime shell: Makepad/UI reachability, snapshot capture, PNG/report generation, filesystem evidence, `xtask` command execution, `ai-release` aggregation, redaction artifact scanning, and animation freeze/pause behavior.
- External systems excluded from Lean proof: filesystem, image codecs, Makepad rendering, Moon/Cargo command execution, wall-clock time, terminal diagnostics, and release evidence aggregation.

## Lean Scope Decision
Lean is explicitly waived for this bead. The only pure deterministic candidates are the eight-screen inventory bijection, rectangle/layout predicates, and redaction denylist matching. They are small, bounded, and implementation-sensitive; Kani/proptest/unit/integration tests provide cheaper and more direct evidence without pretending to prove UI rendering, filesystem evidence, or shell behavior.

## Waivers
- WAIVE-LEAN-INV-001
  - Contract clauses: INV-001, PRE-001, PRE-002.
  - Owner: State 3 SPECIFY for bead `vb-nf2u`; must be reviewed by independent contract reviewer.
  - Reason: finite eight-element inventory is bounded and Rust-representation-specific; Kani exhaustive enum checks plus proptest permutation/duplicate tests are sufficient and more maintainable than a separate Lean model.
  - Expiry: before downstream formal proof work may claim theorem-level screen inventory correctness.
  - Compensating evidence: `cargo nextest run -p vb_ui_snapshot -p vb_ui_makepad inventory`, `cargo test -p vb_ui_snapshot inventory_bijection`, `cargo kani -p vb_ui_snapshot --harness inventory`, and `moon run :verify-standard`.
- WAIVE-LEAN-INV-002
  - Contract clauses: INV-002, POST-002, POST-004.
  - Owner: State 3 SPECIFY for bead `vb-nf2u`; must be reviewed by independent contract reviewer.
  - Reason: layout safety depends on rendered/fixture geometry and failure diagnostics, not an algebraic transition system; Kani can bound rectangle arithmetic and unit/proptest can exercise geometry cases.
  - Expiry: before any future claim of formally proven UI layout non-overlap.
  - Compensating evidence: Kani rectangle-overlap panic-freedom/bounds proofs, proptest generated rectangles, negative overlap fixture integration tests, mutation tests.
- WAIVE-LEAN-INV-003
  - Contract clauses: INV-003, PRE-004, POST-004, POST-005.
  - Owner: State 3 SPECIFY for bead `vb-nf2u`; must be reviewed by independent contract reviewer.
  - Reason: scanner correctness is over concrete strings/artifacts and denylist policy; fuzz/proptest and negative fixtures catch implementation false passes more directly than a theorem projection.
  - Expiry: before any future claim that the scanner recognizes a formally specified secret language.
  - Compensating evidence: unit scanner tests, proptest generated secret sentinels, cargo-fuzz/bolero hostile artifact text inputs, negative secret fixture, cargo-mutants.
- WAIVE-LEAN-INV-004
  - Contract clauses: INV-004, PRE-003, POST-006.
  - Owner: State 3 SPECIFY for bead `vb-nf2u`; must be reviewed by independent contract reviewer.
  - Reason: deterministic capture and hidden-animation pause are shell/runtime properties involving time injection and rendered artifacts; Lean must not model wall-clock or UI runtime behavior.
  - Expiry: none for this bead; any future pure animation-state transition kernel needs a new Lean contract.
  - Compensating evidence: two-run deterministic snapshot integration test, hidden animation pause unit test, Kani/proptest only for any pure tick-state predicate if added.

## Theorem Obligations
None. `proof-obligations.jsonl` records Lean waivers instead of theorem obligations.

## Non-goals
- No Lean proof of I/O, filesystem, UI rendering, image codecs, Makepad runtime, or `xtask` process behavior.
- No Lean proof of live core/UI parity while the bead remains `blocked-by-core` / `ui-paused`.
