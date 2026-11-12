# Baseline Report — vb-2b4g

## Workspace

- Source checkout: `/home/lewis/src/velvet-ballistics`
- Isolated workspace: `/tmp/opencode/go-skill-vb-2b4g`
- Parent change: `xxoyykps ab1117de` from `vb-qi37.10` verified fail-closed state

## Bead State

- `vb-2b4g` is claimed and `in_progress`.
- It blocks `vb-qi37.10`.

## Mechanical Baseline

The current baseline is intentionally fail-closed for the target families:

- `rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture` — PASS, 2 passed / 359 filtered.
- `rtk cargo test -p vb_codegen reduce_generated_parity -- --nocapture` — PASS, 1 passed / 360 filtered.
- `rtk cargo test -p vb_codegen together_generated_parity -- --nocapture` — PASS, 1 passed / 360 filtered.
- `rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture` — PASS, 1 passed / 360 filtered.

## Baseline Meaning

Passing baseline tests are not parity evidence. They prove the repaired `vb-qi37.10` state rejects unsupported target families before source emission. `vb-2b4g` must replace this with real generated-vs-runtime parity evidence.
