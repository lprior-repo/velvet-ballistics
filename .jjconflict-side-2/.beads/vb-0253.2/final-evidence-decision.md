# Final Evidence Decision

STATUS: APPROVED

Reason:
- The rejected artifact-only workspace was replaced with a fresh isolated jj workspace at `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-0253-2` and rebased onto `main` commit `5ba93c4ddc9375cd85c1d21d5419202d228a9816`.
- The local Git metadata now resolves `main`, so the previous missing-main rejection no longer applies.
- Scoped `vb_ipc` gates pass after rebase: `rtk cargo check -p vb_ipc`, `rtk cargo test -p vb_ipc` (`628 passed`), `rtk cargo clippy -p vb_ipc --lib -- -D warnings`, and `cargo kani -p vb_ipc --harness kani_ipc_header_decode_valid --quiet`.

Global gate note:
- `moon ci` was rerun with `main` resolvable. It no longer fails from workspace reference hygiene; it fails on pre-existing/out-of-scope global `xtask/src/forbidden_scan.rs` lint/format findings, `crates/vb_storage/tests/recovery_bdd_tests.rs` unused warnings, and unrelated `vb_cli` mode-module/import drift.

Decision:
- APPROVED for bookmark-ready landing of the scoped `vb_ipc` bead changes.
- Stop before merging main.
