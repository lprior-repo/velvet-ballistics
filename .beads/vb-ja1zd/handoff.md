# Bead vb-ja1zd Handoff

## Status: READY FOR REVIEW

Three-lane fix applied to `.moon/tasks/all.yml` covering all three fuzz tasks.

## What was changed

Three tasks modified, 5 lines changed total:

```diff
--- fuzz-smoke (lines 535, 545) ---
- cargo fuzz build --target x86_64-unknown-linux-gnu
+ cargo fuzz build --target x86_64-unknown-linux-gnu --target-dir "$PWD/target/fuzz-asan/build"
- bin_dir="fuzz/target/x86_64-unknown-linux-gnu/release"
+ bin_dir="target/fuzz-asan/build/x86_64-unknown-linux-gnu/release"

--- fuzz-tsan (lines 593-596, 604) ---
+ cargo fuzz build ... --target-dir "$PWD/target/fuzz-tsan/build"
- bin="target/x86_64-unknown-linux-gnu/release/${tgt}"
+ bin="target/fuzz-tsan/build/x86_64-unknown-linux-gnu/release/${tgt}"

--- fuzz-regress (lines 631, 639) ---
- cargo fuzz build --target x86_64-unknown-linux-gnu
+ cargo fuzz build --target x86_64-unknown-linux-gnu --target-dir "$PWD/target/fuzz-asan/build"
- bin="fuzz/target/x86_64-unknown-linux-gnu/release/${tgt}"
+ bin="target/fuzz-asan/build/x86_64-unknown-linux-gnu/release/${tgt}"
```

## Why

- `fuzz-smoke` and `fuzz-regress` used `fuzz/target/` which doesn't exist in this pure virtual workspace (cargo-fuzz outputs to workspace root `target/`)
- `fuzz-tsan` used generic `target/` which races with ASAN builds when tasks run in parallel or with overlapping build state
- Explicit `--target-dir` on all three tasks ensures isolation and correct binary discovery

## Verification performed

1. Shell syntax validated for all three tasks via `bash -n`
2. Path consistency verified: each task's build output dir matches its binary lookup dir
3. Cross-task consistency verified: ASAN tasks share `target/fuzz-asan/build`, TSAN uses separate `target/fuzz-tsan/build`
4. `cargo fuzz list` returns 93 targets, confirming workspace is intact
5. YAML structure and indentation preserved

## Remaining work (root-owned)

- Final canonical ASAN+TSAN rebuild + run from source-frozen commit
- Full `moon ci` gate
- Fresh DeepSeek review of implementation

## No other changes

No product source, tests, or other task configurations modified. No sanitizer flags, build flags, or limits changed. Pure path contract fix.