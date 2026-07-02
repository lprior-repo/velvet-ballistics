STATUS: LANDED

Landing evidence:
- `jj git fetch` updated remote tracking main.
- `jj rebase -r @ -d main@origin` rebased repair onto remote main.
- `moon run :verify-standard` PASS after final rebase.
- `jj git push --bookmark main` moved remote `main` to `b14d0de7`.
- `jj log -r main@origin --no-graph` shows `b14d0de7 fix(vb_storage): handle Kani fallible matches`.
- `bd close vb-ybi5 --reason ...` succeeded from source checkout after isolated bd lookup lacked the target issue.
- `bd dolt push` succeeded.

Moon CI note: attempted before landing and failed on unrelated global fmt/check debt, recorded as DEFERRED_GLOBAL in regression-diff. Bead acceptance gate `verify-standard` passed.
