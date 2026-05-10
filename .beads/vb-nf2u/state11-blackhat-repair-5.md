STATUS: PASS

# State 11 Black-Hat Blocker Repair 5 — vb-nf2u

## Files changed
- `xtask/src/evidence.rs`
- `crates/vb_ui_snapshot/src/checks.rs`
- `tests/vb_nf2u_ui_release_acceptance.rs`
- `xtask/tests/ui_release_gates.rs`
- `.beads/vb-nf2u/state11-blackhat-repair-5.md`

## Blocker-by-blocker repair summary
1. Positive `ai-release` evidence now derives screen artifact facts from deterministic artifact bytes, computes digests from those bytes, executes layout predicates through `vb_ui_snapshot::layout_kernel`, scans the typed release document text tree, and validates `UiReleaseDocument` before any filesystem writes.
2. Cited functions were split below 25 lines: acceptance overlap tests now delegate field assertions, `xtask/tests/ui_release_gates.rs` delegates command/read/expected-subgate construction, and `check_spelling` delegates spelling extraction/violation construction.
3. Release document construction/validation is separated from the thin filesystem shell (`UiReleaseDocument::from_bundle` / `validate` before `write_release_document`).
4. Boolean deterministic-capture flags were replaced with typed states: `HiddenAnimationState::Paused`, `ClockSource::FixedFixtureTime`, and `CaptureTimestamp::Fixed`.
5. Required negative fixture `actual_status` is now fail-closed; missing status is `MissingEvidence` instead of defaulting to `failed`. Layout fixture parsing uses explicit `FixtureValue::Present/NotApplicable` rather than neutral empty/zero/hidden/rect defaults.
6. Release evidence is rendered from typed document state after validation. YAML/text emission is no longer the proof surface.

## Command results
- PASS: `cargo nextest run -p velvet-ballastics-workspace --test vb_nf2u_ui_release_acceptance` — 8 passed.
- PASS: `cargo nextest run -p vb_ui_snapshot -p xtask` — 130 passed.
- PASS: `cargo kani -p vb_ui_snapshot --harness inventory` — 1 harness verified, 0 failures; output `tool_e11162583001NIwZ78ns4nz0mx`.
- PASS: `cargo kani -p vb_ui_snapshot --harness layout_` — 5 harnesses verified, 0 failures; output `tool_e1116357c001FXBm36VCFa0RMO`.
- PASS: `moon run :verify-all` — `Tasks: 1 completed`; output `tool_e11189f51001QHYkDBJu0aI0gO`.
- PASS: `moon ci --base HEAD --head HEAD` — `Tasks: 20 completed (2 cached)`; output `tool_e1119e5ae0011UqVKPbT3CaR0F`.
- PASS: `rtk cargo fmt --all --check`.
- PASS: `rtk cargo clippy -p vb_ui_snapshot -p xtask --tests --all-features -- -D warnings` — 0 errors, wrapper reported 2 existing warnings.
- PASS expected-fail: `rm -rf "target/vb-nf2u-negative-fixtures" && cargo xtask ai-release --bead vb-nf2u` failed non-zero with `Missing evidence for gate 'negative_fixture'`.
- Function-size scan: black-hat-cited functions are now <=25 lines. Full-file scan still reports pre-existing oversized `#[cfg(test)]` unit tests in `xtask/src/evidence.rs`, outside the cited State 11 blocker surface.

## Power-of-Ten / zero-panic rules affected
- Strengthened type-carried invariants for deterministic capture state and layout fixture field applicability.
- Removed neutral-default laundering on required negative fixture status.
- Preserved no production `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` in modified production code.

## Performance-layer decision
- No performance claim made. This was correctness/provenance/type-safety work; no benchmark/profiler evidence required.

## Second-ring evidence
- Kani inventory and layout proof commands passed.
- No assembly/IR/API/provenance performance claim made; no additional second-ring evidence required.

## Skipped gates
- None of the required verification commands were skipped.

## Residual risks
- Evidence remains deterministic fixture-backed only; no live Makepad rendering or core runtime parity is claimed.
- Lockbud remains bead-waived under the existing bead-scoped structured waiver.
- Pre-existing workspace warnings remain: duplicate Makepad `bitflags`, duplicate binary target names, and `vb_ui_model` no_std attribute warning.
