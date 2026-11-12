# vb-ucsv7 Review: Current Backend Tool Fixture and Archived UI Test Transition

## Verdict: APPROVED

The tool fixture migration is correct. The archived UI tests are byte-exact against the pre-migration source, and the 6 replacement tests verify genuine current-scope CLI and backend behavior.

## Verification Performed

### Byte-Exact Archive Verification
The old `crates/workspace_tests/tests/vb_nf2u_ui_release_acceptance.rs` (from be65d9191527) is preserved byte-exact at `docs/history/vb_nf2u_ui_release_acceptance.rs.pre-vb-5aw5i`. Verified via `diff` comparison.

### Replacement Tests Are Genuine Current-Scope CLI Behavior
The 6 tests in `crates/workspace_tests/tests/vb_nf2u_current_scope_tests.rs` use portable `cargo run --manifest-path` invocation against the actual `velvet-ballistics` package:

1. `removed_ui_command_is_refused_with_unknown_command_error` — verifies `ui` command is rejected
2. `removed_ui_release_command_is_not_registered` — verifies `ui-release` command is rejected
3. `cli_help_lists_current_command_surface` — verifies current commands are listed, removed UI commands are not
4. `cli_version_prints_version_string` — verifies version output
5. `validate_command_runs_on_valid_workflow` — verifies core validate command works
6. `removed_codegen_ui_crates_are_not_workspace_members` — verifies workspace Cargo.toml excludes removed crates

### Four Active Test Suites Run Successfully
```
vb_eepg_bdd_tests:                    7 passed, 0 failed, 0 ignored
vb_8ma2_workspace_assertions:        33 passed, 0 failed, 0 ignored
vb_kkvb_xtask_command_shell_coverage: 2 passed, 0 failed, 6 ignored
vb_nf2u_current_scope_tests:          6 passed, 0 failed, 0 ignored
Total:                               48 passed, 0 failed, 6 ignored
```
Exit code 0. Matches expected counts exactly.

### No Newly Ignored or Skipped Tests
The 6 ignored tests in `vb_kkvb_xtask_command_shell_coverage` are the pre-existing UI-command ignores (`#[ignore] // UI commands moved to velvet-optional repo`). No new ignores introduced.

### eepg Known Kind 4 Mismatch Classification
The `is_known_record_kind_returns_false_for_invalid_kinds` test now correctly verifies that kind 4 (MAGIC_COMPILED_ARTIFACT) used with MAGIC_JOURNAL_EVENT produces `RecordKindFamilyMismatch` rather than `UnknownRecordKind`. True unknown kinds (0, 5-9, 38-39, 41-49, etc.) are still tested as producing `UnknownRecordKind`.

### Synthetic Workspace Valid/Invalid Controls Retained
The `vb_8ma2_workspace_assertions` suite still contains its 7 negative controls (extra member manifests). The manifest content was updated to match the current workspace structure, but the control structure is preserved.

### xtask Correct Virtual Workspace Manifest
The `vb_kkvb_xtask_command_shell_coverage` test now uses the correct `xtask/Cargo.toml` manifest path for xtask invocations, properly handling the virtual workspace exclude pattern.

## Source Code Review Findings

### Import Ordering (Minor)
`crates/workspace_tests/tests/vb_eepg_bdd_tests.rs` has `JournalError, RecordKind,` on the same line as the opening brace with nested `codec` import. rustfmt prefers these on separate lines. This is a pre-existing format issue (present in be65d9191527), not introduced by this change.

### Function Signature Formatting (Fixed)
`crates/workspace_tests/tests/vb_kkvb_xtask_command_shell_coverage.rs` had a function signature that wrapped incorrectly; this change fixed it to the proper rustfmt style.

### Test Implementation Quality
The `run_cli` helper uses `Command::new("cargo")` with `--quiet`, `--manifest-path`, and `-p velvet-ballistics` flags, which is portable and correct. The helper returns a `CommandOutput` struct with status, stdout, and stderr, allowing assertions on all three aspects of command execution.

## Conclusion
The migration correctly preserves the historical UI test source as an exact archive while replacing it with genuine current-scope CLI and backend behavior tests. The 4 active test suites compile and run successfully with no warnings and the expected test counts. The source code changes are appropriate and follow existing patterns.
