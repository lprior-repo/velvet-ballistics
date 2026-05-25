# vb-kyyf Regression Diff

STATUS: APPROVED

## Baseline/scope references

- Baseline report states later State 11 failures must be classified by delivery scope and known global debt, not hidden.
- Delivery scope lists vb-kyyf planned test/proof artifacts under `crates/workspace_tests`, `crates/vb_storage`, `crates/vb_codegen`, `verification/tla`, and `verification/verus`; it does not list `crates/vb_cli/tests/mode_activation_integration_tests.rs` or `crates/vb_cli/tests/vb_qi37_13_structured_reconciliation.rs` as planned obligation artifacts.
- Required canonical package name for workspace BDD was honored: `velvet-ballistics-workspace-tests`.

## Scoped bead-local result

- PO-001..PO-007 BDD/test lanes: PASS.
- PO-008 TLA+ lane: PASS.
- PO-009 Verus lane: PASS.
- All expected `.evidence/vb-kyyf/*.md` artifacts: non-empty.

## Workspace gate failure classification

- Command: `moon ci`.
- Exit: `1`.
- Failing task 1: `velvet-ballistics:test`.
- Raw failure 1: `vb_cli::mode_activation_integration_tests inspect_fails_fast_with_storage_error_on_invalid_path` expected exit `Some(5)` but got `Some(0)` at `crates/vb_cli/tests/mode_activation_integration_tests.rs:544`.
- Raw failure 2: `vb_cli::vb_qi37_13_structured_reconciliation storage_open_json_emits_storage_diagnostic_to_stderr_only` expected exit `Some(5)` but got `Some(0)` at `crates/vb_cli/tests/vb_qi37_13_structured_reconciliation.rs:587`.
- Failing task 2: `velvet-ballistics:mutants-smoke`.
- Raw failure 3: `Error: Failed to copy /home/lewis/src/bd-vb-kyyf-bdd/.tlc-metadir/26-05-18-00-51-01/8059 to /tmp/cargo-mutants-bd-vb-kyyf-bdd-cdgGPI.tmp/.tlc-metadir/26-05-18-00-51-01/8059`; cause: `Disk quota exceeded (os error 122)`.
- Full output: `/home/lewis/.local/share/opencode/tool-output/tool_e3cd7fbff001gu1aeCytJUYiDo`.
- Classification: `DEFERRED_GLOBAL` for PO-010 because all vb-kyyf local/touched-crate/protocol obligations passed first, the two test failures are outside planned vb-kyyf obligation artifacts, and the mutants failure is environment quota debt caused while copying TLC metadir state.
- Follow-up: route separate `vb_cli` storage-error exit-code regression and clean/relocate TLC/cargo-mutants temp storage; do not block vb-kyyf formal proof/test execution on these workspace/global failures.
