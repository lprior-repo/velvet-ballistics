# vb-2b4g Go-Skill State

Bead: `vb-2b4g` - codegen/runtime: Implement Repeat Reduce Together Collect parity

Source checkout: `/home/lewis/src/velvet-ballistics`

Isolated workspace: `/tmp/opencode/go-skill-vb-2b4g`

Parent workspace/change: `/tmp/opencode/go-skill-vb-qi37-10` at `xxoyykps ab1117de`, so this work builds on the verified fail-closed `vb-qi37.10` state.

Current state: State 15 complete; scoped landing artifacts written, bead closed, Dolt pushed, and jj bookmark pushed to origin.

Retry attempt: 0

Claim status: `bd --db "/home/lewis/src/velvet-ballistics/.beads/dolt" update vb-2b4g --claim --json` succeeded.

Path isolation evidence:

- `pwd -P` in isolated workspace returned `/tmp/opencode/go-skill-vb-2b4g`.
- Path guard rejected nesting under `/home/lewis/src/velvet-ballistics` with exit 0.
- `jj status` reports working copy `pqomuxro 29b82f6b` with parent `xxoyykps ab1117de`.

Scope:

- Implement real executable generated-vs-runtime parity for `Repeat*`, `Reduce*`, `Together*`, and `Collect*`.
- Tests must not treat `not_yet_implemented` as pass.
- `Collect*` must include duplicate/stale/multi-page/materialization/capacity/taint/journal parity before support is counted complete.
- Do not weaken `vb-qi37.10` accepted contract/test-plan/traceability.

Baseline evidence:

- `rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture` passed, 2 passed / 359 filtered, because current state is fail-closed.
- `rtk cargo test -p vb_codegen reduce_generated_parity -- --nocapture` passed, 1 passed / 360 filtered, because current state is fail-closed.
- `rtk cargo test -p vb_codegen together_generated_parity -- --nocapture` passed, 1 passed / 360 filtered, because current state is fail-closed.
- `rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture` passed, 1 passed / 360 filtered, because current state is fail-closed.

Final evidence:

- Truth-serum report: `.beads/vb-2b4g/truth-serum-report.md`.
- Final evidence decision: `.beads/vb-2b4g/final-evidence-decision.md`.
- Scoped active-context gates reproduced: repeat/reduce/together/collect parity, generated source contract, journal signature parity, full local `vb_codegen` suite, trybuild, fmt, cargo check, exact PO-007 command, strict production clippy, production assertion scan, forbidden oracle scan, and production placeholder scan.
- Remaining disclosed risk: `moon ci` is `DEFERRED_GLOBAL` due disk quota/resource failures and must be rerun before final release confidence.

Next gate:

- Integration review/merge from remote bookmark `go-skill-vb-2b4g`; resolve `vb-n746` and rerun `moon ci` before release confidence.

Landing evidence:

- Landing report: `.beads/vb-2b4g/landing-report.md`.
- `bd --db "/home/lewis/src/velvet-ballistics/.beads/dolt" close vb-2b4g --reason ...` succeeded.
- `bd --db "/home/lewis/src/velvet-ballistics/.beads/dolt" dolt push` succeeded.
- `jj git push --bookmark go-skill-vb-2b4g` succeeded.
- Remote bookmark verification: `go-skill-vb-2b4g @ origin -> yxnyornz 398a52c2`.
