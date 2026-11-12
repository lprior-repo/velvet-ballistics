---
section: 4
title: "Mandatory Rust Tooling"
parent: velvet-ballistics-MASTER.md
---

## 4. Mandatory Rust Tooling


| Tool | Required use |
|------|--------------|
| `rustup` nightly | Only supported toolchain for first-party builds. |
| `rustfmt` | Formatting gate. |
| `clippy` | Hard deny lint gate. |
| `cargo-nextest` | Primary test runner. |
| `miri` | Pure crates: `vb_core`, `vb_expr`, `vb_compile`. |
| `criterion` | Local statistical benchmarks. |
| `iai-callgrind` | Instruction/cache benchmark gates. |
| `proptest` | Property and invariant tests. |
| `cargo-fuzz` | Parser, decoder, and IR fuzzing. |
| `trybuild` | Compile-fail tests for public macro/schema contracts when such contracts are active. Generated Rust compile-fail testing is removed with codegen. |
| `cargo-audit` | Advisory vulnerability report; non-blocking under the owner waiver unless a bead opts in. |
| `cargo-deny` | Advisory license, duplicate, source, and advisory report; non-blocking under the owner waiver unless a bead opts in. |
| `cargo-vet` | Advisory supply-chain review report; non-blocking under the owner waiver unless a bead opts in. |
| `cargo-geiger` | Advisory unsafe dependency report; first-party unsafe remains forbidden by lint. |
| `cargo-machete` | Advisory unused dependency report. |
| `cargo-hack` | Feature powerset gate. |
| `cargo-semver-checks` | Advisory public compatibility report for released crates unless an API-stability bead opts in. |
| `cargo-public-api` | Advisory public API diff report unless an API-stability bead opts in. |
| `cargo-bloat` | Size regression investigation. |
| `cargo-mutants` | Mutation testing, at least smoke scope in CI. |
| `cargo-llvm-cov` | Coverage report gate. |
| `cargo-insta` | Golden diagnostics only when approved by a bead. |
| `flamegraph` | Local profiling. |
| `samply` or `perf` | CPU profiling on Linux/native hosts. |
| `hyperfine` | CLI/end-to-end timing harness. |
| `valgrind` tools | `callgrind`, `cachegrind`, and `DHAT` investigation where available. |
| `moon` | CI orchestration gate; every mandatory command must be represented as a Moon task before release. |

Mandatory tooling categories:

- Formatting/linting: `cargo fmt`, hard-deny `clippy`, warnings as errors, banned-token scan.
- Test runners: `cargo test`, `cargo nextest`, `miri`, `cargo mutants`, `cargo llvm-cov`.
- Property/fuzz/compile diagnostics: `proptest`, `cargo-fuzz`, `arbitrary`, `trybuild` where active compile-fail contracts exist, and `insta` only when approved for golden diagnostics.
- Feature matrix: `cargo hack`.
- Advisory dependency/API reports: `cargo audit`, `cargo deny`, `cargo vet`, `cargo geiger`, `cargo machete`, `cargo semver-checks`, `cargo public-api`, and `cargo bloat`; these are non-blocking under the 2026-05-23 owner waiver unless a bead explicitly opts in.
- Performance: `criterion`, `iai-callgrind`, `flamegraph`, `samply`/`perf`, `hyperfine`, `callgrind`, `cachegrind`, and `DHAT` for current-scope IR-interpreter evidence. PGO, `target-cpu=native`, and maxperf release workflows are removed.
- Nightly/dynamic verification: Miri, sanitizers, and coverage.

Bootstrap install block:

```bash
cargo install cargo-nextest cargo-audit cargo-deny cargo-vet cargo-geiger cargo-machete cargo-hack cargo-semver-checks cargo-public-api cargo-bloat cargo-mutants cargo-llvm-cov cargo-insta cargo-fuzz flamegraph hyperfine iai-callgrind-runner
```

`rust-toolchain.toml` contract:

```toml
[toolchain]
channel = "nightly-2026-04-28"
profile = "minimal"
components = ["rustfmt", "clippy", "rust-src", "miri", "llvm-tools-preview"]
targets = ["x86_64-unknown-linux-gnu"]
```

MSRV distinction: do not hardcode `rust-version = "1.91"` or any stable MSRV unless verified against actual stable support. The nightly pin controls builds today; a stable MSRV is a separate release promise and must be established by evidence.

Strict nightly governance:

- Nightly is mandatory.
- Unstable features are allowlisted; arbitrary `#![feature]` is rejected.
- `RUSTC_BOOTSTRAP` is rejected in developer shells, CI, scripts, and docs.
- CI must include a first-party source check equivalent to `moon run :nightly-feature-gate`; a strict Cargo `-Zallow-features=try_blocks,portable_simd,allocator_api,generic_const_exprs` probe is required where transitive dependency feature attributes permit it. A represented task or probe is not proof that all governance policies passed; reports must state the actual command outcome.
- Normal source-allowed features: `try_blocks`, `portable_simd`.
- Perf-only features: `allocator_api`, `generic_const_exprs`, restricted to `crates/*/src/perf/**`, `crates/*/src/generated/**`, `benches/**`, or a file carrying `velvet-allow-perf-nightly-feature` if the feature-gate script implements that marker exception.
- Detailed operational policy lives in `docs/rust-governance.md` and is subordinate to this master contract.

---
