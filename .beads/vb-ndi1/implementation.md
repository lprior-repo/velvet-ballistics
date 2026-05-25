# Implementation: vb-ndi1 — target-cpu=native Build Configuration

## Changes Made

### 1. `Cargo.toml` — opt-in to `profile-rustflags` feature

Added `cargo-features = ["profile-rustflags"]` at the top of `Cargo.toml`. This unstable Cargo feature allows profile-specific `rustflags` to be defined in `.cargo/config.toml`, which is the canonical way to persist `target-cpu` settings per profile in a Cargo workspace.

### 2. `.cargo/config.toml` — maxperf profile rustflags

Added:

```toml
[profile.maxperf]
rustflags = ["-C", "target-cpu=native"]
```

This persists `-C target-cpu=native` for any `cargo build --profile maxperf` invocation, without requiring manual `RUSTFLAGS` environment variable setup.

## Verification

### Evidence 1: `cargo +nightly build --profile maxperf` compiles cleanly

```
$ cargo +nightly build --profile maxperf
   Compiling num-traits v0.2.19
   ...
   Compiling velvet-ballistics-workspace v0.1.0
    Finished `maxperf` profile [optimized] target(s) in 0.34s
```

### Evidence 2: `-C target-cpu=native` confirmed in compiler invocations

```
$ cargo +nightly clean --profile maxperf && cargo +nightly build --profile maxperf -v 2>&1 | grep target-cpu

Running `...rustc --crate-name foldhash ... -C opt-level=3 -C linker-plugin-lto -C codegen-units=1 -C target-cpu=native ...`
Running `...rustc --crate-name winnow ... -C target-cpu=native ...`
Running `...rustc --crate-name arraydeque ... -C opt-level=3 -C linker-plugin-lto -C codegen-units=1 -C target-cpu=native ...`
```

Every crate in the maxperf build chain receives `-C target-cpu=native`.

## Native CPU Optimization Approach

`target-cpu=native` tells LLVM to generate code optimized for the CPU on which the compilation is performed. On modern x86_64 Linux this enables:

- **SIMD vectorization** — auto-vectorization of loops using AVX2/AVX-512 registers and instructions available on the host
- **CPU-specific instruction selection** — use of BMI2, FMA, POPCNT, LZCNT, and other extension instruction sets present on the host
- **Optimal memory alignment** — struct/layout decisions tuned to the host's cache line size and memory subsystem
- **Branch prediction hints** — architecture-specific hint instructions for better pipelining

Combined with the existing `maxperf` profile settings (`lto = "fat"`, `codegen-units = 1`), `target-cpu=native` completes the maxperf optimization trinity:

| Setting | Value | Effect |
|---------|-------|--------|
| `lto = "fat"` | Whole-program LTO | Cross-crate inlining + dataflow optimizations |
| `codegen-units = 1` | Single codegen unit | Better IPO, no ThinLTO splitting overhead |
| `target-cpu = "native"` | Native CPU instructions | SIMD + CPU-extensions for host |

## Files Changed

- `Cargo.toml` — added `cargo-features = ["profile-rustflags"]`
- `.cargo/config.toml` — added `[profile.maxperf]` with `rustflags = ["-C", "target-cpu=native"]`
