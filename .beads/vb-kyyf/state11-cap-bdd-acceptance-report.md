# vb-kyyf State 11 cap-unblock BDD acceptance report

STATUS: APPROVED

## Scope
- Bead: `vb-kyyf`
- State: 11 formal-verifier sublane `cap-unblock BDD acceptance`
- Workspace: `/home/lewis/src/bd-vb-kyyf-bdd`
- Manifest: `.beads/vb-kyyf/dispatch-state11-cap-bdd-acceptance.json`

## Startup instructions cited
- `/home/lewis/.claude/skills/formal-verifier/SKILL.md`: mission requires accounting for scoped proof obligations with real command evidence; no hallucinated evidence; record command, exit status, and output summary.
- `/home/lewis/.agents/skills/formal-verifier/SKILL.md`: same content/version observed; no conflict. If conflict existed, this file would win.

## Commands executed

### BDD-KYYF cap acceptance: cross-run determinism
- Command: `TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p velvet-ballastics-workspace-tests --test vb_kyyf_cross_run_determinism -- --test-threads=1`
- Exit status: 0
- Output summary: `cargo test: 16 passed (1 suite, 2.40s)`
- Result: PASS

### Acceptance catalog traceability
- Command: `TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p velvet-ballastics-workspace-tests --test vb_hxm0_acceptance_catalog -- --test-threads=1`
- Exit status: 0
- Output summary: `cargo test: 6 passed (1 suite, 0.00s)`
- Result: PASS

### Evidence integrity check
- Command: `for f in .evidence/vb-kyyf/*.md; do test -s "$f" || exit 1; done && rtk grep -q 'CLI replay/events/inspect|cli_.*command_name: "replay"|command_name: "replay"' .evidence/vb-kyyf/storage-replay-resume.md && rtk grep -q 'command_name: "events"' .evidence/vb-kyyf/storage-replay-resume.md && rtk grep -q 'command_name: "inspect"' .evidence/vb-kyyf/storage-replay-resume.md && rtk grep -q 'events=4' .evidence/vb-kyyf/storage-replay-resume.md && ! rg -q 'locked-writer|events=0' .evidence/vb-kyyf/storage-replay-resume.md`
- Exit status: 0
- Output summary: no output; all checks passed.
- Result: PASS

## Evidence files verified non-empty
- `.evidence/vb-kyyf/acceptance-catalog-traceability.md`
- `.evidence/vb-kyyf/generated-subset-fail-closed.md`
- `.evidence/vb-kyyf/generated-ir-parity.md`
- `.evidence/vb-kyyf/recovery-bdd-errors.md`
- `.evidence/vb-kyyf/non-replay-safe-actions.md`
- `.evidence/vb-kyyf/storage-replay-resume.md`
- `.evidence/vb-kyyf/bdd-cross-run-determinism.md`

## Storage replay/resume evidence facts
- `.evidence/vb-kyyf/storage-replay-resume.md` contains public surface `vb_storage journal and recovery APIs plus CLI replay/events/inspect`.
- CLI replay/events/inspect reports are present via `command_name: "replay"`, `command_name: "events"`, and `command_name: "inspect"`.
- CLI stdout contains `events=4` for replay/events/inspect.
- No `locked-writer` marker found.
- No `events=0` marker found.

## Artifacts written
- `.beads/vb-kyyf/state11-cap-bdd-acceptance-report.md`
- `.beads/vb-kyyf/verification-ledger-cap-bdd.jsonl`
