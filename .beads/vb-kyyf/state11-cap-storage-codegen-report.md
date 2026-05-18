# State 11 cap-unblock storage/codegen regression report

STATUS: APPROVED

## Scope
- Bead: `vb-kyyf`
- State: 11 formal-verifier sublane `cap-unblock-storage-codegen-regression`
- Workspace: `/home/lewis/src/bd-vb-kyyf-bdd`
- Manifest: `.beads/vb-kyyf/dispatch-state11-cap-storage-codegen.json`

## Startup instruction evidence
- Read `/home/lewis/.claude/skills/formal-verifier/SKILL.md`: formal-verifier executes existing gates only, records command/exit/evidence, and must not invent evidence (lines 12-31, 100-114).
- Read `/home/lewis/.agents/skills/formal-verifier/SKILL.md`: identical content; per startup rule this copy wins on conflict.

## Command results

### cap-storage-replay-resume
- Command: `TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p vb_storage --test replay_resume`
- Exit status: 0
- Result: PASS
- Evidence: `cargo test: 3 passed (1 suite, 0.10s)`

### cap-storage-recovery-bdd
- Command: `TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p vb_storage --test recovery_bdd_tests`
- Exit status: 0
- Result: PASS
- Evidence: `cargo test: 29 passed, 2 ignored (1 suite, 0.81s)`

### cap-codegen-tests
- Command: `TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p vb_codegen`
- Exit status: 0
- Result: PASS
- Evidence: `cargo test: 367 passed (4 suites, 23.48s)`

### cap-storage-codegen-cli-check
- Command: `TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo check -p vb_cli -p vb_storage -p velvet-ballastics-workspace-tests --all-targets`
- Exit status: 0
- Result: PASS
- Evidence: `cargo build (0 crates compiled)` and `Finished dev profile [unoptimized + debuginfo] target(s) in 0.45s`

## Summary
- All four requested storage/codegen gates passed in the isolated workspace.
- No production, test, or proof files were modified.
- Ledger: `.beads/vb-kyyf/verification-ledger-cap-storage-codegen.jsonl`
