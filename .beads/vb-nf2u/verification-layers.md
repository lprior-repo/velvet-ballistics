# Verification Layers: vb-nf2u UI release gates

## Boundary
- Verified kernel: finite inventory mapping, rectangle/layout predicates, redaction denylist scanner, report evidence validation, deterministic snapshot-mode state.
- Runtime shell: `cargo xtask ui-snapshot`, `cargo xtask ui-overlap-check`, and the required aggregate release entrypoint `cargo xtask ai-release --bead vb-nf2u`.
- Required UI release entrypoint: `cargo xtask ai-release --bead vb-nf2u` must invoke or certify UI snapshot, layout/readability, redaction, negative-fixture, deterministic-capture, and evidence-shape subgates. No separate `ui-redaction-check` or `ui-release-gate` command is required by this contract.
- External systems excluded from formal proof: Makepad rendering, image codec internals, OS filesystem semantics, wall-clock implementation, Moon/Cargo process execution.

## Layer Assignment
- PRE-001 / INV-001 -> unit + proptest + Kani: inventory must be exactly eight and reject missing/duplicate/extra/unknown entries.
- PRE-002 / INV-001 -> unit + integration + Kani: `ShellNav`, `Screen`, `UiScreenKind`, `REQUIRED_FIXTURES`, fixture loading, and report IDs form a bijection.
- PRE-003 / INV-004 -> unit + integration + Miri: release snapshot mode fixes time, pauses hidden animations, and exits via guard without UB-sensitive behavior.
- PRE-004 / INV-003 -> unit + proptest + fuzz/bolero + `ai-release`: denylist must include raw secret sentinels, API-key/token/password/idempotency-key examples, and tainted fixture values; scanner must reject hostile text/artifacts without panic.
- POST-001 / INV-005 -> integration + coverage: all eight `.fixture.txt` artifacts/reports/digests/checks exist and are readable.
- POST-002 / INV-002 -> unit + proptest + Kani + negative fixture integration: overlap, clipping, bounds, chip readability, and selected-state checks execute, emit evidence, and fail bad fixtures.
- POST-003 -> integration + gauntlet-all: `ai-release` includes UI snapshot, overlap/layout, redaction, negative-fixture, determinism, and evidence-shape gates.
- POST-004 -> unit + integration + mutation: each violation yields a typed error and diagnostic naming screen plus offending control/secret class.
- POST-005 -> integration + mutation: intentional overlap and intentional secret fixtures fail; inverted pass/fail mutants are killed.
- POST-006 -> integration + static scan: two captures in separate temp dirs have identical normalized reports and stable `blake3:` fixture-text digests; no wall-clock/random usage in release capture.
- Error taxonomy -> one unit/integration/mutation obligation per `UiReleaseGateError` variant: every variant is reachable from a negative scenario, emits a typed diagnostic shape, and has command evidence.

## Required Commands and Evidence
- Unit/integration: `cargo nextest run -p vb_ui_snapshot -p vb_ui_makepad -p xtask` -> `.evidence/vb-nf2u/nextest-ui-release.txt`.
- Snapshot all screens: `cargo xtask ui-snapshot --all --emit yaml --output-dir .evidence/vb-nf2u/ui_snapshots` -> `.evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml` plus eight `.fixture.txt` artifacts for this fixture-backed/no-live-Makepad bead boundary.
- Layout gate: `cargo xtask ui-overlap-check --all --input-dir .evidence/vb-nf2u/ui_snapshots` -> `.evidence/vb-nf2u/ui-layout-report.yaml`; `ai-release` must include equivalent layout/readability evidence for overlap, clipping, bounds, chip readability, and selected state.
- Redaction gate: `cargo xtask ai-release --bead vb-nf2u` -> `.evidence/vb-nf2u/ai-release.yaml` containing UI redaction subgate evidence and denylist coverage. No separate redaction command is required.
- Release aggregation: `cargo xtask ai-release --bead vb-nf2u` -> `.evidence/vb-nf2u/ai-release.yaml` containing UI snapshot, layout/readability, redaction, negative-fixture, deterministic-capture, and evidence-shape subgate evidence.
- Proptest: `cargo nextest run -p vb_ui_snapshot proptest` -> `.evidence/vb-nf2u/proptest-ui.txt`.
- Kani inventory: `cargo kani -p vb_ui_snapshot --harness inventory` -> `.evidence/vb-nf2u/kani-ui.txt`.
- Kani layout predicates: `cargo kani -p vb_ui_snapshot --harness layout_` -> `.evidence/vb-nf2u/kani-layout.txt`, covering overlap symmetry, clipping/bounds containment, readable chip area thresholds, selected-state visibility thresholds, panic freedom, and checked arithmetic.
- Fuzz/Bolero: `cargo fuzz run ui_redaction_artifact -- -runs=10000` or `cargo bolero test ui_redaction_artifact` -> `.evidence/vb-nf2u/fuzz-redaction.txt`.
- Miri: `cargo +nightly miri test -p vb_ui_snapshot` -> `.evidence/vb-nf2u/miri-ui-snapshot.txt`.
- Mutation: `cargo mutants -p vb_ui_snapshot --in-place --timeout 120` scoped to UI checks -> `.evidence/vb-nf2u/mutants-ui.txt`.
- Coverage: `cargo llvm-cov nextest --package vb_ui_snapshot --package xtask --lcov --output-path .evidence/vb-nf2u/lcov.info` -> `.evidence/vb-nf2u/lcov.info`.
- Static scan: `moon ci` and banned-token/source scans -> `.evidence/vb-nf2u/moon-ci.txt`.
- Five-lane gauntlet: `moon run :verify-fast`, `moon run :verify-standard`, `moon run :verify-deep`, `moon run :verify-proof`, and `moon run :verify-all` are unconditional release-critical evidence boundaries for this bead. If a lane is absent in the workspace, downstream formal-verifier must add it from `../formal-verifier/templates/`; absence is not passing evidence.

## Structured Waivers

### WAIVE-LEAN-UI-SHELL
- Clause ID: Lean-Owned Clauses; PRE-001, PRE-002, PRE-004, POST-002, POST-004, INV-001, INV-002, INV-003, INV-004.
- Waived layer: Lean.
- Reason: this bead verifies UI/release shell behavior and Rust-representation-specific finite predicates; theorem-level claims would either model I/O/UI incorrectly or duplicate Kani/proptest/fuzz evidence.
- Compensating evidence: `cargo kani -p vb_ui_snapshot --harness inventory`, `cargo kani -p vb_ui_snapshot --harness layout_`, proptest, fuzz/bolero, integration, mutation, and independent contract review.
- Owner: State 3 `rust-contract` for bead `vb-nf2u`; downstream formal-verifier owns expiry enforcement.
- Expiry/follow-up: expires before any future theorem-level claim of inventory, scanner-language, or layout correctness.

### WAIVE-CONCURRENCY-UI-RELEASE
- Clause ID: PRE-003, POST-003, POST-006, INV-004, INV-005.
- Waived layer: Loom/Shuttle/Lockbud.
- Reason: current contract requires deterministic single-process fixture capture and does not add async tasks, shared mutable state, channels, or concurrent cancellation semantics.
- Compensating evidence: Miri, two-run deterministic integration evidence, static scan for spawned tasks/shared state in release capture, and `moon run :verify-deep`.
- Owner: State 3 `rust-contract` for bead `vb-nf2u`; implementation owner must revoke if tasks/shared state are introduced.
- Expiry/follow-up: expires immediately if `ai-release` UI capture uses threads, async tasks, channels, shared mutable state, or cancellation.

### WAIVE-PERF-ASM-UI-RELEASE
- Clause ID: Non-goals; POST-002, POST-003.
- Waived layer: performance and assembly-ir.
- Reason: the contract makes no throughput, latency, vectorization, allocation, code-size, or zero-cost abstraction claim.
- Compensating evidence: functional release gates, mutation, coverage, `moon ci`, and `moon run :verify-all`.
- Owner: State 3 `rust-contract` for bead `vb-nf2u`.
- Expiry/follow-up: expires if any downstream artifact claims speed, latency, vectorization, zero-cost abstraction, or release-gate performance budget.

### WAIVE-API-COMPAT-UI-RELEASE
- Clause ID: Contract Signatures.
- Waived layer: api-compat.
- Reason: proposed signatures are contract targets for internal bead implementation and do not assert stabilized public crate API compatibility.
- Compensating evidence: `cargo check`, `moon ci`, and downstream review of public API changes.
- Owner: State 3 `rust-contract` for bead `vb-nf2u`; implementation owner must revoke if public exported APIs change.
- Expiry/follow-up: expires if any public crate API, exported type, or semver-visible command contract changes; then run `cargo semver-checks` or equivalent.

### WAIVE-RELEASE-PROVENANCE-UI-RELEASE
- Clause ID: POST-003, INV-005, INV-006.
- Waived layer: release-provenance/SBOM as UI-specific evidence.
- Reason: this bead gates UI evidence shape and fail-closed behavior, not package provenance; workspace release provenance remains owned by existing `ai-release` and Moon supply-chain gates.
- Compensating evidence: `cargo xtask ai-release --bead vb-nf2u`, `moon ci`, `moon run :verify-all`, cargo deny/vet/auditable evidence if required by the existing release profile, and explicit no-core-parity static scan.
- Owner: State 3 `rust-contract` for bead `vb-nf2u`; release owner owns workspace provenance evidence.
- Expiry/follow-up: expires if UI artifacts become published release assets or if `ai-release` removes supply-chain/provenance gates.

## Independent Review Gate
Downstream State 4+ work must not consume these artifacts until an independent reviewer writes `.beads/vb-nf2u/contract-verification-review.md` with `STATUS: APPROVED`.
