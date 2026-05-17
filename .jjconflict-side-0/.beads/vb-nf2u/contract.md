# Contract Specification: vb-nf2u UI overlap and redaction release gates

## Context
- Feature: add fail-closed UI release gates proving all eight release screens are reachable, readable, layout-safe, redacted, and captured with deterministic evidence.
- Authority: `velvet-ballistics-MASTER.md` is authoritative; generated docs, commands, and diagnostics must use canonical product spelling `velvet-ballastics` except legacy path references.
- Existing targets: `vb_ui_snapshot::REQUIRED_FIXTURES`, `vb_ui_makepad::shell::{ShellNav, Screen}`, `vb_ui_model::UiScreenKind`, `vb_ui_snapshot::checks`, `vb_ui_snapshot::report`, `cargo xtask ui-snapshot`, `cargo xtask ui-overlap-check`, and `cargo xtask ai-release --bead <id>`.
- Canonical screen IDs: `execution_overview`, `workflow_graph_authoring`, `execution_details`, `verification_certificate`, `replay_theater`, `incident_failure`, `action_registry`, `storage_doctor_ai_context`.

## Assumptions
- Real Makepad rendering may remain blocked by `blocked-by-core` / `ui-paused`; release gates may use deterministic UI model fixtures only if evidence explicitly says fixture-backed and makes no CLI/UI parity claim.
- Snapshot evidence format may stay YAML for the cold UI gate; this does not relax the master ban on JSON/YAML in runtime core.
- Secret scanning includes rendered text, snapshot metadata, YAML reports, diagnostics, and generated artifacts under the UI evidence directory.

## Open questions
- None for the release entrypoint: `cargo xtask ai-release --bead vb-nf2u` is the single required UI release entrypoint. UI snapshot, layout/readability, redaction, negative-fixture, deterministic-capture, and evidence-shape subgates must be traced through that command.
- If OCR is unavailable, scanner implementation must use deterministic fixture text/metadata plus generated evidence text; pixel OCR can be an enhancement but not sole proof.

## Preconditions
- PRE-001: Gate input names must be empty/`--all` or members of the canonical eight-screen inventory; unknown, duplicate, or partial inventories are rejected before capture.
- PRE-002: Every captured screen must have a deterministic `UiScreenKind -> ShellNav -> Screen -> fixture_id -> fixture_text_artifact_path` mapping before release evidence is accepted.
- PRE-003: Release capture must enter deterministic snapshot mode with a fixed snapshot timestamp and hidden-animation pause before any fixture-text report/artifact is produced.
- PRE-004: Redaction scanner input must include an explicit denylist of raw secret sentinels, API-key/token/password/idempotency-key examples, and tainted fixture values.

## Postconditions
- POST-001: A release UI gate succeeds only when the report contains exactly the canonical eight screen IDs, each with one readable `.fixture.txt` artifact path, one deterministic `blake3:` digest over read artifact bytes, and all required check kinds.
- POST-002: For every screen, overlap, clipping, bounds, chip readability, selected-state visibility, fixture-artifact provenance, and redaction checks execute and emit pass/fail evidence; missing checks are failures.
- POST-003: `cargo xtask ai-release --bead vb-nf2u` includes UI snapshot, layout/readability, redaction, negative-fixture, deterministic-capture, and evidence-shape gates; omitting any UI gate fails release.
- POST-004: Any detected clipped, overlapped, unreadable, hidden, out-of-bounds, or exposed-secret condition fails with an actionable diagnostic naming the screen and offending control/secret class.
- POST-005: Intentional negative overlap and secret-leak fixtures fail the correct gate; if a negative fixture passes, the release gate fails as a false-pass detector.
- POST-006: Repeated captures with the same checked-in fixture-text inputs and snapshot time produce identical normalized reports and stable `blake3:` fixture-text artifact digests.

## Invariants
- INV-001: The screen inventory is a bijection across `REQUIRED_FIXTURES`, `UiScreenKind`, `ShellNav::screen()`, `Screen`, and generated report screen IDs; no missing, duplicate, or extra screen is valid.
- INV-002: Layout checks are semantic, not placeholders: success is legal only after inspecting concrete control/label/chip/state bounds or their deterministic fixture representation.
- INV-003: Redaction is fail-closed: raw secrets must not appear in rendered text, fixture-visible labels, metadata, YAML evidence, diagnostics, or generated artifacts; approved placeholders must be non-secret and explicit.
- INV-004: Hidden animations are paused during release gates, visible animations use deterministic snapshot time, and no wall-clock/random value can affect evidence.
- INV-005: Evidence is deterministic and reviewable: each screen result records screen ID, fixture-text artifact path, digest, check list, pass/fail status, and diagnostics for every failure.
- INV-006: UI gates must not claim real core/runtime parity while `blocked-by-core` or synthetic snapshot capture remains in effect.

## Error Taxonomy
- `UiReleaseGateError::InvalidScreenInventory` - unknown, duplicate, missing, or extra screen/fixture/nav entries violate PRE-001 or INV-001.
- `UiReleaseGateError::UnreachableScreen` - a `ShellNav`, `Screen`, `UiScreenKind`, or fixture cannot map to exactly one release screen.
- `UiReleaseGateError::SnapshotDeterminismViolation` - snapshot time is not fixed, hidden animations are not paused, or repeated capture digests differ.
- `UiReleaseGateError::MissingEvidence` - required fixture-text artifact/report/digest/check/negative-fixture evidence is absent or unreadable.
- `UiReleaseGateError::LayoutViolation` - overlap, clipping, bounds, chip readability, or selected-state check reports a violation.
- `UiReleaseGateError::RedactionViolation` - raw secret or forbidden sentinel appears in any scanned UI artifact.
- `UiReleaseGateError::FalsePassFixtureViolation` - intentional overlap or secret-leak fixture does not fail the expected gate.
- `UiReleaseGateError::ReleaseProfileIncomplete` - `ai-release` does not include all required UI release gates.
- `UiReleaseGateError::CoreParityUnsupported` - evidence attempts to claim live CLI/runtime parity while only deterministic fixtures are available.

## Contract Signatures
- `fn canonical_ui_release_inventory() -> Result<UiReleaseInventory, UiReleaseGateError>`
- `fn validate_screen_bijection(inventory: &UiReleaseInventory) -> Result<(), UiReleaseGateError>`
- `fn enter_release_snapshot_mode(config: SnapshotDeterminismConfig) -> Result<ReleaseSnapshotGuard, UiReleaseGateError>`
- `fn capture_all_release_screens(config: UiReleaseGateConfig) -> Result<UiReleaseEvidence, UiReleaseGateError>`
- `fn check_layout_safety(screen: &ScreenEvidence) -> Result<LayoutCheckEvidence, UiReleaseGateError>`
- `fn check_redaction_artifacts(evidence: &UiReleaseEvidence, denylist: &SecretDenylist) -> Result<RedactionEvidence, UiReleaseGateError>`
- `fn run_ui_negative_fixtures(config: UiReleaseGateConfig) -> Result<NegativeFixtureEvidence, UiReleaseGateError>`
- `fn run_ui_release_subgates_for_ai_release(config: UiReleaseGateConfig) -> Result<UiReleaseEvidence, UiReleaseGateError>`
- `fn include_ui_gates_in_ai_release(bead_id: &str) -> Result<ReleaseProfileEvidence, UiReleaseGateError>`

## Lean-Owned Clauses
- None. This bead is a UI/release-gate shell with small bounded inventory and rectangle/scanner kernels better handled by Kani/proptest/unit tests. See `lean-contract.md` waivers for pure-kernel compensation.

## Non-goals
- No production code, test code, proof code, or harness code is specified here.
- No performance superiority claim is made.
- No real runtime/CLI parity claim is allowed until core/UI blockers are removed and live-render evidence exists.
