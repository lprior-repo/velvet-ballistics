# Machine Gate Report — vb-f7k6 State 11 Retry

STATUS: APPROVED

## Gates Run

### Cargo check

- command: `/usr/bin/env cargo check --workspace --all-targets --all-features`
- exit: 0
- result: PASS
- evidence: finished `dev` profile; checked `vb_runtime`, `vb_ipc`, `vb_storage`, `vb_codegen`, `vb_cli`, fuzz, and workspace test crates.

### Canonical CI

- command: `/usr/bin/env moon ci`
- exit: 0
- result: PASS
- evidence: `Tasks: 23 completed`; `Time: 36s 977ms`.

## Classification

- classification: PASS
- baseline: `.beads/vb-f7k6/baseline-report.md` records shared-parent `moon ci` exit 0 with `Tasks: 23 completed`.
- current result: canonical CI remains clean after lint test repair.
