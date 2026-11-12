# Landing Report — vb-jpq7 Wave 1 Evidence Remediation

Date: 2026-05-24
Repository: `/home/lewis/src/velvet-ballistics`

## Work completed

- Restored the required Wave 1 proof/evidence bundle under `.beads/vb-jpq7-proof-wave1/` from `/home/lewis/src/vb-jpq7-wave1-proof/.beads/vb-jpq7-proof-wave1/`.
- Confirmed ignored log evidence files are present locally under `.beads/vb-jpq7-proof-wave1/evidence/`.
- Resolved dirty `fuzz/REDO_PLAN.md` state; HEAD already intentionally omits that file.
- Replaced the standalone truth-serum audit verdict with `TRUTH_SERUM_PASS` after remediation checks passed.

## Commands and outcomes

- `git status --short`: clean before final report add; no dirty source changes after restoring Moon-mutated test files.
- Required evidence/ledger verifier: `missing= []`; `verification-ledger-jsonl-lines= 41`; exit 0.
- `bash scripts/check-test-integrity.sh --self-test`: exit 0; expected fixture failures were reported as self-test PASS cases.
- `bash scripts/check-test-integrity.sh`: `test integrity: PASS base=HEAD`; exit 0.
- `moon ci`: exit 0, captured in `/tmp/opencode/moon-ci-vb-jpq7-remediation-final.log`.
- `bash scripts/check-beads-server-mode.sh`: pass.
- `bd dolt push`: exit 0 with `Push complete`; retained pre-existing auto-backup dangling-ref warning.
- `git pull --rebase`: pass.
- `git push`: pass.

## Residual risks

- Repository-wide `*.log` ignore means the two required `.log` evidence files are local evidence artifacts, not committed Git objects.
- `moon ci` emits existing task hasher warnings for absent optional fixture input directories.
- `bd dolt push` still reports a non-fatal backup dangling-ref warning after a successful Dolt remote push.
