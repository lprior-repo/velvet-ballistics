STATUS: PASS

# State 11 Black-Hat Blocker Repair 2 — vb-nf2u

## Files changed
- `crates/vb_ui_snapshot/src/lib.rs`
- `crates/vb_ui_snapshot/src/layout_kernel.rs`
- `crates/vb_ui_snapshot/src/checks.rs`
- `crates/vb_ui_snapshot/kani/inventory.rs`
- `crates/vb_ui_snapshot/kani/layout_predicates.rs`
- `crates/vb_ui_snapshot/tests/layout_checks.rs`
- `crates/vb_ui_snapshot/tests/report_evidence_shape.rs`
- `xtask/src/evidence.rs`
- `xtask/tests/ui_release_errors.rs`
- `xtask/tests/ui_release_gates.rs`
- `tests/vb_nf2u_ui_release_acceptance.rs`
- `scripts/rust-verification-gauntlet.sh`
- `.beads/vb-nf2u/verification-layers.md`
- `.beads/vb-nf2u/proof-obligations.jsonl`
- `.beads/vb-nf2u/test-plan.md`
- `.beads/vb-nf2u/lean-contract.md`

## Design decisions
- Kani command contract was updated from invalid positional package IDs to executable harness filters: `cargo kani -p vb_ui_snapshot --harness inventory` and `cargo kani -p vb_ui_snapshot --harness layout_`.
- `vb_ui_snapshot` now includes Kani harnesses under `#[cfg(kani)]`, and `moon run :verify-all` invokes the two bead-specific Kani commands when `VERIFY_BEAD_ID=vb-nf2u` is set.
- Layout checks no longer key off filename substrings. They parse deterministic fixture geometry from fixture metadata text and evaluate pure rectangle predicates in `layout_kernel`.
- `UiReleaseGateConfig::for_bead` now returns `Result`; unknown bead IDs are an internal error state, not silently remapped to `vb-nf2u`.
- Positive `ai-release` evidence is assembled through typed `UiReleaseBundle`, `UiSubgateRun`, screen rows, check rows, and validation before writing fixture-backed YAML/text artifacts.
- Negative fixture state now uses `FixtureReadState::{Present, Missing}` instead of `Option<String>` string mush.
- Reviewed substring tests were replaced with typed/domain parsing where targeted: subgate names are parsed into exact vectors, and fixture-backed/no-parity claims are parsed as top-level fields.

## Blocker satisfaction
1. Kani contract parity: PASS. Actual harness commands executed and `moon run :verify-all` now runs harnesses instead of reporting “No proof harnesses”.
2. Positive UI release evidence: PASS. Evidence remains fixture-backed, but is typed and validated before emission; no live Makepad/core parity claim is made.
3. Layout checks: PASS. Overlap, clipping, bounds, chip readability, and selected-state checks inspect deterministic geometry fields.
4. Unknown bead config: PASS. `UiReleaseGateConfig::for_bead("vb-nf2u-missing")` returns `ReleaseProfileIncomplete`.
5. Tests: PASS. Focused tests assert parsed exact domain structures for changed blocker surfaces.
6. Optional text-state workflow: PASS. Negative fixture reads use explicit read-state enum and false-pass result states.

## Command evidence
- PASS: `bd prime` loaded bead workflow context; Dolt auto-push warning was unrelated to this workspace repair.
- PASS: `cargo kani -p vb_ui_snapshot --harness inventory`; output path `/home/lewis/.local/share/opencode/tool-output/tool_e10b3de3e001h3qsqpx8tIXbv3`; summary: `Complete - 1 successfully verified harnesses, 0 failures, 1 total`.
- PASS: `cargo kani -p vb_ui_snapshot --harness layout_`; output path `/home/lewis/.local/share/opencode/tool-output/tool_e10b3f06e002uWP0en2asd2wEy`; summary: `Complete - 5 successfully verified harnesses, 0 failures, 5 total`.
- PASS: `cargo nextest run -p vb_ui_snapshot -p xtask`; summary `130 tests run: 130 passed, 0 skipped`.
- PASS: `cargo nextest run -p velvet-ballistics-workspace --test vb_nf2u_ui_release_acceptance`; summary `8 tests run: 8 passed, 0 skipped`.
- PASS: `rtk cargo fmt --all --check`; no output.
- PASS: `rtk cargo clippy -p vb_ui_snapshot -p xtask --tests --all-features -- -D warnings`; wrapper summary `0 errors, 2 warnings`.
- PASS: `moon run :verify-all`; output path `/home/lewis/.local/share/opencode/tool-output/tool_e10b9b0d3001hNI1uqosxS1T4Y`; summary `Tasks: 1 completed`, Kani summaries show 1 inventory and 5 layout harnesses verified.
- PASS: `moon ci --base HEAD --head HEAD`; output path `/home/lewis/.local/share/opencode/tool-output/tool_e10baddaf001BqOE3PWBBUTKUa`; summary `Tasks: 20 completed (2 cached)`.

## Power-of-Ten / zero-panic rules affected
- Panic freedom: production changes did not add `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, or `unsafe`.
- Checked arithmetic: layout geometry uses `checked_add`, `checked_sub`, and `checked_mul` in pure predicates, with Kani coverage for overflow/panic freedom.
- Bounded control: Kani predicates and fixture validation use bounded finite screen/check inventories.
- Typed failures: unknown bead and release-profile errors remain typed; negative fixture state is explicit.

## Performance-layer decision
- No performance claim made. No benchmark/profiler evidence required.

## Second-ring evidence
- Kani formal evidence attached via the two harness command outputs and `moon run :verify-all` output above.
- No assembly/IR/API/provenance claim made; no cargo asm/semver/SBOM evidence required for this repair.

## Skipped gates
- No required gate from the user request was intentionally skipped.

## Residual risks
- UI evidence is still fixture-backed only by accepted constraint: `fixture_backed: true`, `core_runtime_parity_claim: unsupported`.
- Lockbud remains bead-waived per prior approved waiver; this repair did not add concurrency.
- Kani harnesses verify bounded Rust predicate kernels, not live Makepad rendering or image codec behavior.
