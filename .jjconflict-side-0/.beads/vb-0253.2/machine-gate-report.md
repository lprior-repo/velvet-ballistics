# Machine Gate Report

STATUS: APPROVED_WITH_GLOBAL_DEBT

Workspace/reference hygiene:
- Fresh isolated workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-0253-2`.
- Rebased parent: `main` commit `5ba93c4ddc9375cd85c1d21d5419202d228a9816`.
- Local Git `main` ref resolves for Moon; previous missing-main failure repaired.

Scoped bead gates passed after rebase:
- `rtk cargo check -p vb_ipc` -> PASS.
- `rtk cargo test -p vb_ipc` -> PASS, `628 passed`.
- `rtk cargo clippy -p vb_ipc --lib -- -D warnings` -> PASS.
- `cargo kani -p vb_ipc --harness kani_ipc_header_decode_valid --quiet` -> PASS.
- Duplicate owner grep: exactly six canonical definitions in `bounded.rs`, `error.rs`, and `ingress.rs`.
- `rtk wc -l crates/vb_ipc/src/lib.rs` -> `58`.

Canonical gate rerun:
- `moon ci` -> FAIL_GLOBAL, not FAIL_ENV.
- Failure categories: out-of-scope `xtask/src/forbidden_scan.rs` format/lint debt, out-of-scope `crates/vb_storage/tests/recovery_bdd_tests.rs` unused warnings, and unrelated `vb_cli` mode-module/import drift.

Decision: scoped bead is bookmark-ready; global debt is not introduced by `vb_ipc` changes. Stop before merging main.
