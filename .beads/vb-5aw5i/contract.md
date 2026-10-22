# Bead vb-5aw5i: Fix Tool and Fixture Boundary Failures

## Scope
Fix 12 failing tests across 4 test families from staged nextest run (14283 pass, 61 fail).

## Failures

### 1. vb_eepg_bdd_tests (1 fail)
- Test: `is_known_record_kind_returns_false_for_invalid_kinds`
- Issue: Kind 4 produces `RecordKindFamilyMismatch { magic: 1447184965, kind: 4 }` instead of `UnknownRecordKind`. Kind 4 is a known record kind for the compiled artifact family (MAGIC_COMPILED_ARTIFACT), not truly unknown.
- Fix: Remove kind 4 from invalid kinds list; add separate assertion that kind 4 with MAGIC_JOURNAL_EVENT produces RecordKindFamilyMismatch. Preserve exact family rejection semantics.

### 2. vb_8ma2_workspace_assertions (1 fail)
- Test: `valid_workspace_passes_sharpened_assertions`
- Issue: Synthetic workspace fixture declares outdated members (includes vb_queue_semantics) and outdated excludes (excludes fuzz, includes vb_ui). Actual workspace includes vb_expr, vb_yaml, fuzz as members and excludes xtask.
- Fix: Update fixture to match actual workspace Cargo.toml members and excludes. Retain all negative fixture assertions.

### 3. vb_kkvb_xtask_command_shell_coverage (2 fails)
- Tests: `xtask_help_lists_required_and_legacy_commands_when_requested`, `xtask_version_prints_package_version_when_requested`
- Issue: Tests use `--manifest-path` pointing to workspace Cargo.toml, but xtask is not a workspace member (it's excluded). Exit code 101.
- Fix: Change manifest path to `xtask/Cargo.toml` in run_xtask helper. Use workspace root as current_dir instead of TempDir.

### 4. vb_nf2u_ui_release_acceptance (8 fails)
- Tests: all_eight_screens_pass_reachability_and_overlap_gates, secret_values_are_redacted_in_every_screen, intentional_overlap_fixture_fails_gate, overlap_negative_fixture_is_consumed_by_command_boundary, intentional_secret_fixture_fails_redaction_gate, secret_negative_fixture_is_consumed_by_command_boundary, secret_values_are_redacted_in_every_screen, overlap_false_pass_fixture_is_rejected, secret_false_pass_fixture_is_rejected
- Issue: MASTER explicitly states current scope is Backend/IR Interpreter Complete; UI/codegen/maxperf are removed. Historical tests demand removed UI commands/screens.
- Fix: Archive original test file to docs/history with provenance. Create new test file testing current CLI/backend scope: verify removed UI command refusal, verify core CLI commands function, verify current workspace structure.

## Deliverables
- Fixed vb_eepg_bdd_tests.rs (kind 4 handling)
- Fixed vb_8ma2_workspace_assertions.rs (fixture update)
- Fixed vb_kkvb_xtask_command_shell_coverage.rs (manifest path)
- Archived vb_nf2u_ui_release_acceptance.rs to docs/history
- New vb_nf2u_current_scope_tests.rs testing current CLI/backend
- All 12 tests passing

## Non-goals
- No Go/proof planning stage
- No Kani/full verification
- No unsafe/unwrap/expect/panic/todo/dbg
- No artifact proof/runtime admission source modifications
- No broad refactors
