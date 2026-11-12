bead_id: vb-zrop
bead_title: quality: fix verify-standard ignored fallible result gate
phase: 2
updated_at: 2026-05-18T00:00:00Z
attempt: 1-of-7

# Codebase Map

Baseline `moon run :verify-standard` failed only at ignored fallible result scan.

Scoped findings from `.beads/vb-zrop/baseline-verify-standard.log`:
- `crates/vb_runtime/src/action_queue/tests/bounded_queue_tests.rs:456` ignores `rx.try_recv()` in a source-tree test module.
- `crates/vb_storage/src/admission.rs:512` ignores `std::fs::remove_dir_all(&self.path)` in `TestJournal::drop`.
- `crates/vb_boundary_inventory/src/tests/api_tests.rs` ignores fallible setup calls at lines 47-48, 64-65, 79-80, 98-99, 114-115, 134, 164-165, 732-738.
- `crates/vb_cli/src/app_impl.rs:4916` swallows stderr write failure in a best-effort stderr fallback.
- `crates/vb_cli/src/commands_ai_context.rs:635` swallows stderr write failure in a best-effort stderr fallback.
- `crates/vb_ipc/src/client.rs:370` drops `remove_file` result in test cleanup guard.
- `crates/vb_ipc/src/server/dispatch.rs:89` drops `remove_file` result in test cleanup guard.

No dependency, Cargo, feature, or build-script changes are in scope.
Risk tags: release-blocker, quality-gate, filesystem-cleanup, test-helper, cli-output-best-effort, ipc-test-cleanup.
Required verifier modes: focused ignored-result scanner via `bash scripts/check-ignored-fallible-results.sh`, canonical `moon run :verify-standard`; `moon ci` only if canonical landing policy requires after code changes.
