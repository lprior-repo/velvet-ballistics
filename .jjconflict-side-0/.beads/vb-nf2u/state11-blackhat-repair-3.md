STATUS: PASS

# State 11 Black-Hat Blocker Repair 3 — vb-nf2u

## Files changed
- `xtask/src/evidence.rs`
- `crates/vb_ui_snapshot/src/checks.rs`
- `crates/vb_ui_snapshot/src/layout_kernel.rs`
- `crates/vb_ui_snapshot/kani/layout_predicates.rs`
- `crates/vb_ui_snapshot/tests/layout_checks.rs`
- `tests/vb_nf2u_ui_release_acceptance.rs`
- `xtask/tests/ui_release_gates.rs`

## Blocker-by-blocker repair summary
1. Positive `ai-release` evidence now derives subgate rows from typed `Result` outcomes with explicit origins: snapshot inventory, layout predicates, redaction scan, negative fixture validation, deterministic capture, and evidence-shape validation.
2. Required negative fixture files now fail closed when absent or malformed. Manual command `rm -rf "target/vb-nf2u-negative-fixtures" && cargo xtask ai-release --bead vb-nf2u` exits non-zero with `Missing evidence for gate 'negative_fixture'`.
3. Acceptance helper assertions now parse evidence into typed test-domain structs for subgates, snapshot inventory/checks, and redaction screen/class coverage instead of block slicing.
4. Layout fixture parsing now uses trusted `Rect` boundary type with checked construction; missing required applicable geometry/control-state fields return typed `UiSnapshotError::TokenParseError` rather than permissive defaults.
5. `xtask/src/evidence.rs` now separates domain validation/calculation (`UiReleaseBundle::fixture_backed`, subgate outcome derivation, negative fixture parsing) from artifact writers.
6. Release evidence/check rows now carry explicit provenance/origin, reducing the ability to construct all-green evidence without source outcome context.

## Command results
- PASS: `cargo nextest run -p velvet-ballistics-workspace --test vb_nf2u_ui_release_acceptance` — 8 passed, 0 skipped.
- PASS: `cargo nextest run -p vb_ui_snapshot -p xtask` — 130 passed, 0 skipped.
- PASS: `cargo kani -p vb_ui_snapshot --harness inventory` — 1 harness verified, 0 failures.
- PASS: `cargo kani -p vb_ui_snapshot --harness layout_` — 5 harnesses verified, 0 failures.
- PASS: `moon run :verify-all` — completed inside combined run output `/home/lewis/.local/share/opencode/tool-output/tool_e10d4be1c001o2s3eIa8NjUq0B`.
- PASS: `moon ci --base HEAD --head HEAD` — `Tasks: 20 completed (2 cached)`, same output path above.
- PASS: `rtk cargo fmt --all --check` — no output.
- PASS: `rtk cargo clippy -p vb_ui_snapshot -p xtask --tests --all-features -- -D warnings` — 0 errors, wrapper reported 2 pre-existing warnings.
- PASS expected-fail check: missing negative fixtures command failed closed with `Missing evidence for gate 'negative_fixture'`.

## Power-of-Ten / zero-panic rules affected
- Typed boundary validation strengthened: layout geometry is no longer a raw tuple alias and malformed fixture fields fail explicitly.
- Checked arithmetic preserved in layout kernels; Kani reverified inventory and layout harnesses.
- No production `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` introduced.
- Bounded finite screen/check/subgate inventories remain explicit.

## Performance-layer decision
- No performance claim made. No benchmark/profiler evidence required.

## Second-ring evidence
- Kani inventory/layout proof commands passed.
- No assembly/IR/API/provenance claim made; no extra second-ring evidence required.

## Skipped gates
- None of the user-requested gates were intentionally skipped.

## Residual risks
- Evidence remains deterministic fixture-backed only; no live Makepad rendering or core runtime parity is claimed.
- Lockbud remains bead-waived under the previously approved bead-scoped waiver.
- Existing workspace warnings about duplicate Makepad `bitflags` and duplicate binary target names remain outside this repair.
