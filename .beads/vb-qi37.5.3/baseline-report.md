# Baseline Report — vb-qi37.5.3

**Captured at**: 2026-05-14T00:00:00Z
**Source checkout**: /home/lewis/src/Velvet-ballistics
**Isolated workspace**: /home/lewis/src/vb-qi37-5-3
**Commit**: ffbe7f5cd (main)
**Build command**: `cargo build 2>&1 | tail -40`

## Pre-Edit Baseline Status

The baseline build **FAILS** with a missing file error.

```
error: couldn't read `crates/vb_runtime/src/runtime/chunk_001.rs`: No such file or directory (os error 2)
 --> crates/vb_runtime/src/runtime.rs:4:1
  |
4 | include!("runtime/chunk_001.rs");
  |
  = couldn't compile `vb_runtime` (lib) due to 1 previous error
```

## Crates in workspace

- vb_core
- vb_storage
- vb_validate
- vb_codegen
- vb_expr
- vb_compile
- vb_runtime (failing — missing chunk_001.rs)
- vb_yaml

## Baseline Gate

The baseline is NOT clean. This is a pre-existing issue at commit `ffbe7f5cd`.
This is the starting point for vb-qi37.5.3. Any regressions introduced by this bead
must be distinguished from this pre-existing failure.
