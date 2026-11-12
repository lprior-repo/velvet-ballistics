# runtime-a3 terminal insertion black-hat fix status

## Scope

- Finding fixed: `Shard::terminal_runs_insert` / `RunAggregate::terminal_insert` could insert terminal membership for a never-admitted run.
- Fjall production code was not edited.
- Kani was not run for this fix and no Kani proof is claimed.

## Implementation notes

- `RunAggregate::terminal_insert` now rejects never-admitted/untracked runs with `RunNotFound`, rejects live `runs` owners with `RunAlreadyExists`, and only inserts fresh terminal membership after checked-out ownership proves the run was admitted.
- Terminal insertion remains idempotent for already-retained terminal runs.
- Cancel and kill paths mark active runs checked-out before removing the active `RunState`, then terminal insertion consumes that ownership and clears runtime/pending side data.
- Finish and fail cleanup paths leave checked-out ownership for terminal insertion to consume.

## Evidence

| Gate | Status | Evidence |
|---|---:|---|
| root/JJ checks | PASS | `raw/root-jj-checks-terminal-insert-fix.txt` |
| terminal never-admitted regression | PASS | `raw/targeted-terminal-never-admitted.txt` |
| terminal membership targeted suite | PASS | `raw/targeted-terminal-membership.txt` |
| terminal lifecycle targeted suite | PASS | `raw/targeted-terminal-suite.txt` |
| cancel cleanup targeted suite | PASS | `raw/targeted-cancel-cleanup.txt` |
| kill cleanup targeted suite | PASS | `raw/targeted-kill-cleanup.txt` |
| provenance targeted tests | PASS | `raw/targeted-provenance-tests.txt` |
| `vb_runtime --lib` | PASS | `raw/vb_runtime-lib-tests-terminal-insert-fix.txt` |
| `vb_storage --lib recovery` | PASS | `raw/vb_storage-lib-recovery-terminal-insert-fix.txt` |
| fmt | PASS | `raw/fmt-terminal-insert-fix.txt` |
| check | PASS | `raw/check-terminal-insert-fix.txt` |
| clippy/source lint | PASS | `raw/lint-src-terminal-insert-fix.txt` |
| source length | PASS | `raw/source-length-terminal-insert-fix.txt` |
| jj diff summary | PASS | `raw/jj-diff-summary-terminal-insert-fix.txt` |

## Residual notes

- No performance claim is made.
- No second-ring assembly/API/provenance evidence was required for this runtime ownership fix.
- A first `vb_storage --lib recovery` attempt used relative `TMPDIR=target/tmp` and failed because tempdir resolution occurred under `crates/vb_storage`; the recorded passing rerun uses an absolute workspace `TMPDIR`.
