bead_id: vb-qi37.23
bead_title: quality: Full gate evidence refresh
phase: 2
updated_at: 2026-05-18T20:32:55Z
attempt: 1-of-7
# Codebase Map

STATUS: COMPLETE
Scope type: release quality/evidence refresh; no production code ownership expected.
## Primary source files

- `.moon/tasks/all.yml`: canonical Moon tasks for fmt, lint-src, check, test, doc-test, doc, workspace-assertions, source-length, supply-chain, feature-powerset, miri, coverage, mutants-smoke, fuzz-smoke, sanitizer-address-check, bench-build, verify-standard/deep/proof/all, contracts.
- `scripts/rust-verification-gauntlet.sh`: canonical verification lane wrapper for verify-standard/proof/deep/all.
- `velvet-ballistics-MASTER.md`: DoD clauses require full current-scope gates and traceable bead closure evidence; lines found include required fuzz/Miri/coverage/mutants/supply-chain/benchmark/API/bloat concerns.
- `Cargo.toml`, `Cargo.lock`, `deny.toml`, `cargo-vet.toml`, `supply-chain/**`: dependency/supply-chain inputs.
- `fuzz/**`: fuzz build surface.
- `crates/**`, `contracts/**`, `xtask/**`, `benches/**`: build/test/proof/benchmark surface.
## Public APIs / dependencies

- Workspace crates: vb_core, vb_expr, vb_compile, vb_codegen, vb_ipc, vb_runtime, vb_storage, vb_validate, vb_yaml, vb_cli/workspace tests/fuzz package.
- Dependency changes: none intended by this bead.
- Public API changes: none intended; evidence probes only.
## Risk tags

release-blocker, ci, evidence, testing, performance, supply-chain, fuzz, miri, coverage, mutants, sanitizer, public-api, bloat, benchmark
