# Test Plan: vb-nf2u UI overlap and redaction release gates

## Summary
- Contract review input: `.beads/vb-nf2u/contract-verification-review.md` is approved; `.beads/vb-nf2u/test-plan-review.md` rejected the prior plan and every required repair is incorporated here.
- Behaviors identified: 36 public/release behaviors plus 15 existing `UiSnapshotError` reachability behaviors.
- Trophy allocation target: 45 unit tests / 24 integration tests / 4 e2e-style acceptance tests / 9 static-formal command evidence checks.
- Unit density rule: 9 contract signatures × 5 unit/boundary tests each = 45 planned unit tests. No signature is reclassified away from the density floor.
- Required bead-named acceptance boundary: all four bead-named tests execute `cargo xtask ai-release --bead vb-nf2u`; helpers may only create temp dirs, configure fixtures, and inspect emitted evidence.
- Proptest invariants: 9. Fuzz/Bolero targets: 3. Kani harnesses: 7. Mutation threshold: `>=90%` killed mutants overall and `100%` kill for fail-open UI gate mutations.
- No planned assertion may be `is_ok()` or `is_err()`; every scenario below asserts an exact success value, exact artifact shape, or exact error variant with exact diagnostic fields.

## Intended test files, commands, fixtures, and evidence

### Test files to create or extend
- `tests/vb_nf2u_ui_release_acceptance.rs`
  - `test_all_eight_screens_pass_reachability_and_overlap_gates`
  - `test_secret_values_are_redacted_in_every_screen`
  - `test_intentional_overlap_fixture_fails_gate`
  - `test_intentional_secret_fixture_fails_redaction_gate`
- `crates/vb_ui_snapshot/tests/inventory_bijection.rs`
- `crates/vb_ui_snapshot/tests/layout_checks.rs`
- `crates/vb_ui_snapshot/tests/redaction_checks.rs`
- `crates/vb_ui_snapshot/tests/report_evidence_shape.rs`
- `crates/vb_ui_makepad/tests/shell_reachability.rs`
- `xtask/tests/ui_release_gates.rs`
- `xtask/tests/ui_release_errors.rs`
- `xtask/tests/ui_release_tooling_red_phase.rs`
- `fuzz/fuzz_targets/ui_redaction_artifact.rs` and Bolero equivalent if Bolero is chosen by the workspace.
- `crates/vb_ui_snapshot/kani/inventory.rs` or equivalent Kani-gated module.
- `crates/vb_ui_snapshot/kani/layout_predicates.rs` or equivalent Kani-gated module.

### Canonical fixtures and evidence paths
- Canonical screen IDs, exactly: `execution_overview`, `workflow_graph_authoring`, `execution_details`, `verification_certificate`, `replay_theater`, `incident_failure`, `action_registry`, `storage_doctor_ai_context`.
- State 11 repair 8 contract parity note: while real Makepad rendering remains out of scope, positive release evidence is fixture-backed text evidence. Screen artifacts are checked-in source fixture inputs copied to `.fixture.txt` outputs and validated by `blake3:` digest/readback, not PNG capture evidence.
- Required negative fixtures: `intentional_overlap_fixture`, `intentional_secret_fixture`, plus false-pass harness variants that force the bad fixture to report `passed`.
- Evidence directory: `.evidence/vb-nf2u/`.
- Required evidence files: `.evidence/vb-nf2u/ai-release.yaml`, `.evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml`, `.evidence/vb-nf2u/ui-layout-report.yaml`, `.evidence/vb-nf2u/negative-fixtures.txt`, `.evidence/vb-nf2u/determinism.txt`, `.evidence/vb-nf2u/animation-freeze.txt`, `.evidence/vb-nf2u/nextest-ui-release.txt`, `.evidence/vb-nf2u/proptest-ui.txt`, `.evidence/vb-nf2u/kani-ui.txt`, `.evidence/vb-nf2u/kani-layout.txt`, `.evidence/vb-nf2u/fuzz-redaction.txt`, `.evidence/vb-nf2u/bolero-redaction.txt` when Bolero is used, `.evidence/vb-nf2u/miri-ui-snapshot.txt`, `.evidence/vb-nf2u/mutants-ui.txt`, `.evidence/vb-nf2u/lcov.info`, `.evidence/vb-nf2u/moon-ci.txt`, `formal-verification-report.md`.

### Required commands
- `cargo nextest run -p vb_ui_snapshot -p vb_ui_makepad -p xtask`.
- `cargo xtask ui-snapshot --all --emit yaml --output-dir .evidence/vb-nf2u/ui_snapshots`.
- `cargo xtask ui-overlap-check --all --input-dir .evidence/vb-nf2u/ui_snapshots`.
- `cargo xtask ai-release --bead vb-nf2u`.
- `cargo nextest run -p vb_ui_snapshot inventory`.
- `cargo nextest run -p vb_ui_snapshot layout_proptest`.
- `cargo nextest run -p vb_ui_snapshot redaction_denylist`.
- `cargo nextest run -p xtask deterministic_snapshot_mode`.
- `cargo nextest run -p xtask negative_ui_release_fixtures`.
- `cargo kani -p vb_ui_snapshot --harness inventory`.
- `cargo kani -p vb_ui_snapshot --harness layout_`.
- `cargo fuzz run ui_redaction_artifact -- -runs=10000`.
- `cargo bolero test ui_redaction_artifact` when Bolero is the configured fuzz/property engine.
- `cargo +nightly miri test -p vb_ui_snapshot`.
- `cargo mutants -p vb_ui_snapshot --in-place --timeout 120`.
- `cargo llvm-cov nextest --package vb_ui_snapshot --package xtask --lcov --output-path .evidence/vb-nf2u/lcov.info`.
- `moon ci`; `moon run :verify-fast`; `moon run :verify-standard`; `moon run :verify-deep`; `moon run :verify-proof`; `moon run :verify-all`.

## Red-phase expectations for missing harness/tool wiring

Command absence is a failing red-phase signal, never passing evidence.

| Boundary | Missing-command red phase | Required final evidence |
|---|---|---|
| Kani inventory | `cargo kani -p vb_ui_snapshot --harness inventory` exits non-zero with missing package/harness/subcommand; this fails `kani_inventory_lane_exists_and_runs` | `.evidence/vb-nf2u/kani-ui.txt` contains successful inventory proof summary |
| Kani layout | `cargo kani -p vb_ui_snapshot --harness layout_` exits non-zero with missing harness; this fails `kani_layout_lane_exists_and_runs` | `.evidence/vb-nf2u/kani-layout.txt` contains overlap/clipping/bounds/chip/selected proof names |
| cargo-fuzz | `cargo fuzz run ui_redaction_artifact -- -runs=10000` exits non-zero with missing target/tool; this fails `cargo_fuzz_redaction_lane_exists_and_runs` | `.evidence/vb-nf2u/fuzz-redaction.txt` records target, seed count, and no crashes |
| Bolero | `cargo bolero test ui_redaction_artifact` exits non-zero with missing target/tool when Bolero is declared; this fails `bolero_redaction_lane_exists_and_runs` | `.evidence/vb-nf2u/bolero-redaction.txt` records equivalent redaction corpus execution |
| Miri | `cargo +nightly miri test -p vb_ui_snapshot` exits non-zero with missing component or unsupported test target; this fails `miri_ui_snapshot_lane_exists_and_runs` | `.evidence/vb-nf2u/miri-ui-snapshot.txt` records successful Miri run |
| Mutants | `cargo mutants -p vb_ui_snapshot --in-place --timeout 120` exits non-zero with missing tool/config; this fails `cargo_mutants_ui_lane_exists_and_runs` | `.evidence/vb-nf2u/mutants-ui.txt` reports `>=90%` killed and `100%` critical kills |
| llvm-cov | `cargo llvm-cov nextest ...` exits non-zero with missing tool/config/output; this fails `llvm_cov_ui_lane_exists_and_runs` | `.evidence/vb-nf2u/lcov.info` exists, is non-empty, and includes `vb_ui_snapshot` and `xtask` files |
| Moon CI | `moon ci` exits non-zero with missing task/project; this fails `moon_ci_lane_exists_and_runs` | `.evidence/vb-nf2u/moon-ci.txt` records successful `moon ci` |
| Moon verify lanes | each `moon run :verify-*` exits non-zero if absent; this fails `moon_verify_lanes_exist_and_run` | `formal-verification-report.md` names all five successful lanes |

## 1. Behavior Inventory
1. UI release inventory accepts exactly the eight canonical screen IDs when no screen is missing, duplicated, extra, or unknown.
2. UI release inventory rejects before capture when any screen ID is missing.
3. UI release inventory rejects before capture when any screen ID is duplicated.
4. UI release inventory rejects before capture when any screen ID is extra or unknown.
5. Shell reachability maps every `ShellNav` route to exactly one `Screen`, `UiScreenKind`, fixture ID, and report screen ID.
6. Shell reachability rejects mapping gaps, duplicates, and dangling edges.
7. Snapshot mode fixes timestamp and pauses hidden animations before producing any release artifact.
8. Snapshot mode rejects wall-clock time, unpaused hidden animations, and digest drift with `SnapshotDeterminismViolation`.
9. UI snapshot capture emits exactly eight fresh `.fixture.txt` artifacts and one fresh YAML report when `ui-snapshot --all --emit yaml` runs in fixture-backed mode.
10. UI snapshot evidence records readable fixture-text artifact path, deterministic `blake3:` digest, check execution markers, pass/fail status, diagnostics, and tempdir provenance for every screen.
11. Layout gate detects overlapping controls and returns `LayoutViolation` when rectangles intersect beyond policy.
12. Layout gate detects clipped labels and returns `LayoutViolation` when label bounds exceed their container.
13. Layout gate detects out-of-bounds controls and returns `LayoutViolation` when controls exceed viewport/shell bounds.
14. Layout gate detects unreadable chips and returns `LayoutViolation` when chip visible area or contrast is below threshold.
15. Layout gate detects hidden selected state and returns `LayoutViolation` when selected indicator is absent, hidden, or zero-area.
16. Positive layout gate passes all eight canonical fixtures only after executing overlap, clipping, bounds, chip readability, selected-state, fixture-artifact provenance, and redaction checks with input-derived measurements.
17. Redaction denylist includes raw secret sentinels, API keys, tokens, passwords, idempotency keys, and tainted fixture values.
18. Redaction scanner rejects raw forbidden secret classes in rendered text, fixture-visible labels, metadata, YAML reports, diagnostics, and generated artifacts.
19. Redaction scanner allows only exact approved placeholder forms when secrets are intentionally represented.
20. Redaction diagnostics never echo the raw secret bytes.
21. Intentional overlap fixture fails the layout gate through `cargo xtask ai-release --bead vb-nf2u`.
22. Intentional secret fixture fails the redaction gate through `cargo xtask ai-release --bead vb-nf2u`.
23. False-pass detector fails release with `FalsePassFixtureViolation` when an intentional overlap fixture unexpectedly passes.
24. False-pass detector fails release with `FalsePassFixtureViolation` when an intentional secret fixture unexpectedly passes.
25. `cargo xtask ai-release --bead vb-nf2u` includes UI snapshot, layout/readability, redaction, negative-fixture, deterministic-capture, and evidence-shape subgates.
26. Release profile rejects omission of UI snapshot gate with `ReleaseProfileIncomplete`.
27. Release profile rejects omission of overlap/layout gate with `ReleaseProfileIncomplete`.
28. Release profile rejects omission of redaction gate with `ReleaseProfileIncomplete`.
29. Release profile rejects omission of negative-fixture gate with `ReleaseProfileIncomplete`.
30. Release profile rejects omission of determinism gate with `ReleaseProfileIncomplete`.
31. Release profile rejects omission of evidence-shape gate with `ReleaseProfileIncomplete`.
32. Repeated captures produce identical normalized reports and stable fixture-text artifact digests when inputs and snapshot time are identical.
33. Missing/unreadable fixture-text artifact, report, digest, check evidence, or negative-fixture evidence returns `MissingEvidence` with artifact path and evidence kind.
34. Fixture-backed or synthetic evidence rejects live core/runtime parity claims with `CoreParityUnsupported` while `blocked-by-core`/`ui-paused` applies.
35. Required formal/static tooling lanes fail red phase when missing and cannot be counted as release evidence.
36. Existing `UiSnapshotError` variants remain specifically reachable through public snapshot/check/report/token/image/io operations.

## 2. Trophy Allocation
| Behaviors | Layer | Tool/file | Rationale |
|---|---|---|---|
| 1-6 | Unit + proptest + Kani + integration | `inventory_bijection.rs`, `shell_reachability.rs`, Kani inventory | Finite mapping has pure kernels plus cross-crate reachability. |
| 7-10, 32 | Integration + Miri + static scan | `xtask/tests/ui_release_gates.rs`, Miri | Determinism and artifact freshness are observable through real capture and filesystem evidence. |
| 11-16, 21, 23 | Unit + proptest + Kani + integration + acceptance | `layout_checks.rs`, `ai-release` | Predicate kernels need boundary density; gate behavior must execute through release command. |
| 17-20, 22, 24 | Unit + proptest + fuzz/Bolero + integration + acceptance | `redaction_checks.rs`, fuzz targets, `ai-release` | Scanner must handle hostile text and release aggregation. |
| 25-31 | Integration + e2e-style CLI acceptance | `tests/vb_nf2u_ui_release_acceptance.rs`, `xtask/tests/ui_release_gates.rs` | `ai-release` is the single public release boundary. |
| 33-34 | Integration + static scan | `xtask/tests/ui_release_errors.rs`, `moon ci` | Evidence and overclaim failures depend on real artifacts and release text. |
| 35 | Static-formal command evidence | `ui_release_tooling_red_phase.rs`, Moon/Kani/fuzz/Miri/mutants/llvm-cov | Missing command/harness is a separate failure mode. |
| 36 | Unit/integration | `crates/vb_ui_snapshot/tests/*` | Every existing error variant remains reachable with exact fields. |

## 2.1 Public/unit classification and 45-test density table
All 9 contract signatures are treated as public contract functions for planning. Each has at least five planned unit/boundary tests; integration tests are additional and do not reduce unit density.

| Contract signature | Public? | Unit tests planned | Required unit/boundary cases |
|---|---:|---:|---|
| `canonical_ui_release_inventory()` | Yes | 5 | exact eight; empty input/config rejected as missing; one missing; duplicate; extra/unknown |
| `validate_screen_bijection(&UiReleaseInventory)` | Yes | 5 | valid bijection; missing mapping edge; duplicated mapping edge; dangling fixture/report edge; mismatched `ShellNav -> Screen` |
| `enter_release_snapshot_mode(SnapshotDeterminismConfig)` | Yes | 5 | fixed timestamp + pause succeeds; wall-clock config returns `SnapshotDeterminismViolation`; hidden animation unpaused returns `SnapshotDeterminismViolation`; zero/None timestamp returns `SnapshotDeterminismViolation`; guard evidence records deterministic marker |
| `capture_all_release_screens(UiReleaseGateConfig)` | Yes | 5 | eight fresh `.fixture.txt` artifacts; missing fixture returns `MissingEvidence`; unreadable output path returns `MissingEvidence`; stale checked-in artifact rejected by provenance; digest drift returns `SnapshotDeterminismViolation` |
| `check_layout_safety(&ScreenEvidence)` | Yes | 5 | overlap; clipped label; out-of-bounds control; unreadable chip; hidden selected state, each returning `LayoutViolation` with predicate-specific fields |
| `check_redaction_artifacts(&UiReleaseEvidence, &SecretDenylist)` | Yes | 5 | sentinel; API key/token; password/idempotency key; tainted fixture value; raw secret in diagnostics/YAML, each returning `RedactionViolation` with no raw echo |
| `run_ui_negative_fixtures(UiReleaseGateConfig)` | Yes | 5 | overlap fixture expected fail; secret fixture expected fail; overlap false pass returns `FalsePassFixtureViolation`; secret false pass returns `FalsePassFixtureViolation`; missing negative evidence returns `MissingEvidence` |
| `run_ui_release_subgates_for_ai_release(UiReleaseGateConfig)` | Yes | 5 | all six subgates included; UI snapshot omitted; layout omitted; redaction omitted; negative/determinism/evidence-shape omitted, each omission exact |
| `include_ui_gates_in_ai_release(&str)` | Yes | 5 | bead `vb-nf2u` includes all gates; wrong bead id rejected; missing profile returns `ReleaseProfileIncomplete`; fixture parity overclaim returns `CoreParityUnsupported`; evidence file names and commands exactly match contract |
| **Total** |  | **45** | Satisfies 9 × 5 density floor. |

## 3. BDD Scenarios

### Behavior: all eight screens pass reachability and overlap gates
Test function name: `test_all_eight_screens_pass_reachability_and_overlap_gates`
Given: a temp evidence root with canonical fixture inputs for all eight screens.
When: the test runs `cargo xtask ai-release --bead vb-nf2u`.
Then: `.evidence/vb-nf2u/ai-release.yaml` lists exactly the eight canonical screen IDs, each maps through `ShellNav -> Screen -> UiScreenKind -> fixture_id -> report_id`, and each screen has executed checks `Overlap`, `Clipping`, `Bounds`, `ChipReadability`, `SelectedState`, `FixtureArtifactProvenance`, and `Redaction` with `passed: true`, empty diagnostics, and non-empty execution markers.
And: helper code may only prepare the temp fixture/evidence root; it must not call layout or snapshot internals instead of the `ai-release` command.

### Behavior: secrets are redacted in every screen
Test function name: `test_secret_values_are_redacted_in_every_screen`
Given: canonical fixtures contain sentinel, API key, token, password, idempotency key, and tainted fixture values.
When: the test runs `cargo xtask ai-release --bead vb-nf2u`.
Then: no raw denied value appears in any fixture-text artifact, fixture-visible text, YAML evidence, diagnostic, generated artifact, or `ui_snapshot_report.yaml` row.
And: every intentional representation equals the approved placeholder for its class: `[REDACTED:sentinel]`, `[REDACTED:api_key]`, `[REDACTED:token]`, `[REDACTED:password]`, `[REDACTED:idempotency_key]`, and `[REDACTED:tainted_fixture_value]`.
And: `ai-release.yaml` records a passing redaction subgate for each of the eight canonical screens with subgate evidence shaped exactly as `redaction: { status: "passed", checked_artifacts: ["fixture_text_artifact", "ui_snapshot_report", "diagnostics", "generated_artifacts"], class_coverage: { sentinel: { detectors: 1, raw_matches: 0, approved_placeholders_seen: 1 }, api_key: { detectors: 1, raw_matches: 0, approved_placeholders_seen: 1 }, token: { detectors: 1, raw_matches: 0, approved_placeholders_seen: 1 }, password: { detectors: 1, raw_matches: 0, approved_placeholders_seen: 1 }, idempotency_key: { detectors: 1, raw_matches: 0, approved_placeholders_seen: 1 }, tainted_fixture_value: { detectors: 1, raw_matches: 0, approved_placeholders_seen: 1 } }, diagnostics: [] }`.

### Behavior: intentional overlap fixture fails gate
Test function name: `test_intentional_overlap_fixture_fails_gate`
Given: the helper prepares `intentional_overlap_fixture` with control `run_button` at rectangle `{ x: 10, y: 10, width: 100, height: 60 }` and control `stop_button` at rectangle `{ x: 80, y: 40, width: 50, height: 50 }`.
When: the test runs `cargo xtask ai-release --bead vb-nf2u`.
Then: release evidence records the negative fixture as expected-failed under the layout subgate with diagnostic code `layout_violation`, fields `screen_id: "execution_overview"`, `fixture_id: "intentional_overlap_fixture"`, `control_id: "run_button"`, `second_control_id: "stop_button"`, `predicate: "overlap"`, `overlap_area_px: 600`, `bounds`, and `action`; the acceptance test passes only if the command boundary observes this expected failure.

### Behavior: intentional secret fixture fails redaction gate
Test function name: `test_intentional_secret_fixture_fails_redaction_gate`
Given: the helper prepares `intentional_secret_fixture` containing raw denied values in fixture text, metadata, report text, and diagnostic-like text.
When: the test runs `cargo xtask ai-release --bead vb-nf2u`.
Then: release evidence records the negative fixture as expected-failed under the redaction subgate with diagnostic code `redaction_violation`, fields `screen_id`, `fixture_id`, `artifact_path`, `secret_class`, `redacted_sample`, and `action`; the raw secret bytes are absent from diagnostics and evidence.

### Behavior: inventory accepts exactly canonical eight
Test function name: `inventory_returns_canonical_eight_when_all_required_screens_present`
Given: inventory inputs equal the canonical eight screen IDs in canonical order.
When: `canonical_ui_release_inventory()` and `validate_screen_bijection()` run.
Then: the output screen IDs equal exactly `[execution_overview, workflow_graph_authoring, execution_details, verification_certificate, replay_theater, incident_failure, action_registry, storage_doctor_ai_context]` and validation returns exact success `()` with cardinality `8` and no duplicate report IDs.

### Behavior: inventory rejects bad screen sets
Test function name: `invalid_screen_inventory_error_returns_typed_variant_and_diagnostic`
Given: an inventory with one missing, duplicated, extra, partial, or unknown screen ID.
When: inventory validation runs before capture.
Then: `Err(UiReleaseGateError::InvalidScreenInventory { code: "invalid_screen_inventory", screen_id_or_count, reason, action })`, where `screen_id_or_count` names the offending ID for duplicate/unknown/extra and the missing count for missing/partial.

### Behavior: unreachable mapping rejects release
Test function name: `unreachable_screen_error_returns_typed_variant_and_diagnostic`
Given: a fixture/nav/screen/kind mapping omits or duplicates one canonical edge.
When: `validate_screen_bijection()` runs.
Then: `Err(UiReleaseGateError::UnreachableScreen { code: "unreachable_screen", screen_id, mapping_edge, action })` where `mapping_edge` is one of `ShellNav`, `Screen`, `UiScreenKind`, `fixture_id`, or `report_id`.

### Behavior: snapshot mode fixes time and pauses hidden animations
Test function name: `deterministic_snapshot_mode_fixes_time_and_pauses_hidden_animations_before_capture`
Given: a release snapshot config with fixed timestamp `2026-05-09T00:00:00Z` and hidden-animation pause enabled.
When: `enter_release_snapshot_mode(config)` runs before capture.
Then: the guard evidence records `snapshot_timestamp: 2026-05-09T00:00:00Z`, `hidden_animations_paused: true`, `wall_clock_used: false`, and no random nonce field in normalized evidence.

### Behavior: wall-clock snapshot time fails closed
Test function name: `snapshot_determinism_rejects_wall_clock_time_with_exact_diagnostic`
Given: screen `execution_overview` is captured with `snapshot_timestamp_source: "wall_clock"` and fixed release timestamp requirement `2026-05-09T00:00:00Z`.
When: deterministic snapshot validation runs.
Then: `Err(UiReleaseGateError::SnapshotDeterminismViolation { code: "snapshot_determinism_violation", screen_id: "execution_overview", expected_field: "snapshot_timestamp", expected_value: "2026-05-09T00:00:00Z", actual_field: "snapshot_timestamp_source", actual_value: "wall_clock", action: "set fixed snapshot timestamp before capture" })`.

### Behavior: unpaused hidden animation fails closed
Test function name: `snapshot_determinism_rejects_unpaused_hidden_animation_with_exact_diagnostic`
Given: screen `workflow_graph_authoring` is captured with deterministic timestamp `2026-05-09T00:00:00Z` and hidden animation `workflow_edge_pulse` reports `paused: false`.
When: deterministic snapshot validation runs.
Then: `Err(UiReleaseGateError::SnapshotDeterminismViolation { code: "snapshot_determinism_violation", screen_id: "workflow_graph_authoring", expected_field: "hidden_animation.workflow_edge_pulse.paused", expected_value: "true", actual_field: "hidden_animation.workflow_edge_pulse.paused", actual_value: "false", action: "pause hidden animations before capture" })`.

### Behavior: repeated capture digest drift fails closed
Test function name: `snapshot_determinism_rejects_digest_drift_with_exact_diagnostic`
Given: screen `execution_overview` is captured twice with fixed timestamp `2026-05-09T00:00:00Z`; first normalized digest is a `blake3:` digest over the checked-in fixture-text bytes; second normalized digest differs after the fixture-text bytes are changed.
When: deterministic snapshot validation runs.
Then: `Err(UiReleaseGateError::SnapshotDeterminismViolation { code: "snapshot_determinism_violation", screen_id: "execution_overview", expected_field: "normalized_fixture_text_digest", expected_value: "blake3:<expected>", actual_field: "normalized_fixture_text_digest", actual_value: "blake3:<actual>", action: "remove nondeterministic fixture inputs before release" })`.

### Behavior: repeated capture is stable
Test function name: `repeated_ui_snapshot_is_stable`
Given: two isolated temp output directories and identical fixture inputs with fixed snapshot timestamp.
When: two captures run through the release subgate.
Then: normalized YAML reports are byte-identical; all eight fixture-text artifact digests match exactly; normalized reports remove tempdir prefixes but preserve relative artifact names and digest values.

### Behavior: UI snapshot emits complete fresh fixture-text artifacts
Test function name: `ui_snapshot_all_emits_eight_fixture_text_artifacts_and_complete_yaml_report`
Given: a new temp output directory with no pre-existing `.fixture.txt` or YAML files.
When: `cargo xtask ui-snapshot --all --emit yaml --output-dir <tempdir>` runs.
Then: exactly eight `.fixture.txt` files named after canonical fixtures exist, `ui_snapshot_report.yaml` exists, `total_screens == 8`, `passed_screens == 8`, every file modified time is after command start and before command finish, every digest in YAML equals the digest of the just-created fixture-text artifact, and no artifact path points outside `<tempdir>`.

### Behavior: release evidence rejects stale snapshot artifacts
Test function name: `ui_snapshot_evidence_rejects_stale_checked_in_artifacts`
Given: a report references fixture-text artifact files whose modification time predates command start or whose provenance root is not the current tempdir.
When: evidence validation runs.
Then: `Err(UiReleaseGateError::MissingEvidence { code: "missing_evidence", screen_id, artifact_path, evidence_kind: "fresh_snapshot_artifact", action })`.

### Behavior: layout violation reports exact fields
Test function name: `layout_violation_error_returns_typed_variant_and_diagnostic`
Given: one fixture each for overlap, clipping, out-of-bounds, unreadable chip, and hidden selected state.
When: `check_layout_safety(&ScreenEvidence)` runs for each fixture.
Then: each invalid fixture returns `Err(UiReleaseGateError::LayoutViolation { code: "layout_violation", screen_id, control_id, predicate, bounds, action })`; `predicate` is exactly `overlap`, `clipping`, `bounds`, `chip_readability`, or `selected_state` for its fixture.

### Behavior: redaction denylist covers all required classes
Test function name: `redaction_denylist_includes_raw_secret_sentinels_api_keys_tokens_passwords_idempotency_keys_and_tainted_fixture_values`
Given: the scanner denylist is built for release mode.
When: the denylist is enumerated.
Then: the release-required class set equals exactly `{sentinel, api_key, token, password, idempotency_key, tainted_fixture_value}` and includes one concrete detector per class; if a class is missing, `check_redaction_artifacts` returns `Err(UiReleaseGateError::RedactionViolation { code: "redaction_violation", screen_id, artifact_path, secret_class, redacted_sample, action })` for an artifact containing that class.

### Behavior: redaction violation never echoes raw secret
Test function name: `redaction_violation_error_returns_typed_variant_and_diagnostic_without_echoing_secret`
Given: an artifact contains `sk_test_vb_nf2u_raw_secret`, `password=hunter2`, and `Idempotency-Key: idem_vb_nf2u_secret`.
When: `check_redaction_artifacts(&evidence, &denylist)` runs.
Then: it returns `Err(UiReleaseGateError::RedactionViolation { code: "redaction_violation", screen_id, artifact_path, secret_class, redacted_sample, action })`; `redacted_sample` equals `[REDACTED:<secret_class>]`; none of the raw secret substrings occur in the error display, debug text, YAML evidence, or diagnostic fields.

### Behavior: overlap false-pass fixture is detected
Test function name: `false_pass_fixture_rejects_overlap_fixture_that_reports_passed`
Given: the harness forces fixture `intentional_overlap_fixture` to report `actual_status: "passed"` while its required expected gate is `layout`.
When: `run_ui_negative_fixtures(config)` runs via the release subgate.
Then: `Err(UiReleaseGateError::FalsePassFixtureViolation { code: "false_pass_fixture_violation", fixture_id: "intentional_overlap_fixture", expected_gate: "layout", actual_status: "passed", action: "fail release because expected-fail negative fixture passed" })`.

### Behavior: secret false-pass fixture is detected
Test function name: `false_pass_fixture_rejects_secret_fixture_that_reports_passed`
Given: the harness forces fixture `intentional_secret_fixture` to report `actual_status: "passed"` while its required expected gate is `redaction`.
When: `run_ui_negative_fixtures(config)` runs via the release subgate.
Then: `Err(UiReleaseGateError::FalsePassFixtureViolation { code: "false_pass_fixture_violation", fixture_id: "intentional_secret_fixture", expected_gate: "redaction", actual_status: "passed", action: "fail release because expected-fail negative fixture passed" })`.

### Behavior: ai-release includes every UI subgate
Test function name: `ai_release_includes_ui_release_gates`
Given: the release profile for bead `vb-nf2u`.
When: `cargo xtask ai-release --bead vb-nf2u` runs.
Then: `.evidence/vb-nf2u/ai-release.yaml` contains subgate entries exactly named `ui_snapshot`, `layout_readability`, `redaction`, `negative_fixture`, `deterministic_capture`, and `evidence_shape`, each with command, status, artifact paths, diagnostic count, started/finished timestamps, and pass/fail result.

### Behavior: ai-release fails when UI snapshot gate is missing
Test function name: `ai_release_fails_when_ui_snapshot_gate_missing`
Given: a test release profile for bead `vb-nf2u` omits only the `ui_snapshot` subgate.
When: profile validation runs for `cargo xtask ai-release --bead vb-nf2u`.
Then: `Err(UiReleaseGateError::ReleaseProfileIncomplete { code: "release_profile_incomplete", bead_id: "vb-nf2u", missing_subgates: ["ui_snapshot"], action })`.

### Behavior: ai-release fails when overlap/layout gate is missing
Test function name: `ai_release_fails_when_ui_overlap_gate_missing`
Given: a test release profile omits only the `layout_readability` subgate.
When: profile validation runs for `cargo xtask ai-release --bead vb-nf2u`.
Then: `Err(UiReleaseGateError::ReleaseProfileIncomplete { code: "release_profile_incomplete", bead_id: "vb-nf2u", missing_subgates: ["layout_readability"], action })`.

### Behavior: ai-release fails when redaction gate is missing
Test function name: `ai_release_fails_when_ui_redaction_gate_missing`
Given: a test release profile omits only the `redaction` subgate.
When: profile validation runs for `cargo xtask ai-release --bead vb-nf2u`.
Then: `Err(UiReleaseGateError::ReleaseProfileIncomplete { code: "release_profile_incomplete", bead_id: "vb-nf2u", missing_subgates: ["redaction"], action })`.

### Behavior: ai-release fails when negative-fixture gate is missing
Test function name: `ai_release_fails_when_ui_negative_fixture_gate_missing`
Given: a test release profile omits only the `negative_fixture` subgate.
When: profile validation runs for `cargo xtask ai-release --bead vb-nf2u`.
Then: `Err(UiReleaseGateError::ReleaseProfileIncomplete { code: "release_profile_incomplete", bead_id: "vb-nf2u", missing_subgates: ["negative_fixture"], action })`.

### Behavior: ai-release fails when determinism gate is missing
Test function name: `ai_release_fails_when_ui_determinism_gate_missing`
Given: a test release profile omits only the `deterministic_capture` subgate.
When: profile validation runs for `cargo xtask ai-release --bead vb-nf2u`.
Then: `Err(UiReleaseGateError::ReleaseProfileIncomplete { code: "release_profile_incomplete", bead_id: "vb-nf2u", missing_subgates: ["deterministic_capture"], action })`.

### Behavior: ai-release fails when evidence-shape gate is missing
Test function name: `ai_release_fails_when_ui_evidence_shape_gate_missing`
Given: a test release profile omits only the `evidence_shape` subgate.
When: profile validation runs for `cargo xtask ai-release --bead vb-nf2u`.
Then: `Err(UiReleaseGateError::ReleaseProfileIncomplete { code: "release_profile_incomplete", bead_id: "vb-nf2u", missing_subgates: ["evidence_shape"], action })`.

### Behavior: missing evidence fails closed
Test function name: `missing_ui_evidence_error_returns_typed_variant_and_diagnostic`
Given: a report references a missing/unreadable fixture-text artifact, missing digest, missing check result, missing YAML report, or missing negative-fixture evidence file.
When: evidence validation runs.
Then: `Err(UiReleaseGateError::MissingEvidence { code: "missing_evidence", screen_id, artifact_path, evidence_kind, action })`, with `evidence_kind` exactly `fixture_text_artifact`, `report`, `digest`, `check`, or `negative_fixture`.

### Behavior: core parity overclaim is rejected
Test function name: `core_parity_unsupported_error_returns_typed_variant_when_fixture_evidence_overclaims_live_parity`
Given: fixture-backed or synthetic release evidence while bead labels include `blocked-by-core` or `ui-paused`.
When: evidence text claims live core/runtime/CLI parity.
Then: `Err(UiReleaseGateError::CoreParityUnsupported { code: "core_parity_unsupported", claim, blocker, action })`, where `claim` names the overclaim and `blocker` is `blocked-by-core` or `ui-paused`.

## 3.1 Existing `UiSnapshotError` variant scenarios
Each scenario uses the public snapshot/check/report/token/image/io operation that owns the failure. These are not acceptable as one omnibus test.

### Behavior: fixture lookup reports missing fixture
Test function name: `ui_snapshot_returns_fixture_not_found_when_fixture_id_unknown`
Given: fixture ID `unknown_vb_nf2u_screen`.
When: the public fixture loader runs.
Then: `Err(UiSnapshotError::FixtureNotFound { fixture_id: "unknown_vb_nf2u_screen" })`.

### Behavior: snapshot command failure is typed
Test function name: `ui_snapshot_returns_snapshot_command_failed_when_renderer_exits_nonzero`
Given: a renderer command result with exit code `17` and stderr `render failed`.
When: the public snapshot command wrapper runs.
Then: `Err(UiSnapshotError::SnapshotCommandFailed { command, exit_code: 17, stderr })`.

### Behavior: fixture-text artifact generation failure is typed
Test function name: `ui_snapshot_returns_fixture_text_generation_failed_when_writer_rejects_target`
Given: a valid fixture and an unwritable fixture-text output target.
When: fixture-text artifact generation runs.
Then: `Err(UiSnapshotError::IoError { artifact_path, operation: "write_fixture_text", source_kind })`.

### Behavior: overlap detection is typed
Test function name: `ui_snapshot_returns_overlap_detected_when_controls_intersect`
Given: screen `execution_overview` contains control `run_button` at rectangle `{ x: 10, y: 10, width: 100, height: 60 }` and control `stop_button` at rectangle `{ x: 80, y: 40, width: 50, height: 50 }`.
When: the public overlap check runs.
Then: `Err(UiSnapshotError::OverlapDetected { screen_id: "execution_overview", first_control_id: "run_button", second_control_id: "stop_button", overlap_area_px: 600 })`.

### Behavior: label clipping is typed
Test function name: `ui_snapshot_returns_label_clipped_when_label_exceeds_container`
Given: a label rectangle extending beyond its container.
When: the public clipping check runs.
Then: `Err(UiSnapshotError::LabelClipped { screen_id, control_id, label_bounds, container_bounds })`.

### Behavior: unreadable chip is typed
Test function name: `ui_snapshot_returns_chip_unreadable_when_chip_area_or_contrast_below_threshold`
Given: a chip with zero visible area or below-threshold contrast.
When: the public chip readability check runs.
Then: `Err(UiSnapshotError::ChipUnreadable { screen_id, control_id, visible_area_px, contrast_ratio, threshold })`.

### Behavior: control out of bounds is typed
Test function name: `ui_snapshot_returns_control_out_of_bounds_when_control_exceeds_viewport`
Given: a control rectangle whose right or bottom edge exceeds the viewport.
When: the public bounds check runs.
Then: `Err(UiSnapshotError::ControlOutOfBounds { screen_id, control_id, control_bounds, viewport_bounds })`.

### Behavior: selected state hidden is typed
Test function name: `ui_snapshot_returns_selected_state_hidden_when_indicator_missing_or_zero_area`
Given: a selected control with missing, hidden, or zero-area selected indicator.
When: the public selected-state check runs.
Then: `Err(UiSnapshotError::SelectedStateHidden { screen_id, control_id, selected_state_id, reason })`.

### Behavior: color drift is typed
Test function name: `ui_snapshot_returns_color_drift_when_token_color_diff_exceeds_threshold`
Given: image/token evidence with a color delta above threshold.
When: the public color drift check runs.
Then: `Err(UiSnapshotError::ColorDrift { screen_id, token_name, expected, actual, delta })`.

### Behavior: spelling violation is typed
Test function name: `ui_snapshot_returns_spelling_violation_when_unapproved_text_found`
Given: extracted text containing an unapproved spelling violation.
When: the public spelling check runs.
Then: `Err(UiSnapshotError::SpellingViolation { screen_id, term, suggestion, artifact_path })`.

### Behavior: missing screen is typed
Test function name: `ui_snapshot_returns_screen_missing_when_required_screen_omitted`
Given: a report missing `storage_doctor_ai_context`.
When: the public report validator runs.
Then: `Err(UiSnapshotError::ScreenMissing { screen_id: "storage_doctor_ai_context" })`.

### Behavior: incomplete report is typed
Test function name: `ui_snapshot_returns_report_incomplete_when_required_fields_absent`
Given: a report row missing digest or check list.
When: the public report validator runs.
Then: `Err(UiSnapshotError::ReportIncomplete { screen_id, missing_fields })` with `missing_fields` exactly naming the absent field set.

### Behavior: token parse error is typed
Test function name: `ui_snapshot_returns_token_parse_error_when_token_or_hex_input_malformed`
Given: malformed token text or hex color `#12`.
When: the public token parser runs.
Then: `Err(UiSnapshotError::TokenParseError { token_name, value, reason })`.

### Behavior: image error is typed
Test function name: `ui_snapshot_returns_artifact_error_when_fixture_text_unreadable_or_malformed`
Given: a missing, unreadable, or malformed fixture-text artifact path.
When: the public fixture-text loader/checker runs.
Then: `Err(UiSnapshotError::IoError { artifact_path, operation, source_kind })` or `Err(UiSnapshotError::ReportIncomplete { screen_id, missing_fields })` with exact malformed fields.

### Behavior: IO error is typed
Test function name: `ui_snapshot_returns_io_error_when_filesystem_read_or_write_fails`
Given: a path denied for read or write by tempdir permissions.
When: the public snapshot/report file operation runs.
Then: `Err(UiSnapshotError::IoError { artifact_path, operation, source_kind })`.

## 4. Proptest Invariants

### Proptest: inventory validation
Invariant: any permutation of exactly the canonical eight unique IDs validates to the same canonical set; any missing/duplicate/extra/unknown ID fails with `InvalidScreenInventory`.
Strategy: generate `Vec<String>` from canonical IDs plus unknown ASCII IDs, length `0..12`.
Anti-invariant: any vector whose sorted unique canonical subset is not exactly the eight IDs always fails.

### Proptest: screen bijection
Invariant: every one-to-one mapping across fixture ID, `UiScreenKind`, `ShellNav`, `Screen`, and report ID has cardinality eight and no dangling edge.
Strategy: permute the eight canonical rows and optionally delete/duplicate one edge.
Anti-invariant: any mapping with count not equal to eight or repeated target fails with `UnreachableScreen` or `InvalidScreenInventory`.

### Proptest: rectangle overlap predicate
Invariant: overlap is symmetric; intersecting rectangles report exact `overlap_area_px` equal to `intersection_width * intersection_height` using checked arithmetic.
Strategy: bounded rectangles within and slightly outside `1920x1080` using checked coordinates/dimensions.
Anti-invariant: intersecting positive-area rectangles must never be reported as non-overlapping.

### Proptest: clipping/bounds predicates
Invariant: a control is valid only if all edges lie within configured viewport/shell/content bounds using checked arithmetic.
Strategy: rectangles around edges `0`, `OUTER_MARGIN`, `SIDEBAR_WIDTH`, `TOP_BAR_HEIGHT`, `BASELINE_WIDTH`, `BASELINE_HEIGHT`, plus random interior rectangles.
Anti-invariant: right edge greater than width, bottom edge greater than height, or overflow-derived dimensions always fail.

### Proptest: chip readability predicate
Invariant: chip readability succeeds only when visible area is positive and dimensions/contrast meet thresholds.
Strategy: generate chip text, visible rectangle, container rectangle, and contrast ratios around the threshold.
Anti-invariant: zero-area chips, clipped chips, empty labels, or contrast below threshold always fail.

### Proptest: selected-state predicate
Invariant: selected-state success requires a visible, in-bounds, non-zero-area indicator tied to the selected node/control.
Strategy: generate selected node IDs plus indicator rectangles and hidden/visible flags.
Anti-invariant: hidden flag, zero-area indicator, missing indicator, or out-of-bounds indicator always fails.

### Proptest: redaction scanner
Invariant: any artifact text containing a denied raw secret class returns `RedactionViolation` and diagnostics never include raw secret bytes.
Strategy: generate strings inserting sentinel/API-key/token/password/idempotency-key/tainted values into benign YAML/text/metadata templates.
Anti-invariant: any exact raw denied value in any artifact must fail.

### Proptest: approved placeholders
Invariant: approved placeholders pass only when no raw secret is present.
Strategy: generate artifact templates containing `[REDACTED:api_key]`, `[REDACTED:token]`, `[REDACTED:password]`, and benign text.
Anti-invariant: malformed placeholders that include raw suffix/prefix secret material fail.

### Proptest: normalized evidence determinism
Invariant: normalized report content excludes output-dir-specific absolute paths and volatile wall-clock/random values, so repeated captures with identical logical evidence normalize identically.
Strategy: generate report structs with path prefixes, fixed screen rows, digest strings, and optional volatile fields.
Anti-invariant: changing a digest, check status, screen ID, or diagnostic changes normalized output.

## 5. Fuzz Targets

### Fuzz Target: `ui_redaction_artifact`
Input type: arbitrary bytes interpreted as lossy UTF-8 artifact text plus optional artifact kind tag.
Risk: scanner panic, OOM, Unicode boundary mistakes, raw secret false negatives, diagnostic secret echo.
Corpus seeds: empty bytes; canonical screen IDs; YAML report with `password=hunter2`; `sk_test_vb_nf2u_raw_secret`; `Bearer vb_nf2u_token`; `Idempotency-Key: idem_vb_nf2u_secret`; approved `[REDACTED:api_key]`; very long line; invalid UTF-8.

### Fuzz Target: `ui_snapshot_report_yaml`
Input type: bytes/YAML-like text for report loading or evidence-shape validation if deserialization is added.
Risk: parser panic, expansion/OOM, missing check false pass, duplicate screen acceptance.
Corpus seeds: valid eight-screen report; missing screen; duplicate screen; unknown check kind; missing fixture-text artifact path; raw secret in diagnostics; malformed YAML.

### Fuzz Target: `ui_token_and_color_inputs`
Input type: TOML/token text and hex color strings used by snapshot color checks.
Risk: token parse panic, invalid hex fallback masking errors, color-drift false pass.
Corpus seeds: valid tokens file; `#000000`; `#ffffff`; invalid `#12`; non-UTF8; oversized string; lowercase/uppercase hex; embedded raw secret in token metadata.

## 6. Kani Harnesses

### Kani Harness: inventory bijection
Property: all bounded enum/index combinations across eight screens map exactly once and reject duplicate/missing edges.
Bound: eight screens; all `UiScreenKind` discriminants `0..=7`; all `ShellNav` variants.
Rationale: finite state space is small and critical for release coverage.

### Kani Harness: overlap predicate
Property: overlap is symmetric, panic-free, uses checked arithmetic, and cannot mark intersecting positive-area rectangles as non-overlapping.
Bound: `u16` or bounded `u32` coordinates/dimensions within `0..=4096`.
Rationale: arithmetic/index mistakes create false release passes.

### Kani Harness: clipping predicate
Property: controls extending beyond viewport are rejected and no coordinate addition overflows.
Bound: viewport up to `4096x4096`; rectangles near all edges.
Rationale: out-of-bounds UI must never pass due to wrapping/saturating mistakes.

### Kani Harness: bounds predicate
Property: all shell/content/nav bounds checks use checked coordinate and dimension arithmetic.
Bound: baseline constants plus generated margins/sidebar/topbar values within sane UI limits.
Rationale: checked arithmetic is mandated and layout safety is release-critical.

### Kani Harness: chip readability predicate
Property: success implies positive visible area and threshold-satisfying dimensions/contrast.
Bound: chip width/height `0..=512`; contrast modeled around threshold.
Rationale: readability must not pass zero-area or below-threshold chips.

### Kani Harness: selected-state predicate
Property: success implies selected indicator exists, is visible, non-zero-area, and in bounds.
Bound: node count `0..=32`; indicator rectangles bounded to viewport.
Rationale: hidden selected state was a contract failure mode.

### Kani Harness: redaction diagnostic non-echo for bounded secrets
Property: for bounded secret classes, violation diagnostics contain class/redacted sample but not exact raw secret bytes.
Bound: secret length `1..=64`, artifact length `0..=512` if a pure scanner kernel exists.
Rationale: scanner must fail closed without leaking in diagnostics.

## 7. Mutation Testing Checkpoints
Mutation threshold: `>=90%` killed mutants overall, and `100%` kill for fail-open mutations listed here.

- Removing a required fixture from `REQUIRED_FIXTURES` is caught by `test_all_eight_screens_pass_reachability_and_overlap_gates` and `invalid_screen_inventory_error_returns_typed_variant_and_diagnostic`.
- Allowing duplicate/unknown screen IDs is caught by `invalid_screen_inventory_error_returns_typed_variant_and_diagnostic`.
- Mapping `ShellNav::Storage` to the wrong `Screen` is caught by `unreachable_screen_error_returns_typed_variant_and_diagnostic`.
- Replacing real overlap predicate with `false`/empty result is caught by `test_intentional_overlap_fixture_fails_gate` and `ui_snapshot_returns_overlap_detected_when_controls_intersect`.
- Inverting overlap intersection comparison is caught by overlap proptest and Kani overlap harness.
- Replacing clipping/bounds checks with pass-all is caught by `layout_violation_error_returns_typed_variant_and_diagnostic`, `ui_snapshot_returns_label_clipped_when_label_exceeds_container`, and `ui_snapshot_returns_control_out_of_bounds_when_control_exceeds_viewport`.
- Lowering chip readability threshold to zero is caught by `ui_snapshot_returns_chip_unreadable_when_chip_area_or_contrast_below_threshold` plus chip proptest/Kani.
- Treating hidden selected state as visible is caught by `ui_snapshot_returns_selected_state_hidden_when_indicator_missing_or_zero_area` plus selected-state proptest/Kani.
- Removing a redaction class from denylist is caught by `redaction_denylist_includes_raw_secret_sentinels_api_keys_tokens_passwords_idempotency_keys_and_tainted_fixture_values`.
- Replacing redaction scanner with pass-all is caught by `test_intentional_secret_fixture_fails_redaction_gate`, `redaction_violation_error_returns_typed_variant_and_diagnostic_without_echoing_secret`, and fuzz seeds.
- Echoing raw secret in diagnostic is caught by `redaction_violation_error_returns_typed_variant_and_diagnostic_without_echoing_secret`.
- Omitting any UI subgate is caught by its named omission test: `ai_release_fails_when_ui_snapshot_gate_missing`, `ai_release_fails_when_ui_overlap_gate_missing`, `ai_release_fails_when_ui_redaction_gate_missing`, `ai_release_fails_when_ui_negative_fixture_gate_missing`, `ai_release_fails_when_ui_determinism_gate_missing`, `ai_release_fails_when_ui_evidence_shape_gate_missing`.
- Ignoring overlap false-pass negative fixtures is caught by `false_pass_fixture_rejects_overlap_fixture_that_reports_passed`.
- Ignoring secret false-pass negative fixtures is caught by `false_pass_fixture_rejects_secret_fixture_that_reports_passed`.
- Replacing fixed snapshot timestamp with wall clock is caught by `snapshot_determinism_rejects_wall_clock_time_with_exact_diagnostic`, `repeated_ui_snapshot_is_stable`, and `deterministic_snapshot_mode_fixes_time_and_pauses_hidden_animations_before_capture`.
- Leaving hidden animations running is caught by `snapshot_determinism_rejects_unpaused_hidden_animation_with_exact_diagnostic`.
- Allowing digest drift is caught by `snapshot_determinism_rejects_digest_drift_with_exact_diagnostic`.
- Removing digest/check/freshness/provenance fields from evidence is caught by `ui_snapshot_all_emits_eight_fixture_text_artifacts_and_complete_yaml_report` and `ui_snapshot_evidence_rejects_stale_checked_in_artifacts`.
- Allowing fixture-backed evidence to claim live runtime parity is caught by `core_parity_unsupported_error_returns_typed_variant_when_fixture_evidence_overclaims_live_parity` and static scan.

## 8. Combinatorial Coverage Matrix

### Function-by-function boundary matrix
| Function | Scenario | Input class | Expected output | Layer |
|---|---|---|---|---|
| `canonical_ui_release_inventory` | happy path | exact eight canonical IDs | exact canonical inventory with 8 rows | unit |
| `canonical_ui_release_inventory` | empty/missing | empty/7 IDs | `Err(UiReleaseGateError::InvalidScreenInventory { code: "invalid_screen_inventory", ... })` | unit |
| `canonical_ui_release_inventory` | duplicate | repeated canonical ID | `Err(UiReleaseGateError::InvalidScreenInventory { code: "invalid_screen_inventory", ... })` | unit/proptest |
| `canonical_ui_release_inventory` | unknown/extra | non-canonical ID | `Err(UiReleaseGateError::InvalidScreenInventory { code: "invalid_screen_inventory", ... })` | unit/proptest |
| `canonical_ui_release_inventory` | boundary count | 0, 1, 7, 8, 9, 12 IDs | exact success only at 8 unique canonical IDs | unit/proptest |
| `validate_screen_bijection` | valid mapping | one-to-one edges | exact `()` and 8 unique report IDs | unit/Kani |
| `validate_screen_bijection` | missing edge | absent `ShellNav`/fixture/report | `Err(UiReleaseGateError::UnreachableScreen { code: "unreachable_screen", ... })` | unit |
| `validate_screen_bijection` | duplicate edge | repeated target | `Err(UiReleaseGateError::UnreachableScreen { code: "unreachable_screen", ... })` | unit/proptest |
| `validate_screen_bijection` | dangling edge | fixture with no report ID | `Err(UiReleaseGateError::UnreachableScreen { code: "unreachable_screen", ... })` | unit |
| `validate_screen_bijection` | enum boundary | all 8 discriminants | exact bijection, no unchecked indexing | Kani |
| `enter_release_snapshot_mode` | valid deterministic config | fixed time + pause | guard evidence with fixed timestamp and `hidden_animations_paused: true` | unit/integration |
| `enter_release_snapshot_mode` | wall clock | no fixed time | `Err(UiReleaseGateError::SnapshotDeterminismViolation { code: "snapshot_determinism_violation", ... })` | unit |
| `enter_release_snapshot_mode` | hidden animation | pause disabled | `Err(UiReleaseGateError::SnapshotDeterminismViolation { code: "snapshot_determinism_violation", ... })` | unit |
| `enter_release_snapshot_mode` | timestamp boundary | None/zero/min/max timestamp | exact deterministic marker for valid fixed time; exact determinism error otherwise | unit |
| `enter_release_snapshot_mode` | guard exit | guard dropped | evidence records finalize marker; Miri reports no UB | integration/Miri |
| `capture_all_release_screens` | happy path | all fixtures | 8 fresh `.fixture.txt` artifacts + report + matching digests | integration |
| `capture_all_release_screens` | missing fixture | one fixture absent | `Err(UiReleaseGateError::MissingEvidence { evidence_kind: "fixture_text_artifact", ... })` | integration |
| `capture_all_release_screens` | unreadable path | permission denied | `Err(UiReleaseGateError::MissingEvidence { evidence_kind: "fixture_text_artifact", ... })` | integration |
| `capture_all_release_screens` | stale artifact | old mtime/wrong tempdir | `Err(UiReleaseGateError::MissingEvidence { evidence_kind: "fresh_snapshot_artifact", ... })` | integration |
| `capture_all_release_screens` | digest drift | changed bytes | `Err(UiReleaseGateError::SnapshotDeterminismViolation { code: "snapshot_determinism_violation", ... })` | integration |
| `check_layout_safety` | overlap | intersecting rectangles | `Err(UiReleaseGateError::LayoutViolation { predicate: "overlap", ... })` | unit/Kani |
| `check_layout_safety` | clipping | label outside container | `Err(UiReleaseGateError::LayoutViolation { predicate: "clipping", ... })` | unit |
| `check_layout_safety` | bounds | rectangle outside viewport | `Err(UiReleaseGateError::LayoutViolation { predicate: "bounds", ... })` | unit/Kani |
| `check_layout_safety` | chip unreadable | zero area/low contrast | `Err(UiReleaseGateError::LayoutViolation { predicate: "chip_readability", ... })` | unit/Kani |
| `check_layout_safety` | selected hidden | missing/hidden indicator | `Err(UiReleaseGateError::LayoutViolation { predicate: "selected_state", ... })` | unit/Kani |
| `check_redaction_artifacts` | sentinel | raw sentinel | `Err(UiReleaseGateError::RedactionViolation { secret_class: "sentinel", ... })` | unit/fuzz |
| `check_redaction_artifacts` | api key | raw API key | `Err(UiReleaseGateError::RedactionViolation { secret_class: "api_key", ... })` | unit/fuzz |
| `check_redaction_artifacts` | token | raw bearer token | `Err(UiReleaseGateError::RedactionViolation { secret_class: "token", ... })` | unit/fuzz |
| `check_redaction_artifacts` | password | raw password | `Err(UiReleaseGateError::RedactionViolation { secret_class: "password", ... })` | unit/fuzz |
| `check_redaction_artifacts` | idempotency key | raw idempotency key | `Err(UiReleaseGateError::RedactionViolation { secret_class: "idempotency_key", ... })` | unit/fuzz |
| `check_redaction_artifacts` | tainted fixture | fixture secret | `Err(UiReleaseGateError::RedactionViolation { secret_class: "tainted_fixture_value", ... })` | unit |
| `check_redaction_artifacts` | approved placeholder | only placeholders | exact redaction evidence with no diagnostics | unit/proptest |
| `run_ui_negative_fixtures` | overlap expected-fail | bad overlap fixture | negative evidence marks expected failure under `layout` | integration/acceptance |
| `run_ui_negative_fixtures` | secret expected-fail | bad secret fixture | negative evidence marks expected failure under `redaction` | integration/acceptance |
| `run_ui_negative_fixtures` | overlap false-pass | bad overlap reports passed | `Err(UiReleaseGateError::FalsePassFixtureViolation { expected_gate: "layout", actual_status: "passed", ... })` | integration |
| `run_ui_negative_fixtures` | secret false-pass | bad secret reports passed | `Err(UiReleaseGateError::FalsePassFixtureViolation { expected_gate: "redaction", actual_status: "passed", ... })` | integration |
| `run_ui_negative_fixtures` | missing evidence | missing negative fixture log | `Err(UiReleaseGateError::MissingEvidence { evidence_kind: "negative_fixture", ... })` | integration |
| `run_ui_release_subgates_for_ai_release` | complete profile | all six gates | exact release evidence with six subgates | integration/e2e |
| `run_ui_release_subgates_for_ai_release` | missing snapshot | omit `ui_snapshot` | `Err(UiReleaseGateError::ReleaseProfileIncomplete { missing_subgates: ["ui_snapshot"], ... })` | integration |
| `run_ui_release_subgates_for_ai_release` | missing layout | omit `layout_readability` | `Err(UiReleaseGateError::ReleaseProfileIncomplete { missing_subgates: ["layout_readability"], ... })` | integration |
| `run_ui_release_subgates_for_ai_release` | missing redaction | omit `redaction` | `Err(UiReleaseGateError::ReleaseProfileIncomplete { missing_subgates: ["redaction"], ... })` | integration |
| `run_ui_release_subgates_for_ai_release` | missing remaining gate | omit negative/determinism/evidence | exact `ReleaseProfileIncomplete` naming only omitted gate | integration |
| `include_ui_gates_in_ai_release` | bead accepted | `vb-nf2u` | profile evidence includes exact commands and evidence paths | integration/e2e |
| `include_ui_gates_in_ai_release` | wrong bead | non-`vb-nf2u` for this test fixture | exact profile rejection with bead ID field | integration |
| `include_ui_gates_in_ai_release` | missing profile | no UI profile | `Err(UiReleaseGateError::ReleaseProfileIncomplete { bead_id: "vb-nf2u", ... })` | integration |
| `include_ui_gates_in_ai_release` | overclaim | fixture evidence claims live parity | `Err(UiReleaseGateError::CoreParityUnsupported { code: "core_parity_unsupported", ... })` | static/integration |
| `include_ui_gates_in_ai_release` | evidence names | generated release evidence | commands and evidence paths exactly equal contract paths | integration |

### Execution-marker and provenance assertions
| Check | Required marker/measurement | Mutation killed |
|---|---|---|
| Overlap | pair count inspected, control IDs, intersection area | stub returning empty overlap |
| Clipping | label bounds, container bounds, clipped edge | no-op clipping check |
| Bounds | viewport bounds, control bounds, checked edge sums | unchecked/saturating arithmetic false pass |
| Chip readability | visible area, contrast ratio, threshold | threshold set to zero |
| Selected state | selected control ID, indicator bounds, visibility flag | hidden indicator treated visible |
| Fixture artifact provenance | fixture-text artifact path, byte length, parsed geometry, digest | stale or unreadable fixture artifact accepted |
| Redaction | artifact path, exact six-class coverage map, scanned byte count, redacted sample | scanner not executed or class omitted |
| Freshness | command start/end timestamps, mtime within interval, tempdir root | checked-in stale artifacts accepted |
| Digest provenance | YAML digest equals computed fixture-text digest and normalized report digest | hard-coded digest accepted |

## Error-variant coverage checklist

### Contract `UiReleaseGateError` variants
- `InvalidScreenInventory`: `invalid_screen_inventory_error_returns_typed_variant_and_diagnostic`.
- `UnreachableScreen`: `unreachable_screen_error_returns_typed_variant_and_diagnostic`.
- `SnapshotDeterminismViolation`: `snapshot_determinism_rejects_wall_clock_time_with_exact_diagnostic`, `snapshot_determinism_rejects_unpaused_hidden_animation_with_exact_diagnostic`, and `snapshot_determinism_rejects_digest_drift_with_exact_diagnostic`.
- `MissingEvidence`: `missing_ui_evidence_error_returns_typed_variant_and_diagnostic` and `ui_snapshot_evidence_rejects_stale_checked_in_artifacts`.
- `LayoutViolation`: `layout_violation_error_returns_typed_variant_and_diagnostic`.
- `RedactionViolation`: `redaction_violation_error_returns_typed_variant_and_diagnostic_without_echoing_secret`.
- `FalsePassFixtureViolation`: `false_pass_fixture_rejects_overlap_fixture_that_reports_passed` and `false_pass_fixture_rejects_secret_fixture_that_reports_passed`.
- `ReleaseProfileIncomplete`: six per-subgate omission tests above.
- `CoreParityUnsupported`: `core_parity_unsupported_error_returns_typed_variant_when_fixture_evidence_overclaims_live_parity`.

### Existing `UiSnapshotError` variants
- `FixtureNotFound`: `ui_snapshot_returns_fixture_not_found_when_fixture_id_unknown`.
- `SnapshotCommandFailed`: `ui_snapshot_returns_snapshot_command_failed_when_renderer_exits_nonzero`.
- `IoError` fixture-text generation: `ui_snapshot_returns_fixture_text_generation_failed_when_writer_rejects_target`.
- `OverlapDetected`: `ui_snapshot_returns_overlap_detected_when_controls_intersect`.
- `LabelClipped`: `ui_snapshot_returns_label_clipped_when_label_exceeds_container`.
- `ChipUnreadable`: `ui_snapshot_returns_chip_unreadable_when_chip_area_or_contrast_below_threshold`.
- `ControlOutOfBounds`: `ui_snapshot_returns_control_out_of_bounds_when_control_exceeds_viewport`.
- `SelectedStateHidden`: `ui_snapshot_returns_selected_state_hidden_when_indicator_missing_or_zero_area`.
- `ColorDrift`: `ui_snapshot_returns_color_drift_when_token_color_diff_exceeds_threshold`.
- `SpellingViolation`: `ui_snapshot_returns_spelling_violation_when_unapproved_text_found`.
- `ScreenMissing`: `ui_snapshot_returns_screen_missing_when_required_screen_omitted`.
- `ReportIncomplete`: `ui_snapshot_returns_report_incomplete_when_required_fields_absent`.
- `TokenParseError`: `ui_snapshot_returns_token_parse_error_when_token_or_hex_input_malformed`.
- `IoError`/`ReportIncomplete` fixture-text parsing: `ui_snapshot_returns_artifact_error_when_fixture_text_unreadable_or_malformed`.
- `IoError`: `ui_snapshot_returns_io_error_when_filesystem_read_or_write_fails`.

## Open Questions
- None blocking for test planning. Implementation may use deterministic fixtures while `blocked-by-core`/`ui-paused` applies, but every artifact must say fixture-backed and must not claim live core/runtime parity.
