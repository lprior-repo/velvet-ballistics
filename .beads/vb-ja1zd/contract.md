# Bead vb-ja1zd: Fix all three fuzz task binary lookup paths

## Problem

Three Moon fuzz tasks in `.moon/tasks/all.yml` used incorrect or race-prone binary lookup paths:

1. **fuzz-smoke**: Used `fuzz/target/x86_64-unknown-linux-gnu/release/` (wrong; pure virtual workspace outputs to `target/`)
2. **fuzz-tsan**: Used `target/x86_64-unknown-linux-gnu/release/` (generic target dir, races with ASAN builds)
3. **fuzz-regress**: Used `fuzz/target/x86_64-unknown-linux-gnu/release/` (wrong; same bug as fuzz-smoke)

All three tasks built without `--target-dir`, meaning builds shared the default `target/` directory. When ASAN and TSAN tasks ran in parallel (or sequentially with overlapping build state), they could check the wrong binary or miss binaries entirely.

## Contract

All three fuzz tasks must:
1. Use explicit `--target-dir` build flag
2. Use consistent executable lookup path matching the build output
3. ASAN tasks (fuzz-smoke, fuzz-regress) share one target directory
4. TSAN task (fuzz-tsan) uses a separate target directory to avoid race conditions

Requirements:
- Unchanged sanitizer flags (`--sanitizer thread` for TSAN, default ASAN for others)
- Unchanged `--build-std` flag for TSAN
- Unchanged pinned nightly toolchain
- Unchanged target triple `x86_64-unknown-linux-gnu`
- No disabled builds, no skipped targets, no fake binaries

## Fix Applied

Three-lane path correction:

| Task | Build --target-dir | Binary lookup path |
|------|-------------------|-------------------|
| fuzz-smoke | `$PWD/target/fuzz-asan/build` | `target/fuzz-asan/build/x86_64-unknown-linux-gnu/release` |
| fuzz-tsan | `$PWD/target/fuzz-tsan/build` | `target/fuzz-tsan/build/x86_64-unknown-linux-gnu/release/${tgt}` |
| fuzz-regress | `$PWD/target/fuzz-asan/build` | `target/fuzz-asan/build/x86_64-unknown-linux-gnu/release/${tgt}` |

## Verification Performed

1. Shell syntax validated for all three tasks via `bash -n`
2. Path consistency verified: build output dir matches binary lookup dir
3. Cross-task consistency: ASAN tasks share dir, TSAN uses separate dir
4. Actual fuzz targets listed via `cargo fuzz list` (93 targets)
5. YAML structure preserved

## Remaining Work (root-owned)

- Final canonical ASAN+TSAN rebuild + run from source-frozen commit
- Full `moon ci` gate

## Scope

Configuration fix only. No runtime implementation changes. No sanitizer flag changes. No build/skip policy changes.