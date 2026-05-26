# vb-nf2u codebase map

## 1. Scope summary

Bead `vb-nf2u` adds UI release gates for overlap/readability and redaction evidence. The relevant code is split between Makepad-facing shell primitives in `crates/vb_ui_makepad`, deterministic snapshot fixture/report infrastructure in `crates/vb_ui_snapshot`, typed UI view models in `crates/vb_ui_model`, and `xtask` commands that currently expose `ui-snapshot`, `ui-overlap-check`, and generic `ai-release` profiles.

The release-gate target is not yet fully implemented: the eight-screen fixture inventory exists, snapshot PNGs exist, and overlap/check report types exist, but overlap/clipping/chip/selected-state checks are stubs, redaction scanning is absent, fixture-level negative cases are absent, and `ai-release` does not include UI-specific snapshot/overlap/redaction evidence.

## 2. Files/directories inspected with relevant roles

- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/crates/vb_ui_makepad/src/lib.rs` — exports UI shell, screen enum, graph canvas/node/edge, packet dot animation, tokens.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/crates/vb_ui_makepad/src/shell.rs` — defines eight `ShellNav` routes mapped to eight `Screen` variants and labels; this is current reachability encoding.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/crates/vb_ui_makepad/src/packet_dot.rs` — defines `AnimationTick`/packet motion; no release-gate snapshot freeze or hidden-screen pause API was found.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/crates/vb_ui_snapshot/src/lib.rs` — declares `REQUIRED_FIXTURES`, baseline 1920x1080 dimensions, and fixture-name enumeration.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/crates/vb_ui_snapshot/src/fixtures.rs` — builds deterministic in-memory fixtures for all eight screens; contains secret/taint indicators only as model data, including `secrets_redacted: true` in storage/AI context.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/crates/vb_ui_snapshot/src/checks.rs` — contains result types for overlap, clipping, bounds, chip readability, selected state, color drift, spelling, and PNG validity; key layout/readability checks currently return empty pass results.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/crates/vb_ui_snapshot/src/report.rs` — serializes `UiSnapshotReport` YAML with per-screen checks and pass/fail status.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/crates/vb_ui_model/src/lib.rs` — typed screen data model; includes `UiScreenKind`, `UiAppSnapshot`, `SlotDiffView` taint, `AiContextPanel`, and `AiContextView` redaction booleans.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/xtask/src/main.rs` — CLI entrypoints for `ui-snapshot`, `ui-tokens`, `ui-overlap-check`, and `ai-release`; snapshot generation currently creates synthetic PNGs rather than invoking real Makepad rendering.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/xtask/src/evidence.rs` — generic command-center evidence model and profile list; `run_gate`/`run_profile` are still red-phase stubs.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/xtask/src/gates.rs` — generic release gate command list; no UI snapshot/overlap/redaction release gates are listed.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/tests/ui_snapshots/` — existing generated PNG artifacts plus `ui_snapshot_report.yaml` for eight screens.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/tests/` — no bead-named UI release tests found; existing top-level tests are phase0/diagnostic/recovery focused.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/xtask/tests/integration_gates.rs` — generic `ai-fast`/`ai-deep`/`ai-release` evidence tests; no UI-specific release-gate assertions found.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/design/reference/` — reference board and eight reference screenshots.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/design/reference/figma_makepad_notes.md` — states eight 1920x1080 screens, fixed shell dimensions, no overlaps, and Makepad animation cue guidance.
- `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/velvet_ballistics_makepad_ui_master_plan_with_images/velvet-ballistics-MASTER-makepad-ui-update.md` — non-authoritative UI plan mirror with snapshot requirements and UI acceptance commands.

## 3. Existing UI snapshot/overlap/redaction commands, xtask entrypoints, tests, fixtures, generated artifacts

Existing commands/entrypoints:

- `cargo xtask ui-snapshot --all --emit yaml` in `xtask/src/main.rs` captures all `demo_fixture_names()` to `tests/ui_snapshots/*.png` and writes `tests/ui_snapshots/ui_snapshot_report.yaml`.
- `cargo xtask ui-snapshot --fixture <name> --emit yaml` captures one fixture.
- `cargo xtask ui-overlap-check --all` iterates all fixtures and calls `checks::check_overlap` on `tests/ui_snapshots/<name>.png`.
- `cargo xtask ui-overlap-check --screen <name>` checks one screen PNG.
- `cargo xtask ui-tokens --check` validates generated token output.
- `cargo xtask ai-release --bead <id>` exists, but current generic release profile only lists check/test/supply-chain/miri/fuzz/coverage/mutants/bench/feature/source/maxperf gates and does not list UI snapshot/overlap/redaction gates.

Existing tests:

- No acceptance tests named `test_all_eight_screens_pass_reachability_and_overlap_gates`, `test_secret_values_are_redacted_in_every_screen`, `test_intentional_overlap_fixture_fails_gate`, or `test_intentional_secret_fixture_fails_redaction_gate` were found.
- `xtask/tests/integration_gates.rs` tests generic evidence profile shape and `ai-release` aggregation, but evidence execution functions are red-phase stubs in `xtask/src/evidence.rs`.
- `crates/vb_ui_snapshot/src/report.rs` has a YAML serialization unit test.
- `crates/vb_ui_snapshot/src/tokens.rs` has token parse/generation tests.

Existing fixtures/artifacts:

- Fixture source is code-generated in `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/crates/vb_ui_snapshot/src/fixtures.rs`, not file-backed `fixtures/ui/*.fixture` files.
- Generated snapshot artifacts exist in `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/tests/ui_snapshots/` for all eight names plus `ui_snapshot_report.yaml`.
- Design reference artifacts exist under `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/design/reference/screenshots/` for screens `01_...` through `08_...` and `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/design/reference/white_makepad_8_screen_board.png`.

Current redaction support:

- Model fields `AiContextPanel::secrets_redacted` and `AiContextView::secrets_redacted` exist.
- `fixtures.rs` sets `secrets_redacted: true` in storage/AI-context fixture data.
- No `xtask` redaction command, `vb_ui_snapshot::checks::check_redaction`, redaction report kind, raw-secret denylist, OCR/text scan, or intentional secret leak fixture was found.

## 4. Eight-screen inventory and where reachability is encoded

Canonical fixture names in `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/crates/vb_ui_snapshot/src/lib.rs`:

1. `execution_overview`
2. `workflow_graph_authoring`
3. `execution_details`
4. `verification_certificate`
5. `replay_theater`
6. `incident_failure`
7. `action_registry`
8. `storage_doctor_ai_context`

Fixture construction in `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/crates/vb_ui_snapshot/src/fixtures.rs` maps these names to screen kinds:

- `execution_overview` -> `ExecutionOverview`
- `workflow_graph_authoring` -> `WorkflowGraphAuthoring`
- `execution_details` -> `ExecutionDetailsGraph`
- `verification_certificate` -> `VerificationCertificate`
- `replay_theater` -> `ReplayTheater`
- `incident_failure` -> `IncidentFailureConsole`
- `action_registry` -> `ActionRegistry`
- `storage_doctor_ai_context` -> `StorageDoctorAiContext`

Reachability is encoded in `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/crates/vb_ui_makepad/src/shell.rs`:

- `ShellNav::Overview.screen()` -> `Screen::ExecutionOverview`
- `ShellNav::WorkflowGraph.screen()` -> `Screen::WorkflowGraphAuthoring`
- `ShellNav::Executions.screen()` -> `Screen::ExecutionDetailsGraph`
- `ShellNav::Verification.screen()` -> `Screen::VerificationCertificate`
- `ShellNav::Replay.screen()` -> `Screen::ReplayTheater`
- `ShellNav::Incidents.screen()` -> `Screen::IncidentFailureConsole`
- `ShellNav::Actions.screen()` -> `Screen::ActionRegistry`
- `ShellNav::Storage.screen()` -> `Screen::StorageDoctorAiContext`

Additional typed inventory exists in `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/crates/vb_ui_model/src/lib.rs` as `UiScreenKind` discriminants `0..=7` for the same eight screen concepts.

## 5. Current gaps vs bead acceptance tests

- `test_all_eight_screens_pass_reachability_and_overlap_gates`: missing. Existing `ui-snapshot --all` iterates eight fixtures, but no test asserts Makepad shell reachability, all eight report entries, chip readability, selected-state visibility, bounds, and overlap/clipping. `check_overlap`, `check_clipping`, `check_chip_readability`, `check_bounds`, and `check_selected_state` currently return empty success values.
- `test_secret_values_are_redacted_in_every_screen`: missing. Redaction booleans exist for AI context, but there is no all-screen scan for raw secret markers, fixture secret inventories, UI report redaction check kind, or deterministic evidence that secrets are redacted in every screen.
- `test_intentional_overlap_fixture_fails_gate`: missing. No negative overlap fixture was found. Since `check_overlap` always returns no overlaps, an intentional bad fixture would currently pass.
- `test_intentional_secret_fixture_fails_redaction_gate`: missing. No negative secret fixture or redaction check exists, so raw secret exposure would not fail a UI release gate.
- Release-gate evidence: missing. Generic `ai-release` profile does not invoke `ui-snapshot`, `ui-overlap-check`, or redaction checks, and `xtask/src/evidence.rs` is red-phase stubbed.
- Deterministic snapshot time and hidden animation pause: missing in inspected code. `packet_dot.rs` has animation tick math, but no release/snapshot mode API to freeze time, set deterministic now, or pause hidden animations.

## 6. Risks/blockers and dependencies on core/P0/P1 beads

Actual blocker evidence found:

- `vb-nf2u` has labels `blocked-by-core` and `ui-paused` in `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/.beads/vb-nf2u/STATE.md` evidence from `bd show`.
- Generic command-center gates are not production-ready: `/home/lewis/src/Velvet-ballistics-vb-nf2u-go/xtask/src/evidence.rs` marks `run_gate`, `run_profile`, and `explain_failure` as `RED_PHASE` stubs, so integrating UI gates into `ai-release` depends on completing or bypassing that gate-evidence layer.
- Core/UI parity data may be incomplete for real screens: current snapshots are synthetic image blocks from `xtask/src/main.rs`, not proof that real Makepad screens render reachable/readable UI controls.

Risks:

- False pass risk is high because layout/readability functions currently pass unconditionally.
- Redaction false pass risk is high because only model booleans exist; no scanner rejects raw secret strings in rendered/snapshot artifacts.
- Snapshot determinism risk remains because no inspected API freezes Makepad time or pauses hidden animations for release capture.
- File-backed fixture mismatch: UI plan references `fixtures/ui/*.fixture`, but current implementation uses Rust fixture constructors in `vb_ui_snapshot::fixtures`.

## 7. Recommended State 3 contract focus: concrete contract clauses and verification layers to request

Contract clauses to request:

1. Eight-screen coverage: release UI gate must enumerate exactly the canonical eight screen IDs and fail closed on missing, duplicate, unreachable, or non-readable screens.
2. Shell reachability: every `ShellNav` route must map to a `Screen`, every `Screen` must map to a fixture/snapshot ID, and the report must prove all eight were visited.
3. Deterministic capture: release mode must freeze snapshot time, pause hidden animations, and make output stable across repeated runs for the same fixture/token input.
4. Layout safety: overlap, clipping, bounds, chip readability, and selected-state checks must be executable, evidence-producing, and fail on intentional bad fixtures.
5. Redaction safety: no raw secret sentinel/API key/token/password/idempotency key may appear in any rendered text, snapshot metadata, YAML evidence, or generated artifact; redacted placeholders are allowed only in approved form.
6. Evidence shape: release gate output must include per-screen evidence with screen ID, PNG path, checks run, pass/fail, actionable diagnostic, and deterministic artifact path.
7. Fail-closed negative fixtures: intentional overlap and intentional secret fixtures must fail the correct gate with diagnostics naming the offending control/secret and screen.
8. Core dependency boundary: if real core artifacts are not available, gates may use deterministic UI model fixtures, but must not claim CLI/UI parity with unfinished core outputs.

Verification layers to request:

- Unit tests in `vb_ui_snapshot` for overlap/clipping/bounds/chip/selected-state/redaction pure check functions, including negative fixtures.
- Integration tests through `cargo xtask ui-snapshot --all --emit yaml`, `cargo xtask ui-overlap-check --all`, and a new redaction/release UI gate command.
- Bead-named acceptance tests matching the four required names.
- Snapshot determinism test that runs capture twice into separate temp dirs and compares normalized report plus image bytes or stable digests.
- Fixture inventory test that ties `REQUIRED_FIXTURES`, `UiScreenKind`, `ShellNav`, and `Screen` together without omissions.
- Mutation target: invert redaction/overlap result booleans and ensure acceptance tests fail.
- No Lean required unless State 3 defines a pure screen-inventory bijection model; Kani/Proptest can cover bounded rectangle intersection and redaction scanner invariants more cheaply.
