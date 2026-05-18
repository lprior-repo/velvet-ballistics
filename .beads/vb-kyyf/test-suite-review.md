# vb-kyyf State 9 Test Suite Review — BDD-KYYF-002 Cap-Unblock

STATUS: APPROVED

## Scope
- Bead: vb-kyyf only.
- Reviewed file: `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs`.
- Review limited to owner-authorized BDD-KYYF-002 CLI hardening.

## Startup citations
- `/home/lewis/.claude/skills/test-reviewer/SKILL.md`: lines 113-125 define suite review and fail-fast static analysis; lines 127-166 require banned-pattern/private-surface scans; lines 329-337 require exact cited findings and evidence.
- `/home/lewis/.agents/skills/test-reviewer/SKILL.md`: same content and wins on conflict; no conflict found.
- `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md`: lines 52-71 require resource close/cleanup; lines 114-123 reject swallowed errors; lines 195-210 require compile/execute evidence.

## Commands run from isolated workspace
```text
$ pwd -P
/home/lewis/src/bd-vb-kyyf-bdd
```

```text
$ test "$(pwd -P)" = /home/lewis/src/bd-vb-kyyf-bdd && TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p velvet-ballastics-workspace-tests --test vb_kyyf_cross_run_determinism bdd_kyyf_001_to_006_require_executable_public_surfaces_not_catalog_bookkeeping_only -- --test-threads=1
cargo test: 1 passed, 15 filtered out (1 suite, 1.17s)
```

## Static review evidence
- Banned weak assertion/private-surface scan in the focused test found no `assert!(result.is_ok())`, `assert!(result.is_err())`, `#[ignore]`, sleeps, mocks, or `use crate::` private integration-test laundering.
- The hardened CLI report type captures `command_name`, `status_code`, `stdout`, and `stderr`: `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs:93-99`.
- The test rejects locked-writer and zero-event success stubs: `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs:996-1012` rejects `storage is held by an active writer`, `writer_lock_held`, and `events=0`, while requiring `events=4`, run id, digest marker, scenario id, evidence path, and command name.
- Command-specific semantic facts are asserted: replay seq `0..3` and terminal facts at `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs:1014-1023`; events seq `0..3` and `4 event(s) total` at lines 1024-1030; inspect status/events facts at line 1031.
- Exact repeated CLI normalized report equality is asserted before accepting evidence: `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs:1036-1058` compares both `CliReport` arrays exactly and rechecks each command.
- The writer is closed before CLI reads: the initial journal writer scope ends at `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs:1391-1402`, the store is reopened for storage/recovery reads at lines 1403-1509, and `drop(reopened)` occurs before both CLI replay/events/inspect passes at lines 1510-1520.
- Evidence artifact records durable journal events `seq=0..3`, two CLI runs, status `Some(0)`, stdout/stderr, scenario id, command name, run id, evidence path, digest marker, and `events=4`: `.evidence/vb-kyyf/storage-replay-resume.md:12-19`.

## Findings
- No lethal findings in the BDD-KYYF-002 cap-unblock hardening.
- No major findings.
- No minor findings.

## Verdict
The suite now fails closed against the black-hat defect: a locked-writer / `writer_lock_held` / `events=0` success stub cannot satisfy BDD-KYYF-002. The focused test passes only after real persisted store close/drop/reopen semantics and repeated public CLI-readable evidence are present.
