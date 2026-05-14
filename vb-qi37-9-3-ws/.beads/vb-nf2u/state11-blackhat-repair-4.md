STATUS: PASS

# State 11 Black-Hat Blocker Repair 4 — vb-nf2u

## Files changed
- `xtask/src/evidence.rs`
- `crates/vb_ui_snapshot/src/checks.rs`
- `crates/vb_ui_snapshot/src/layout_kernel.rs`
- `crates/vb_ui_snapshot/kani/layout_predicates.rs`
- `crates/vb_ui_snapshot/tests/layout_checks.rs`
- `tests/vb_nf2u_ui_release_acceptance.rs`
- `.beads/vb-nf2u/state11-blackhat-repair-4.md`

## Blocker-by-blocker repair summary
1. Positive `ai-release` evidence now builds screen/check/subgate rows through provenance-bearing outcome constructors. Check rows evaluate deterministic screen facts, stable digest metadata, generated redaction artifact text, negative fixture input state, deterministic timestamp/animation metadata, and evidence-shape validators.
2. Negative fixture acceptance assertions now parse `negative-fixtures.txt` into typed test-domain entries and assert exact fields instead of raw substring smoke checks.
3. Split required reviewed functions under 25 lines: `UiReleaseBundle::fixture_backed`, `append_screen_snapshot`, `LayoutFixture::parse`, and `extract_words_from_image`. New helper functions were kept small.
4. Release rows are no longer built by public/direct all-green literals in the repaired UI evidence surface; `UiSubgateRun` and `UiCheckEvidenceRow` are created from validation outcomes and provenance.
5. Layout domain state now uses typed `LayoutKernelResult`, `LayoutKernelError`, `SelectedIndicator`, `FixtureFieldNeed`, and `SelectionVisibility`; `Rect::new` returns a typed result and Kani no longer launders invalid rectangles through `Rect::unit`.
6. Removed the black-hat cited `unwrap_or` / `unwrap_or_else(Rect::unit)` defaults from `checks.rs`, `xtask/src/evidence.rs`, and Kani layout harnesses. Missing negative fixtures now remain fail-closed.

## Command results
- PASS: `cargo nextest run -p velvet-ballastics-workspace --test vb_nf2u_ui_release_acceptance` — 8 passed, 0 skipped.
- PASS: `cargo nextest run -p vb_ui_snapshot -p xtask` — 130 passed, 0 skipped.
- PASS: `cargo kani -p vb_ui_snapshot --harness inventory` — 1 harness verified, 0 failures; output `/home/lewis/.local/share/opencode/tool-output/tool_e10f3b508001iMh7MFD0MK0WZM`.
- PASS: `cargo kani -p vb_ui_snapshot --harness layout_` — 5 harnesses verified, 0 failures; output `/home/lewis/.local/share/opencode/tool-output/tool_e10f3c7b1001jyxcOd4VGd14Xx`.
- PASS: `moon run :verify-all` — `Tasks: 1 completed`; output `/home/lewis/.local/share/opencode/tool-output/tool_e10f62721002kKRYSPRl6xhCE8`.
- PASS: `moon ci --base HEAD --head HEAD` — `Tasks: 20 completed (2 cached)`; output `/home/lewis/.local/share/opencode/tool-output/tool_e10f77159001VkTHdn527DpNrr`.
- PASS: `rtk cargo fmt --all --check`.
- PASS: `rtk cargo clippy -p vb_ui_snapshot -p xtask --tests --all-features -- -D warnings` — 0 errors, wrapper reported 2 existing warnings.
- PASS expected-fail: `rm -rf "target/vb-nf2u-negative-fixtures" && cargo xtask ai-release --bead vb-nf2u` failed non-zero with `Missing evidence for gate 'negative_fixture'`.

## Power-of-Ten / zero-panic rules affected
- Function size rule tightened on reviewed hot/release-gate helpers.
- Panic-vector-adjacent defaults removed from the reviewed layout/release surfaces.
- Option-based arithmetic/control state replaced at layout boundaries with typed results/enums.
- No production `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` introduced.

## Performance-layer decision
- No performance claim made. This repair was correctness/provenance/type-safety work; no benchmark/profiler evidence required.

## Second-ring evidence
- Kani inventory and layout proof commands passed.
- No assembly/IR/API/provenance performance claim made; no additional second-ring evidence required.

## Skipped gates
- None of the user-requested gates were skipped.

## Residual risks
- Evidence remains deterministic fixture-backed only; no live Makepad rendering or core runtime parity is claimed.
- Lockbud remains bead-waived under the existing bead-scoped structured waiver.
- Pre-existing workspace warnings about duplicate Makepad `bitflags`, duplicate binary target names, and a `vb_ui_model` no_std attribute warning remain outside this repair.
