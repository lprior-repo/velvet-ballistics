# Formal Verification Report

STATUS: APPROVED

## Startup Doctrine

- Read `/home/lewis/.claude/skills/formal-verifier/SKILL.md`: version `1.5.0`; requires exact approved commands, accounting for every obligation, fail-closed missing required tools, and `formal-verification-report.md` plus `verification-ledger.jsonl` outputs.
- Read `/home/lewis/.agents/skills/formal-verifier/SKILL.md`: same version/content observed; this agents copy wins on conflict.

## Inputs

- Workdir: `/home/lewis/src/go-skill-vb-m5gp`.
- Bead: `vb-m5gp`; State 11 rerun after dependency-edge repair.
- `proof-obligations.jsonl`: `.beads/vb-m5gp/proof-obligations.jsonl`, parsed with `jq -c .`.
- `traceability-matrix.jsonl`: `.beads/vb-m5gp/traceability-matrix.jsonl`, parsed with `jq -c .`.
- `delivery-scope.jsonl`: `.beads/vb-m5gp/delivery-scope.jsonl`, parsed with `jq -c .`.
- `proof-obligations.planned.jsonl`: parsed with `jq -c .`.
- `baseline-report.md`: baseline `moon ci` clean at shared parent.
- `tla-spec.md` / `lean-contract.md`: waiver inputs present.
- `contract-verification-review.md`: contains `STATUS: APPROVED`.

## Tool Availability

- cargo: `/home/lewis/.cargo/bin/cargo`.
- cargo kani: `/home/lewis/.cargo/bin/cargo-kani`.
- moon: `/home/lewis/.local/share/mise/installs/npm-moonrepo-cli/2.2.4/bin/moon`.
- jq: `/usr/bin/jq`.
- tlc: `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`.
- apalache-mc: `/home/lewis/.local/share/mise/installs/http-apalache/0.57.0/bin/apalache-mc`.
- verus: `/home/lewis/.local/bin/verus`.
- lake: `/home/lewis/.elan/bin/lake`.
- cargo-mutants: `/home/lewis/.cargo/bin/cargo-mutants`.
- cargo-llvm-cov: `/home/lewis/.cargo/bin/cargo-llvm-cov`.
- Missing but not required for this approved bead scope: kani binary alias, aeneas, charon, hax, cargo-careful, cargo-fuzz, cargo-semver-checks, cargo-auditable, cargo-cyclonedx, crux, saw, stateright.

## Obligation Results

- STRUCT-001: PASS — `cargo +nightly check -p vb_compile --all-targets --all-features` exit 0; `lib.rs` declares private `mod_compile_core`, `mod_compile_errors`, `mod_compile_lowering`, and `mod_compile_validation` modules and preserves crate-root re-exports.
- API-001: PASS — `cargo +nightly test -p vb_compile --all-targets --all-features` exit 0; 245 lib tests, 9 idempotency tests, 15 primitive-lowering tests, and 10 strict-yaml tests passed.
- API-002: PASS — `cargo +nightly test -p velvet-ballastics-workspace-tests --test integration_compile_codegen_pipeline --test integration_compile_codegen_runtime_e2e --test integration_compile_error_message_quality --test integration_validate_yaml_parsing` exit 0; selected integration suites passed: 15, 23, 21 passed/4 ignored, and 29 tests.
- BEHAV-001: PASS — `moon ci` exit 0; 23 tasks completed; nextest summary 10771 passed, 44 skipped; source-length lane emitted only DEFERRED_GLOBAL pre-existing unrelated files.
- ERR-001: PASS — exact diagnostics command exit 0; diagnostic integration suite passed 21 tests with 4 ignored pre-existing tests.
- STRUCT-002: PASS — manual source review after dependency-edge repair found `errors_to_validation=0 matches`, `validation_to_lowering_or_core=0 matches`, no `include!` bodies, and executable dependency-edge test passed.
- STRUCT-003: PASS — manual source review found real owned split modules, no `compile_core_impl.rs` include body, and no blind wiring of stale `compile/`, `lower/`, or `validation/` scaffolding.
- STATIC-001: PASS — `cargo +nightly clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings` exit 0; stricter source scan with no unsafe/unwrap/expect/panic/todo/dbg/indexing/arithmetic/as-conversions also passed.
- FORMAT-001: PASS — `cargo +nightly fmt --all --check` exit 0.
- LEN-001: PASS — exact top-level source-count command exit 0: `lib.rs=61`, `mod_compile_core.rs=230`, `mod_compile_errors.rs=12`, `mod_compile_validation.rs=17`, `mod_compile_lowering.rs=42`; recursive `bash scripts/check-source-length.sh` exit 0 with only DEFERRED_GLOBAL pre-existing unrelated files.
- MIRI-001: DEFERRED_GLOBAL — optional direct `cargo +nightly miri test -p vb_compile` failed before tests because local nightly rust-src path `/home/lewis/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library` does not exist; canonical `moon ci` Miri lane passed selected checks.
- KANI-001: PASS — `cargo kani --package vb_compile --harness idempotency_gate_parity --quiet` exit 0.
- TLA-WAIVER-001: WAIVED — approved non-applicability in `contract-verification-review.md`.
- THM-WAIVER-001: WAIVED — approved theorem-kernel waiver in `contract-verification-review.md`.
- VERUS-WAIVER-001: WAIVED — approved conditional pure-refactor waiver; rerun source/API/behavior/Kani evidence supports no semantic change.

## Waivers

- TLA-WAIVER-001, THM-WAIVER-001, and VERUS-WAIVER-001 are accepted by `contract-verification-review.md` (`STATUS: APPROVED`).

## Residual Risk

- Direct optional `cargo +nightly miri test -p vb_compile` remains environment-blocked by a missing nightly rust-src path. This is non-blocking because the obligation is `required:false` and `moon ci` Miri lane passed selected checks.
- Pre-existing oversized unrelated `vb_compile` sources remain DEFERRED_GLOBAL: `expression_bytecode.rs`, `expression.rs`, `references.rs`, `schema.rs`, and `type_taint.rs`. Bead-local `lib.rs` and `mod_compile_*` sources are below 300 lines recursively.
