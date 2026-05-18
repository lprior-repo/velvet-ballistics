# vb-kyyf State 14 mutants TMPDIR repair report

STATUS: REJECTED

## Workspace

- Isolated workspace: `/home/lewis/src/bd-vb-kyyf-bdd`
- Source checkout `/home/lewis/src/velvet-ballistics`: not touched.
- Manifest: `.beads/vb-kyyf/dispatch-state14-mutants-tmpdir-repair-attempt1.json`

## Environment repair performed

- Removed workspace-local `.tmp` to eliminate prior recursive cargo-mutants temp state.
- Removed/recreated external temp directory `/tmp/opencode/vb-kyyf-moon-ci-tmp`.
- Ran required gates with `TMPDIR=/tmp/opencode/vb-kyyf-moon-ci-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0`.

## Command evidence

1. `pwd -P`
   - Exit: 0
   - Output: `/home/lewis/src/bd-vb-kyyf-bdd`

2. `rtk ls -ld /tmp/opencode`
   - Exit: 0
   - Output: `/tmp/opencode/`

3. `rm -rf .tmp /tmp/opencode/vb-kyyf-moon-ci-tmp && mkdir -p /tmp/opencode/vb-kyyf-moon-ci-tmp`
   - Exit: 0

4. `TMPDIR=/tmp/opencode/vb-kyyf-moon-ci-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --check`
   - Exit: 0

5. `TMPDIR=/tmp/opencode/vb-kyyf-moon-ci-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 moon ci`
   - Exit: non-zero
   - `mutants-smoke` no longer failed with workspace-local recursive `File name too long`.
   - Remaining blocker:
     ```text
     velvet-ballastics:mutants-smoke | Error: Failed to copy /home/lewis/src/bd-vb-kyyf-bdd/target-test/debug/deps/libenum_dispatch-3f1f56c5aae4b342.so to /tmp/opencode/vb-kyyf-moon-ci-tmp/cargo-mutants-bd-vb-kyyf-bdd-WdAqSX.tmp/target-test/debug/deps/libenum_dispatch-3f1f56c5aae4b342.so
     velvet-ballastics:mutants-smoke | Caused by:
     velvet-ballastics:mutants-smoke |     Disk quota exceeded (os error 122)
     ```

## Decision

- TMPDIR recursion repair: effective.
- Global `moon ci`: failed due external temp disk quota, not due `File name too long` recursion.
- vb-kyyf State 14 landing can rerun only after the `/tmp/opencode` quota/free-space blocker is cleared or a larger external TMPDIR is provided.
