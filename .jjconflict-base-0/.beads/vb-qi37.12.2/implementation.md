# Implementation — vb-qi37.12.2

STATUS: APPROVED

Holzman references read before implementation:
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`

## Changes
- `ResumeError::JournalAppendFailed` now preserves `RuntimeError` source.
- Added `ResumeError::ResumeDriveFailed { source }` for resume drive failures.
- `handle_resume` now propagates `drive_run` failure instead of silently discarding it.
- Failed `Resumed` append restores `RuntimeState::Resumable`.
- `From<ResumeError> for RuntimeError` preserves source errors for the new variants.

## Touched production files
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`
- `crates/vb_runtime/src/shard/types.rs`
- `crates/vb_runtime/src/error/conversions.rs`

## State 11 fmt/parser repair — 2026-05-14

STATUS: PASS — release fmt/parser blocker repaired.

References read before repair:
- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

Repair changes:
- `fuzz/src/bin/step_budget_new.rs`: repaired malformed doc-comment line `!` to `//!` so rustfmt/parser can parse the fuzz bin.
- `crates/vb_core/src/lib.rs`: corrected missing `#[cfg(kani)]` module declaration from nonexistent `kani` to existing `kani_idempotency_gates`, unblocking rustfmt module resolution.
- Ran rustfmt for workspace formatting drift required by the release fmt gate.

Command evidence:
- `cargo fmt --check` — FAIL before repair: malformed `fuzz/src/bin/step_budget_new.rs` (`expected item, found '!'`) plus missing `crates/vb_core/src/kani.rs` module resolution and formatting drift.
- `cargo fmt --check` — FAIL after fuzz repair: missing `crates/vb_core/src/kani.rs` module resolution and formatting drift remained.
- `cargo fmt` — PASS, formatted workspace drift after parser/module blockers were repaired.
- `cargo fmt --check` — PASS.

Rules affected:
- Power-of-Ten Rule 10 / zero warnings: satisfied for the requested fmt gate.
- Zero-panic/unsafe rule: no production `unsafe`, `unwrap`, `expect`, `panic`, `todo`, or `unimplemented` introduced by this repair.

Performance-layer decision: no performance claim made; no benchmark/profiler evidence required.

Remaining blockers:
- Full clippy/test cleanup intentionally not run for this scoped fmt/parser repair; vb_runtime test lint cleanup remains owned by another agent per task instruction.

## State 11 API compatibility repair — 2026-05-14

STATUS: REPAIRED — `cargo semver-checks -p vb_runtime --baseline-rev HEAD` passes.

References read before repair:
- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

Repair changes:
- Restored public `ResumeError` variant set and shapes to the HEAD baseline: `JournalAppendFailed` is again a unit variant, `StructuredOutputFailed` returns to discriminant position 4, and `ResumeDriveFailed` was removed.
- Added `ResumeError::source_runtime_error()` as a non-breaking accessor for the most recent same-thread `RuntimeError` source associated with `JournalAppendFailed`.
- Kept failed `Resumed` append state restoration and drive-error propagation; drive errors now return the semver-compatible `JournalAppendFailed` variant with recorded source detail.
- Updated `From<ResumeError> for RuntimeError` to preserve the recorded runtime source when present and fall back to the historical `WriteLockPoisoned` journal error otherwise.
- Updated scoped resume-propagation tests to assert both semver-compatible variant shape and source observability through the accessor.

Command evidence:
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime --test vb_qi37_12_2_resume_error_propagation --all-features` — PASS, 7 passed.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime --lib is_resumable` — PASS, 2 passed.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo semver-checks -p vb_runtime --baseline-rev HEAD` — PASS, 196 checks passed, 56 skipped, no semver update required.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo clippy -p vb_runtime --lib --bins --examples --all-features -- -D warnings` — PASS.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= rustfmt --edition 2024 --check crates/vb_runtime/src/shard/types.rs crates/vb_runtime/src/shard/lifecycle/chunk_001.rs crates/vb_runtime/src/error/conversions.rs crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs` — PASS after formatting touched files.

Rules affected:
- Power-of-Ten Rule 5 / typed invariants: source detail is exposed by a typed accessor without widening the public enum.
- Power-of-Ten Rule 7 / checked returns: drive failures are no longer silently dropped.
- Power-of-Ten Rule 10 / API/static gate: semver-checks passes against HEAD.
- Zero-panic/unsafe rule: no production `unsafe`, `unwrap`, `expect`, `panic`, `todo`, or `unimplemented` introduced.

Performance-layer decision: no performance claim made; no benchmark/profiler evidence required.

Residual risk:
- Source detail is semver-compatible but stored as same-thread last resume source, so concurrent consumers must inspect/convert the returned error promptly. A per-instance source field would be cleaner but is the exact public API break repaired here.

## State 12.2 black-hat source-binding repair — 2026-05-14

STATUS: REPAIRED — State 8 black-hat source-staleness tests pass while preserving the public `ResumeError::JournalAppendFailed` unit variant and semver compatibility.

References read before repair:
- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

Repair changes:
- Replaced the ambient single `LAST_RESUME_SOURCE` slot with a bounded same-thread `ResumeSourceRegistry` containing pending sourced failures and bound accessor keys.
- `ResumeError::journal_append_failed_with_source` records a bounded pending source without changing the public enum shape.
- `ResumeError::source_runtime_error()` now binds a pending source to the concrete returned error reference on first observation and returns the same source for later observations of that value.
- `Drop for ResumeError` clears bound source entries for dropped journal-append errors to prevent stack-address reuse from leaking stale sources into later fresh values.
- Fresh manually constructed `ResumeError::JournalAppendFailed` values no longer inherit stale sources when no pending source belongs to them.
- Formatted the scoped resume propagation test file lines reported by rustfmt.

Command evidence:
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime --test vb_qi37_12_2_resume_error_propagation --all-features` — PASS, 10 passed.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime --lib is_resumable` — PASS, 2 passed.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo semver-checks -p vb_runtime --baseline-rev HEAD` — PASS, 196 checks passed, 56 skipped, no semver update required.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo clippy -p vb_runtime --lib --bins --examples --all-features -- -D warnings` — PASS.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= rustfmt --edition 2024 --check crates/vb_runtime/src/shard/types.rs crates/vb_runtime/src/shard/lifecycle/chunk_001.rs crates/vb_runtime/src/error/conversions.rs crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs` — FAIL before test-file formatting repair; PASS after repair.

Rules affected:
- Power-of-Ten Rule 2 / bounded resources: registry keeps at most 64 pending sources and 64 bound source entries per thread.
- Power-of-Ten Rule 5 / invariant density: stale-source prevention is explicit in `source_for` instead of hidden ambient latest-state lookup.
- Power-of-Ten Rule 7 / checked results: source-producing drive/append errors remain propagated through typed `ResumeError` values.
- Power-of-Ten Rule 10 / zero warnings: required tests, clippy, semver, and rustfmt gates pass.
- Zero-panic/unsafe rule: no production `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable`, production assert macros, or unchecked indexing introduced.

Performance-layer decision: no performance claim made; this is cold error-path bookkeeping only, bounded to 64 entries per thread.

Second-ring evidence:
- Public API compatibility evidence attached by `cargo semver-checks -p vb_runtime --baseline-rev HEAD` PASS.
- No assembly/IR/provenance claim made; no assembly/IR/SBOM evidence required.

Residual risk:
- The semver-compatible unit variant cannot store a private source payload directly; this repair uses bounded same-thread accessor binding to avoid stale-source laundering without changing public enum shape.

## State 10 stop — unobserved-source binding is impossible without contract change — 2026-05-14

STATUS: BLOCKED — requires State 3 contract narrowing/owner decision.

References read before decision:
- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

Decision:
- True error-bound source preservation cannot be implemented while keeping `ResumeError::JournalAppendFailed` as a public unit variant and without adding a non-semver-compatible payload/variant.
- The current `ResumeSourceRegistry` is a same-thread side channel. It can bind a source only when `source_runtime_error()` is first called. If the real returned `ResumeError` remains unobserved, a fresh unrelated `ResumeError::JournalAppendFailed` is indistinguishable from the real unobserved error and can consume the pending source.
- Binding at `journal_append_failed_with_source` construction time is not sound because Rust moves the enum value after return; the address of a local temporary is not the stable identity of the returned value.
- A unit enum variant carries no per-instance token, allocation handle, or hidden private field. There is no move hook to update a registry key. Any attempted fix that preserves the unit variant must remain an ambient side channel and therefore fails the black-hat unobserved-source tests.

Commands run before stop:
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime --test vb_qi37_12_2_resume_error_propagation --all-features` — FAIL, 10 passed / 2 failed: `fresh_journal_append_failed_cannot_steal_unobserved_pending_source`, `runtime_conversion_of_fresh_error_cannot_steal_unobserved_pending_source`.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime --test vb_qi37_12_2_resume_error_propagation unobserved_pending_source -- --nocapture` — FAIL, 0 passed / 2 failed with the same two unobserved-source theft regressions.

Required commands intentionally not run after stop:
- `cargo test -p vb_runtime --lib is_resumable`
- `cargo semver-checks -p vb_runtime --baseline-rev HEAD`
- `cargo clippy -p vb_runtime --lib --bins --examples --all-features -- -D warnings`
- `cargo clippy -p vb_runtime --lib --tests --all-features -- -D warnings`
- rustfmt check touched files

Reason skipped:
- The user instruction required stopping instead of faking with another side channel when true source binding is impossible under the semver-compatible unit-variant constraint.

Owner decision needed:
- Narrow the contract so `JournalAppendFailed` has no source-preservation requirement; or
- Approve a semver break/new public shape that carries the source (for example a source-bearing variant or opaque/private payload design) and rerun State 10/11/12.

Power-of-Ten and zero-panic impact:
- No production Rust changed in this stop.
- Avoided adding another side channel that would violate typed-error/invariant-density requirements.

Performance-layer decision: no performance claim made; no benchmark/profiler evidence required.

Second-ring evidence:
- Public API compatibility remains an unresolved owner tradeoff; `cargo semver-checks` was not rerun because implementation stopped before code changes.
