bead_id: vb-qi37.16.2
phase: state-8
updated_at: 2026-05-11T22:18:00Z

# State 8 Moon CI Block

STATUS: BLOCKED

Retry classes:

- BLOCK_LOCAL: formatting diffs in bead-scoped files after State 6/rebase repair.
- BLOCK_REGRESSION: unresolved conflict marker remained in `xtask/src/main.rs`; duplicate `Default` implementation appeared in `vb_proof_kernels::EnvelopeHeader` after rebase/conflict integration.

## Context

State 6 black-hat repair was completed by holzman-rust and wrote `state-6-blackhat-repair.md` with `STATUS: REPAIRED`.

The bead was then rebased onto local bookmark `go/vb-jkrk-global-ci` (`ylnywtnm/326d2579`). Conflicts were resolved by holzman-rust and `state-8-rebase-conflict-repair.md` was written, but the full release gate found remaining conflict/format defects.

## Command

```bash
moon ci
```

Full output capture:

```text
/home/lewis/.local/share/opencode/tool-output/tool_e1925108f001uiDbGKJNl2l8Cr
```

## Evidence

Observed failing tasks:

- `velvet-ballistics:fmt`: rustfmt diffs in scoped resume files:
  - `crates/vb_runtime/src/shard/lifecycle.rs`
  - `crates/vb_runtime/tests/durable_resume_red_phase.rs`
- `velvet-ballistics:fmt`: `xtask/src/main.rs` contains an unclosed delimiter and leftover conflict marker at line 787: `>>>>>>> conflict 2 of 2 ends`.
- `velvet-ballistics:lint-src` / `check`: `crates/vb_proof_kernels/src/envelope_header.rs` has conflicting `Default` implementations: `#[derive(Default)]` plus manual `impl Default`.

Passing evidence before the full release gate:

```text
rtk cargo test --package vb_runtime --test durable_resume_red_phase -> 17 passed
rtk cargo check --package vb_runtime -> Finished dev profile
moon run :quick -> Tasks: 1 completed
```

## Required next action

Route to `holzman-rust` for State 8 repair, not State 6: source behavior passed focused tests, but rebase/conflict integration left format/compile blockers.
