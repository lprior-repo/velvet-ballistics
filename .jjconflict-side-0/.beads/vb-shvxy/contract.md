# Contract: vb-shvxy Formal Verifier Tooling

## Contract Clauses

- **C-001 Lane closure**: All verifier evidence must reference one closed `LaneId`: Kani, Flux, TLA/TLC, proptest, fuzz, or Loom.
- **C-002 Availability preflight**: Missing scripts, missing tools, and missing jars produce typed blockers before evidence classification.
- **C-003 Non-vacuous success**: Exit status 0 is not sufficient. Behavior evidence requires nonzero applicable tests/harnesses/models/targets.
- **C-004 Evidence classification**: Setup health and inventory logs cannot close behavior-affecting proof or test obligations.
- **C-005 Kani feature parity**: `KANI_FEATURES` and harness selectors must match declared package features and known harness inventory before execution evidence is accepted.
- **C-006 Flux wrapper shape**: Flux package wrapper accepts package-only selection; unsupported target selectors are command-spec blockers.
- **C-007 TLC portability**: TLC commands must use the canonical runner policy and preserve raw status output; hardcoded missing `tools/tla2tools.jar` is blocked.
- **C-008 Proptest zero-test guard**: Cargo test/proptest commands that report zero executed tests are blockers even when exit code is 0.
- **C-009 Fuzz target/sanitizer guard**: cargo-fuzz commands must preflight registered target names and use sanitizer-compatible target triples.
- **C-010 Loom cfg/dependency guard**: `cfg(loom)` execution requires dependency wiring valid for the compiled package/test graph.
- **C-011 Fresh evidence boundary**: Prior capped evidence can justify scope but cannot be reclassified as fresh pass evidence.
- **C-012 Fail closed on unknowns**: Unknown parser output, missing counts, ambiguous targets, or unclassified evidence become blockers.

## Acceptance Criteria for Later States

1. Each lane has explicit preflight checks matching clauses C-002 through C-010.
2. Evidence parser or wrappers reject zero applicable tests/harnesses/models.
3. Formal reports distinguish setup health, inventory, and behavior evidence.
4. Raw command output is preserved with enough content to audit command, target, counts, and status.
5. Any command-spec migration from prior capped bead is represented as a blocker or rewritten to current declared features/targets.

## Non-Goals

- No production Rust behavior changes.
- No verifier harnesses, TLA specs, test code, or proof obligations are authored in this state.
- No claim that any lane is repaired or proof-complete.
