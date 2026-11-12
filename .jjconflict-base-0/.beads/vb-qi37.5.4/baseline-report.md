# baseline-report.md — vb-qi37.5.4

## Build Baseline (pre-edit state)

**Command**: `cargo build --release 2>&1 | tail -20`
**Exit code**: non-zero (build failed)
**Date**: captured at State 1

```
   Compiling serde-saphyr v0.0.25
   Compiling postcard v1.1.3
   Compiling vb_core v0.1.0 (/home/lewis/src/vb-qi37-5-4/crates/vb_core)
   Compiling vb_validate v0.1.0 (/home/lewis/src/vb-qi37-5-4/crates/vb_validate)
   Compiling vb_storage v0.1.0 (/home/lewis/src/vb-qi37-5-4/crates/vb_storage)
   Compiling vb_codegen v0.1.0 (/home/lewis/src/vb-qi37-5-4/crates/vb_codegen)
   Compiling vb_expr v0.1.0 (/home/lewis/src/vb-qi37-5-4/crates/vb_expr)
   Compiling vb_compile v0.1.0 (/home/lewis/src/vb-qi37-5-4/crates/vb_compile)
   Compiling vb_runtime v0.1.0 (/home/lewis/src/vb-qi37-5-4/crates/vb_runtime)
warning: ignoring -C extra-filename flag due to -o flag

error: couldn't read `crates/vb_runtime/src/runtime/chunk_001.rs`: No such file or directory (os error 2)
 --> crates/vb_runtime/src/runtime.rs:4:1
  |
4 | include!("runtime/chunk_001.rs");
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: `vb_runtime` (lib) generated 1 warning
error: could not compile `vb_runtime` (lib) due to 1 previous error; 1 warning emitted
warning: build failed, waiting for other jobs to finish...
```

## Baseline Status

- **build_success**: false
- **missing_file**: crates/vb_runtime/src/runtime/chunk_001.rs
- **note**: This appears to be pre-existing baseline state. The bead vb-qi37.5.4 is about "verifier: Idempotency gate evidence tests" — this is a test/infrastructure bead, not a code implementation bead. The missing file may be intentional for the test scenario or part of the verification scope.
