# qi37 combined integration landing evidence

- Workspace: `/home/lewis/src/Velvet-ballistics-landing-qi37-combined`
- Base: `main@origin` / `c9939431` after `jj git fetch` returned `Nothing changed`.
- Combined change before this evidence file: `xuqoyknz` / `bc95e98e`.
- Integration order: restored `vb-qi37.4.4`, applied `vb-qi37.4.3` strict split, re-applied non-overlapping `4.4` split/error files, mapped `4.5` semantics without include-body layout, then mapped `vb-qi37.16.4` ask-answer semantics into split files.
- Conflict/layout policy: final diff was checked for the forbidden include-body filename pattern; no matches were present.

## Commands run

- `jj git fetch` — PASS, `Nothing changed`.
- `jj workspace add --name landing-qi37-combined --revision main@origin ...` — PASS.
- `rtk cargo fmt --all` — PASS.
- `moon run :source-length` — PASS.
- `rtk cargo test -p vb_runtime journal::` — PASS, 9 passed.
- `rtk cargo test -p vb_runtime runtime::` — PASS, 61 passed.
- `rtk cargo test -p vb_runtime shard::` — PASS after mapping missing runtime chunks and ask-answer split tests, 405 passed.
- `rtk cargo test -p velvet_ballastics --test admission_evidence_integration` — PASS, 8 passed.
- `rtk cargo test -p vb_runtime --lib ask_answer` — PASS, 19 passed.
- `rtk cargo test -p vb_runtime --lib red_ask_answer_secret` — PASS, 1 passed.
- `rtk cargo test -p vb_ipc answer` — PASS, 13 passed.
- `rtk cargo test -p vb_ipc answer_ask_taint` — PASS, 4 passed.
- `moon ci` — PASS, 19 tasks completed.

## Notes

- `vb-qi37.4.5` include-body layout was not landed.
- `vb-qi37.16.4` runtime error, IPC answer, CLI answer, lifecycle, and tests were mapped onto the strict chunk split.
- No push, main move, close, or workspace forget was performed.
