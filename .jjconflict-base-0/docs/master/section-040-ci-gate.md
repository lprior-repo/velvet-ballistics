---
section: 40
title: "CI Gate"
parent: velvet-ballistics-MASTER.md
---

## 40. CI Gate


Required Moon tasks:

```text
check
test
feature-powerset
miri
coverage
mutants-smoke
bench-build
benchmark-regression-policy
source-length
fuzz-smoke
```

CI must gate on `moon ci`, whose pipeline must include `check`, `test`, `fuzz-smoke`, `miri`, `coverage`, `mutants-smoke`, `bench-build`, `benchmark-regression-policy`, `source-length`, and `feature-powerset`. Nightly sanitizer jobs are required for runtime, IPC, storage, and binary decoding crates. The `source-length` task must fail any hot runtime function over 25 logical lines. Advisory supply-chain reporting may exist as a Moon task, but supply-chain/advisory report warnings are non-blocking under the 2026-05-23 owner waiver unless a future bead explicitly opts in.

Mandatory CI commands:

```bash
cargo +nightly fmt --all -- --check
cargo +nightly clippy --workspace --all-targets --all-features -- \
  -D warnings \
  -D clippy::unwrap_used -D clippy::expect_used \
  -D clippy::panic -D clippy::panic_in_result_fn \
  -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro \
  -D clippy::indexing_slicing -D clippy::string_slice \
  -D clippy::get_unwrap -D clippy::arithmetic_side_effects \
  -D clippy::as_conversions -D clippy::let_underscore_must_use
cargo +nightly nextest run --workspace --all-features
cargo +nightly test --doc --workspace --all-features
cargo +nightly doc --workspace --all-features --no-deps
cargo +nightly miri test -p vb_core -p vb_expr -p vb_compile
cargo +nightly bench --no-run
cargo hack check --feature-powerset --workspace
cargo llvm-cov --workspace --all-features
cargo mutants --in-place --timeout 60 --package vb_core
cargo fuzz build
```

Advisory report commands, non-blocking under the owner waiver unless a bead opts in:

```bash
cargo audit
cargo deny check
cargo vet
cargo geiger
cargo machete
cargo semver-checks check-release
cargo public-api diff
cargo bloat --release --crates
```

Moon expectation: each mandatory command above must have a Moon task before release, and the release gate must run through Moon rather than a hand-maintained shell script. Advisory report tasks may run in Moon, but warnings from those reports cannot block current release closure without a bead-specific opt-in.

---
