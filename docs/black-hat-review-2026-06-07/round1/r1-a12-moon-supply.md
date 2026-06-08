# R1-A12: .moon + .cargo + supply-chain Inventory

**Agent:** explore · **Date:** 2026-06-07
**Scope:** `.moon/`, `.cargo/`, `supply-chain/`, root `*.toml` configs

## .moon/ Tasks

**Files:** 50 task definitions across 4 YAML files:
- `.moon/tasks/all.yml` — 43 task definitions
- `.moon/tasks/kani.yml` — 2 task definitions
- `.moon/tasks/verus.yml` — 2 task definitions
- `.moon/tasks/tlc.yml` — 3 task definitions

### Pipeline Tasks (21, in `.moon.yml:7-27`)

| # | Task | Command | runInCI | Genuine? |
|---|------|---------|:-------:|---------:|
| 1 | fmt | cargo fmt --all --check | true | ✓ |
| 2 | lint-src | clippy -D warnings | true | ✓ |
| 3 | check | cargo check --workspace --all-targets --all-features | true | ✓ |
| 4 | sanitizer-address-check | RUSTFLAGS=-Zsanitizer=address cargo test | true | ✓ |
| 5 | verify-kani | cargo kani -p vb_core --harness ... (4 harnesses) | true | 🟡 |
| 6 | nightly-feature-gate | bash scripts/check-nightly-features.sh | true | ✓ |
| 7 | nightly-feature-cargo-probe | `true` (no-op) | true | ❌ **PHANTOM** |
| 8 | source-length | bash scripts/check-source-length.sh | true | ❌ RED |
| 9 | supply-chain | cargo audit/deny/vet/geiger/macheta | true | ✓ (advisory) |
| 10 | feature-powerset | cargo hack check --feature-powerset | true | ✓ |
| 11 | hardened-build | cargo build --profile hardened | true | ✓ |
| 12 | test | cargo nextest run --workspace | true | ✓ |
| 13 | doc-test | cargo test --doc --workspace | true | ✓ |
| 14 | doc | cargo doc --workspace | true | ✓ |
| 15 | mutants-smoke | cargo mutants (1 function) | true | ❌ **THEATER** |
| 16 | fuzz-smoke | cargo fuzz (5 targets × 1s) | true | ❌ **THEATER** |
| 17 | miri | cargo miri test (3 filter calls) | true | ❌ **THEATER** |
| 18 | verify-verus | bash scripts/verify-verus.sh | true | ✓ (registry) |
| 19 | verify-tlc | tlcc (2 root specs) | true | ✓ (fail-closed) |
| 20 | coverage | cargo llvm-cov (1 test) | true | ❌ **THEATER** |
| 21 | bench-build | cargo bench (1 benchmark) | true | 🟡 |

**5 of 21 pipeline tasks are smoke-only theater**: mutants, fuzz, miri, coverage, bench-build.

**2 of 21 are phantom tasks**: nightly-feature-cargo-probe (script body `true`), banned-token-gates (no `command:`).

### runInCI: false Tasks (15)

| Task | Reason |
|------|--------|
| test-determinism | "Current tree has pre-existing findings; run explicitly until clean" — **1,088 findings** |
| benchmark-regression-policy | "xtask excluded from workspace" |
| benchmark-proof | "180m criterion run" |
| pgo-instrument-build | "PGO profile generation, not a CI gate" |
| pgo-optimized-build | "PGO profile use, not a CI gate" |
| maxperf | "release build" |
| maxperf-native | "native CPU profile" |
| verify-fast | "Kani gauntlet — 4 harnesses" |
| verify-standard | "Kani gauntlet — 7 harnesses" |
| verify-deep | "Kani gauntlet + dedup" |
| verify-proof | "full gauntlet" |
| verify-all | "wraps verify-proof" |
| contracts | "xtask not in workspace" |
| quick | "Local fast dev loop" |
| verify-verus-all | "180m run" |

**test-determinism is the worst — it hides 1,088 findings (256 UncontrolledClock, 784 SharedTempState, 31 UncontrolledRandom, 15 GlobalMutableState, 2 SleepAsSync).**

## .cargo/

**Files:** `config.toml` (1.2 KB), `rust-toolchain.toml` (122 B), no `vendor/`

`config.toml`:
```toml
[target.x86_64-unknown-linux-gnu]
linker = "/usr/bin/clang"
runner = "true"
```

`rust-toolchain.toml`:
```toml
[toolchain]
channel = "nightly-2026-04-28"
components = ["rustc", "cargo", "rust-std", "rust-src", "clippy", "rustfmt", "miri", "rust-analyzer", "llvm-tools-preview"]
profile = "minimal"
```

Toolchain pinned ✓.

## supply-chain/

**Files:** `cargo-vet.toml` (3.4 KB) + `deny.toml` (1.8 KB) + `imports/` directory (4.2 KB, 1 .lock file + 3 .json audit files)

`cargo-vet.toml` exemptions: 12 entries (each is a `published-above = "<crate> <version>"` waiver for crates not yet vetted).

`deny.toml`:
- license: allow MIT/Apache-2.0/BSD-3-Clause/ISC/Zlib
- bans: 5 banned crates (json, http, reqwest, hyper, tokio-postgres)
- advisories: `db-path = "$WORKSPACE/.supply-chain/audit-db.json"` (1,234 entries)

## Root Configs

- `Cargo.toml` workspace: 19 members (17 production + xtask excluded + 1 fuzz)
- `.geigerignore`: 4 entries (for the inevitable serde/postcard false positives)
- `mutants.toml`: 4 entries (skip proptest files + skip test mocks + skip workspace_tests + skip fuzz)
- `config.yaml`: master §6 reviewer config (3 reviewer types, 5 reviewer weights)

## Forbidden Pattern Audit

| Pattern | .moon | scripts | .cargo |
|---------|------:|--------:|-------:|
| `unwrap()` | n/a | n/a | n/a |
| `unsafe` | 0 | 0 | 0 |
| `true` (no-op script) | 1 (nightly-feature-cargo-probe) | 0 | 0 |

## verdict

**72 / 100 — Broad coverage, smoke lanes are false confidence.**

Top concerns:
1. 5 of 21 pipeline tasks are smoke-only theater (mutants 0.026%, fuzz 5s, miri 0.14%, coverage 1 test, bench-build 1)
2. 2 phantom tasks (nightly-feature-cargo-probe script body is `true`; banned-token-gates no command)
3. test-determinism hides 1,088 findings with runInCI: false
4. 15 of 50 tasks are runInCI: false
5. 12 cargo-vet exemptions
6. Toolchain pinned ✓ (nightly-2026-04-28)
