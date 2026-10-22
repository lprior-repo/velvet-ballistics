# Bead vb-ja1zd Handoff

## Status: READY FOR REVIEW

Single-line fix applied to `.moon/tasks/all.yml` line 604.

## What was changed

```diff
-        bin="fuzz/target/x86_64-unknown-linux-gnu/release/${tgt}"
+        bin="target/x86_64-unknown-linux-gnu/release/${tgt}"
```

## Why

Pure virtual Cargo workspace outputs `cargo fuzz build` binaries to `target/`, not `fuzz/target/`.
The baseline d20 CI run built all TSAN fuzz targets successfully (15m 21s), but the `fuzz-tsan`
task's executable-lookup loop used the wrong path and reported `TSAN-MISSING: accessor_traversal`
(and all other targets) as failures.

## Verification performed

1. Confirmed actual executable exists at `target/x86_64-unknown-linux-gnu/release/accessor_traversal` (ELF binary, 1.3MB)
2. Confirmed old path `fuzz/target/` does not exist at all
3. Confirmed build actually succeeded (build.err shows `Finished ... in 15m 21s`)
4. Validated task YAML structure preserved (indentation, shell quoting)
5. Git diff shows exactly one line changed

## Remaining work (root-owned)

- Final canonical TSAN rebuild + run from source-frozen commit (15m cold build)
- Full `moon ci` gate

## No other changes

No product source, tests, or other task configurations modified. `fuzz-regress` task path
not changed (not the same bug; it already uses the correct workspace target path).