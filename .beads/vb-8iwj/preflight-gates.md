bead_id: vb-8iwj
phase: State 15 non-landing preflight gates
updated_at: 2026-05-11T00:00:00Z

# Preflight Gate Evidence

STATUS: LANDING_BLOCKED

## Workspace

```text
/home/lewis/src/Velvet-ballistics-vb-8iwj-wave3-preflight
zmryxnnv e3b5bb45 (empty) vb-8iwj: run wave 3 landing preflight
parent: tqypyqys 57f44923 vb-8iwj: integrate wave 3 CLI workspaces
```

## Commands run

### `moon run :quick`

Result: PASS.

```text
Tasks: 1 completed
Time: 1s 344ms
```

### `moon run :test`

First bounded attempt: timed out at 300s while `velvet-ballistics:test` was still running.

Retry with 600s bound: PASS.

```text
velvet-ballistics:test | Summary [57.732s] 9863 tests run: 9863 passed, 0 skipped
Tasks: 4 completed (1 cached)
Time: 1m 8s 549ms
```

### `moon ci`

Bounded run: completed non-zero in 3m 10s 873ms.

Full output:

```text
/home/lewis/.local/share/opencode/tool-output/tool_e1827fd690013B19rlGHjOY9zu
```

Summary:

```text
Tasks: 13 completed (1 cached), 3 failed, 3 skipped
```

Primary failures match existing `vb-w823` repo-wide global debt:

- `velvet-ballistics:fmt` diffs in `crates/vb_proof_kernels/src/envelope_header.rs`, `lib.rs`, `step_state.rs`, `taint.rs`.
- `velvet-ballistics:fmt` diffs in `crates/vb_storage/src/codec_miri_tests.rs`, `kani_codec.rs`, `lib.rs`.
- `velvet-ballistics:fmt` diffs in `fuzz/fuzz_targets/decode_record.rs`, `fuzz/fuzz_targets/lex_expr.rs`.
- `velvet-ballistics:fmt` diffs in `xtask/src/main.rs`, `xtask/src/proof.rs`.
- `velvet-ballistics:lint-src` clippy `new_without_default` for `crates/vb_proof_kernels/src/envelope_header.rs:26:5`.

Classification: `DEFERRED_GLOBAL`, not bead-local, because these files are outside the Wave 3 CLI integration delivery scope and are already tracked by `vb-w823`.

## State 15 decision

State 15 remains `LANDING_BLOCKED` because source landing/push/bookmark policy is still missing and original workspaces must not be forgotten.
