# vb-2b4g Go-Skill State

Bead: `vb-2b4g` - codegen/runtime: Implement Repeat Reduce Together Collect parity

Source checkout: `/home/lewis/src/velvet-ballistics`

Isolated workspace: `/tmp/opencode/go-skill-vb-2b4g`

Parent workspace/change: `/tmp/opencode/go-skill-vb-qi37-10` at `xxoyykps ab1117de`, so this work builds on the verified fail-closed `vb-qi37.10` state.

Current state: State 1 complete, implementation/exploration next

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

Next gate:

- Explore runtime/core semantics and route Rust implementation/test repair through `holzman-rust`.
