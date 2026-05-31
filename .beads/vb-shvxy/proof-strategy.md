# Proof Strategy: vb-shvxy (Global Tooling Blocker)

## Bead Context
- **Bead**: vb-shvxy
- **Kind**: global-blocker (tooling infrastructure)
- **State**: 4 (proof-planner)
- **Original blocked bead**: vb-ttyc (State 12 FAIL_LOCAL — missing Kani/Flux/proptest/fuzz/Loom lanes)

## Strategy Summary

This bead restores formal verifier tooling LANES — not production behavior. Every proof obligation targets a tooling script, wrapper, or command-spec contract. The goal is NON-VACUOUS evidence that each verifier lane can produce valid, auditable output.

### Verifier Lane Targets

| Lane | Status | Script/Wrapper | Key Risk |
|------|--------|---------------|----------|
| Kani | Present | `scripts/kani-list.sh` | Feature gate drift, inventory-only vs execution |
| Flux-rs | Present | `scripts/flux-check-package.sh` | Unsupported selector guard |
| proptest | Missing guard | None (needs zero-test detector) | Vacuous success (`running 0 tests` + exit 0) |
| cargo-fuzz | Partial | `.moon/tasks/all.yml` fuzz-smoke | Musl+sanitizer incompatibility, unset default target |
| Loom | Partial | `xtask/src/loom.rs` | Dev-dependency leak into library cfg |
| TLA+ | **GLOBALLY REMOVED** | N/A | No lane planned |

### Provenance
- Verus is already working (`bash scripts/verify-verus.sh`). Its verified scripts and registry-driven execution serve as the TRUSTED BASE template for the other lanes.
- Prior capped evidence from `vb-ttyc` State 12 is CONTEXT ONLY; no fresh pass evidence is inherited.

## Lane Decision Rationale

### Kani — required
The script exists (`scripts/kani-list.sh:1-66`) and `cargo-kani 0.67.0` is on PATH. Obligations verify:
1. Script produces valid JSON inventory for packages with harnesses.
2. Feature gate requests match declared package features (or fail closed).
3. Inventory evidence is NOT confused with harness execution evidence (evidence classification guard).

### Flux-rs — required
The wrapper exists (`scripts/flux-check-package.sh:1-21`). Obligations verify:
1. Package-level `cargo flux -p <package>` smoke succeeds.
2. Unsupported selectors (`--lib`, `--test`, etc.) are rejected before execution.
3. Package pass is classified as setup/refinement smoke, not behavior proof.

### proptest — required
No dedicated script exists. The hazard is `cargo test` exiting 0 with "running 0 tests". Obligations require:
1. A fail-closed detector (script or parser) that rejects zero-applicable output.
2. Proof that existing proptest tests with valid filters produce non-zero execution counts.

### cargo-fuzz — required
`cargo fuzz` is available. The hazard is musl+sanitizer incompatibility. Obligations verify:
1. Fuzz targets are registered (`cargo fuzz list` succeeds).
2. Fuzz build uses explicit GNU target (`--target x86_64-unknown-linux-gnu`).
3. Missing/unregistered targets fail closed.

### Loom — required
`cfg(loom)` is allowed but `loom` is dev-dependency only. Obligations verify:
1. Library-level loom models compile under `#[cfg(loom)]`.
2. The dependency wiring resolves for the selected build target.
3. Integration-test-only loom models fail closed when dependency is unavailable.

### TLA+ — not_applicable
TLA+ has been GLOBALLY REMOVED from the repository. No TLA/TLC lane is planned. Existing seeds referencing TLA (seed-003) are resolved as not_applicable. The contract clause C-007 is noted for waiver.

## Non-Vacuity Principle

Every lane obligation includes:
- **Applicable count > 0**: At least one harness/model/test/target must execute.
- **Classification guard**: Inventory/setup/version output is never accepted as behavior evidence.
- **Raw evidence preservation**: Command output must retain enough lines to audit status, counts, and errors.

## Fail-Closed Policy

Every lane fails closed:
- Missing script → `MissingScript` blocker
- Missing tool → `MissingExecutable` blocker
- Zero applicable → `ZeroApplicableTests` blocker
- Undeclared feature → `UndeclaredFeature` blocker
- Incompatible target/sanitizer → `IncompatibleTargetSanitizer` blocker
- Unresolved cfg dependency → `InvalidCfgWiring` blocker

## Open Decisions for Downstream

1. Kani wrapper pass-through: should `scripts/kani-list.sh` support `--harness` for execution, or remain inventory-only?
2. Kani feature migration: should `vb_runtime/kani-artifact-version-barrier` be restored or obligations rewritten?
3. Proptest zero-test detector: script, evidence parser, or cargo-test helper?
4. Loom wiring: real Cargo feature, optional dependency, or package-test-only?
5. TLA removal cleanup: should C-007 be waived, removed, or replaced with a no-TLA assertion?
