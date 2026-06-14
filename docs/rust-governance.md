# Rust Governance

`velvet-ballistics` uses pinned nightly Rust for bounded, explicit performance and verification work. The pin is `nightly-2026-04-28` in `rust-toolchain.toml` and must be used for first-party builds.

## Nightly Feature Gate

Allowed unstable features are intentionally narrow:

| Tier | Features | Rule |
|------|----------|------|
| Normal | `try_blocks`, `portable_simd` | May be used in first-party crates when the code stays safe, bounded, and justified by the implementation need. `portable_simd` still requires benchmark evidence for performance claims. |
| Perf-only | `allocator_api`, `generic_const_exprs` | May appear only in `crates/*/src/perf/**`, `crates/*/src/generated/**`, `benches/**`, or another first-party Rust file carrying the marker text `velvet-allow-perf-nightly-feature` if the feature-gate script implements that marker exception. |

First-party mechanical gate:

```bash
moon run :nightly-feature-gate
```

Strict Cargo probe, where transitive dependencies do not require their own nightly internals:

```bash
cargo +nightly-2026-04-28 -Zallow-features=try_blocks,portable_simd,allocator_api,generic_const_exprs check --workspace --all-targets --all-features
```

Use `moon run :nightly-feature-gate` for the first-party source gate instead of making every local Cargo command fragile. This gate is the mechanical source-policy check for unstable features; it is not evidence that every governance policy below has been enforced. Use `moon run :nightly-feature-cargo-probe` as a stricter compatibility probe when dependency feature attributes permit it.

`RUSTC_BOOTSTRAP` is prohibited. Adding any other `#![feature(...)]` requires a bead, master contract update, governance update, and a passing feature gate.

## Safe SIMD Policy

`portable_simd` is allowed only through safe Rust APIs. First-party unsafe SIMD, inline assembly, target-feature unsafe calls, raw pointer vector loads, or unchecked alignment assumptions are rejected.

SIMD work must provide:

- Scalar fallback or a documented platform limit.
- Fixed lane counts or bounded lane selection.
- Checked slice lengths before vector loads or stores.
- Benchmark evidence against the scalar baseline.
- Tests that cover tails, empty input, one-lane-short input, and misaligned offsets when applicable.

## Unsafe And Panic Policy

Runtime, core, storage, IPC, generated workflow, and other production first-party implementations must not use `unsafe`, `.unwrap()`, `.expect()`, `panic!`, `todo!`, `unimplemented!`, or `dbg!`. Runtime/core/generated code has a zero-unsafe policy. Fuzz scaffolding should also be safe first-party Rust; if an FFI boundary ever requires `unsafe`, it must be isolated behind a narrow audited exception naming the file, reason, invariants, and owner. Do not describe fuzz unsafe as governed unless that exception is documented and mechanically covered.

Third-party unsafe is allowed only when the dependency is pinned, audited, justified, and covered by the dependency policy plus `cargo audit`, `cargo deny`, `cargo vet`, and `cargo geiger`.

## Error And Bounds Policy

All recoverable failures use typed railway errors: parse, validation, compile, runtime, IPC, storage, and tooling errors must return explicit `Result` types. User input and external state must not trigger panics.

Holzmann bounds are mandatory:

- Bounded queues, loops, fanout, retries, buffers, expression stacks, IPC frames, and persistence batches.
- Checked indexing, slicing, casts, arithmetic, lengths, and capacities.
- No hidden allocation growth in hot loops unless the bound is explicit and tested.
- No ignored `Result` or fallible return value.

## Profiles

`release` is optimized for normal release builds. `hardened` inherits release settings but enables debug assertions, overflow checks, and debug info for verification builds.

# allow-removed-feature: master §41 — rust-governance policy statement enumerates the removed tokens
`maxperf`, PGO, generated Rust execution, and `target-cpu=native` workflows are deferred from the current Backend / IR Interpreter Complete milestone. They must not be current release gates or performance evidence unless a future architecture bead reactivates them.

## Required Gates

Routine acceptance uses:

```bash
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Governance and deeper verification are represented as Moon tasks. A represented task/probe is not the same as a passing gate; implementation reports must say whether each task was actually executed and passed, skipped, run with `--no-actions`, or left as a placeholder.

- `nightly-feature-gate`: mechanically checks the first-party unstable feature whitelist and the perf-only feature scope implemented by the script.
- `nightly-feature-cargo-probe`: attempts the strict Cargo `-Zallow-features` check where transitive dependency feature attributes permit it.
- `hardened-build`: builds with release-like optimization plus debug assertions and overflow checks.
- `miri`: runs Miri tests on the pinned nightly.
- `fuzz-smoke`: builds fuzz targets.
- `mutants-smoke`: runs bounded mutation testing.
- `sanitizer-address-check`: compiles tests with AddressSanitizer instrumentation where the host supports it.
- `bench-build`: compiles benchmarks.
- `benchmark-regression-policy`: validates `contracts/perf-budget.yaml` and `evidence/benchmark-evidence.jsonl` so every speed claim has baseline/result/raw-log evidence, explicit thresholds, and current-milestone `ir-interpreter` scope.
- `benchmark-proof`: records a Criterion baseline named `vb-current` when real benchmarks exist; it is not itself acceptance evidence until paired with `benchmark-regression-policy` metadata.
- Deferred PGO probes may exist for future research, but they are not current release gates and do not prove current IR-interpreter performance.

## Performance Crate And Tool Policy

Performance crates are allowed only when they beat simple first-party code or provide audited primitives that cannot be maintained locally. Each addition must name the hot path, the alternative considered, the expected resource bound, and the benchmark that proves value.

Performance tools must measure the claimed behavior. Use Criterion for statistical microbenchmarks, `iai-callgrind` or Valgrind for instruction/cache evidence, `perf` or `samply` for CPU profiles, and `cargo bloat` for size investigation. PGO evidence is future-scope unless reactivated by a dedicated architecture bead.

Current Criterion scaffold benchmarks that only compile placeholder harnesses are compileability checks, not performance evidence. They cannot justify latency, throughput, allocation, instruction-count, or maxperf claims.

## Benchmark-Proof Optimization

No optimization lands on assertion alone. Every speed claim needs:

- Baseline command and result.
- Optimized command and result.
- Input shape and workload size.
- Host CPU and relevant target flags.
- Regression threshold or explicit reason no threshold exists yet.

If a change cannot be measured yet, describe it as a cleanup or enabling change, not as faster.

## AI-Agent Rules

Agents must keep changes mechanical, minimal, bounded, and explicit. Do not use functional-Rust rewrites for this repository unless the user explicitly changes governance. Do not add unstable features outside the whitelist and documented scope. Do not add first-party unsafe or panic APIs. Do not add dependencies without the dependency policy review. Do not claim performance wins without real baseline/result benchmark evidence.
