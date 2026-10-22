# Bead vb-ja1zd: Fix fuzz-tsan target binary lookup path

## Problem

The `fuzz-tsan` Moon task (`.moon/tasks/all.yml`) hardcodes the binary lookup path as
`fuzz/target/x86_64-unknown-linux-gnu/release/${tgt}`, but this is a pure virtual Cargo
workspace where `cargo fuzz build` outputs binaries to the workspace root `target/`
directory, not to `fuzz/target/`.

Evidence from canonical baseline d20 (vb-2lyiw):
- `.beads/vb-2lyiw/evidence/canonical-frozen-baseline.stdout` reports `TSAN-MISSING: accessor_traversal`
- `target/fuzz-tsan/build.err` proves the build succeeded (15m 21s)
- Actual executable exists at `target/x86_64-unknown-linux-gnu/release/accessor_traversal`
- `fuzz/target/` directory does not exist

## Contract

The `fuzz-tsan` task must resolve the actual Cargo target directory where `cargo fuzz build`
outputs binaries for this pure virtual workspace. Every listed fuzz target that was actually
built must be found at the declared output directory.

Requirements:
1. Unchanged thread sanitizer flags
2. Unchanged `--build-std` flag
3. Unchanged pinned nightly toolchain
4. Unchanged target triple `x86_64-unknown-linux-gnu`
5. Correct binary lookup path matching actual `cargo fuzz build` output

## Fix

Change line 604 of `.moon/tasks/all.yml` from:
```
bin="fuzz/target/x86_64-unknown-linux-gnu/release/${tgt}"
```
to:
```
bin="target/x86_64-unknown-linux-gnu/release/${tgt}"
```

This is a one-line path correction. No build flags, cache policy, or check logic changed.

## Acceptance Criteria

1. Task YAML syntax valid
2. `accessor_traversal` (and any other actually-built TSAN target) found at corrected path
3. Old path `fuzz/target/...` verified absent (not a symlink, not a real directory)
4. Minimal diff: single line changed

## Limits

- Bounded validation only: inspected baseline build output, did not rebuild
- Final canonical TSAN rerun is root-owned after source freeze
- `fuzz-regress` task path not expanded (not the same bug; verified correct)
- Only `accessor_traversal` was built in the baseline; other targets would be built
  and found by the same corrected path when the task runs in full CI