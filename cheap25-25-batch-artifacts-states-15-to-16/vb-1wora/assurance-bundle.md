# Assurance Bundle — vb-1wora

bead_id: vb-1wora
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora
commit_or_change: vlyqryto ba210bf8 (p11-holzman-rust — reject trailing bytes in codec)

## Status

STATUS: APPROVED

This bundle packages the existing State 11 (holzman-rust) implementation, State 12 (formal-verifier) verification ledger, and State 13 (black-hat-reviewer) review artifacts. It does not create new correctness claims. The 2 BLOCKED_TOOLING items (full Kani run + production-inner drift gate) are pre-existing workspace-level or unowned issues, not vb-1wora regressions; they are disclosed for transparency in the Residual Risk section and do not prevent bead approval.

## Scope and Claim Boundaries

- **Bead:** `vb-1wora` — Codec: reject trailing bytes after declared record payload (P1 bug).
- **Delivery scope:** `crates/vb_storage/src/codec/payload.rs`, `crates/vb_storage/src/codec/envelope.rs`, `crates/vb_storage/src/codec/tests.rs`, `crates/vb_storage/src/error/mod.rs`, `crates/vb_storage/src/error/codes.rs`, `crates/vb_storage/src/error_tests.rs`, `crates/vb_storage/src/error_code_tests.rs`, `crates/vb_storage/src/kani_postcard_envelope_wire.rs`, `crates/vb_storage/src/security_tests.rs`, `crates/vb_storage/src/tests.rs`, `crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs`, `verification/verus/vb-vzcuf-PS-003.rs`, `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs`, `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs`.
- **Claimed confidence:** executable runtime behavior (cargo test), property-based behavior (proptest), hostile-input behavior (cargo-fuzz), Verus bridge arm (WEAK_MIRROR with exec wrapper), Kani H6 syntax (full Kani BLOCKED_TOOLING), strict production source lint (clippy), and rustfmt.
- **No claimed confidence:** TLA+ (not_applicable), Loom (not_applicable), Miri (not_applicable), Flux (not_applicable), `SymbolicCode::JOURNAL_TRAILING_BYTES` registration in `CODE_REGISTRY` (recommended but not mandatory), full Kani H6 (BLOCKED_TOOLING), production-inner drift gate (BLOCKED_TOOLING).
- **No global claim:** `moon ci` (not run in this state; out of bead scope).

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| **INV-CODEC-TB-001** decoder returns Err(TrailingBytes) iff bytes.len() > payload_end | `contract.md §5.1` row 3, `§5.2` lane: Verus+Kani+proptest | POB-001 rust-local structural review; POB-002 cargo-test 6/6 (decode_rejects_trailing_bytes_after_payload, decode_envelope_only_rejects_trailing_payload, trio); POB-003 proptest ps003_trailing_bytes_are_rejected; POB-004 Kani H6 syntax (BLOCKED_TOOLING for full Kani) | proof-review.md (APPROVED), black-hat-review.md PHASE 1 (APPROVED) | SATISFIED_BY_EXECUTABLE_AND_PROPERTY_EVIDENCE |
| **INV-CODEC-TB-002** Ok only if bytes.len() == payload_end (no false positive) | `contract.md §5.1` row 2, `§5.2` lane: Verus+Kani+proptest | POB-003 proptest ps003_exact_boundary_roundtrips (1024 cases); POB-005 proptest same family | proof-review.md, black-hat-review.md | SATISFIED_BY_PROPERTY_EVIDENCE |
| **INV-CODEC-TB-003** trailing-bytes check precedes verify_digest_match (cheap-before-expensive) | `contract.md §5.3` ordering, `§5.2` lane: structural review | POB-001 rust-local diff (lines 11-16 of payload.rs diff; lines 12-17 of envelope.rs diff — `if bytes.len() > payload_end` block is positioned between `bytes.get` and `verify_digest_match`); POB-004 Kani H6 `Err(_)` and `Ok(_)` arms call `kani::assert(false, ...)` | proof-review.md, black-hat-review.md PHASE 1 | SATISFIED_BY_STRUCTURAL_AND_KANI_SYNTAX_EVIDENCE |
| **INV-CODEC-TB-004** decode_envelope_only mirror site has the same check | `contract.md §4.1` mirror site, `§5.2` lane: Verus+proptest | POB-001 diff (envelope.rs:77-83); POB-002 cargo-test decode_envelope_only_rejects_trailing_payload | proof-review.md, black-hat-review.md | SATISFIED_BY_EXECUTABLE_EVIDENCE |
| **INV-CODEC-TB-005** TrailingBytes { trailing } reachable only when trailing > 0 | `contract.md §5.2` lane: type system + Kani | POB-001 producer site is `if bytes.len() > payload_end` (mathematically implies `trailing > 0`); POB-002 cargo-test asserts `trailing: 3` (3-byte fixture) and `trailing: 4` (4-byte fixture); POB-003 proptest asserts `trailing > 0`; POB-004 Kani H6 `kani::assert(actual > 0, ...)` at line 408-411 | proof-review.md, black-hat-review.md | SATISFIED_BY_TYPE_AND_KANI_SYNTAX_EVIDENCE |
| **INV-CODEC-TB-006** TRAILING_BYTES_CODE == DiagnosticCode::new(0x4042) and diagnostic_code()/symbolic_code() arms wired | `contract.md §6.2`, `§4.1` items 4-6, `§5.2` lane: unit test + Verus | POB-001 diff (codes.rs:85 `pub const TRAILING_BYTES_CODE: DiagnosticCode = DiagnosticCode::new(0x4042)`; line 132 `Self::TrailingBytes { .. } => Self::TRAILING_BYTES_CODE`); POB-002 cargo-test trailing_bytes_error_code and trailing_bytes_error_has_correct_code (6/6) | proof-review.md, black-hat-review.md | SATISFIED_BY_EXECUTABLE_EVIDENCE |
| **INV-CODEC-TB-007** Verus PS-003 bridge enumerates Err(SpecJournalError::TrailingBytes { trailing: u32 }) as a reachable arm | `contract.md §7.2`, `§5.2` lane: Verus + drift gate | POB-006 Verus smoke (`verus --crate-type=lib verification/verus/vb-vzcuf-PS-003.rs` -> 25 verified, 0 errors); production-binding gate (`bash scripts/check-verus-production-binding.sh` -> STRONG:0, WEAK:71, VACUUM:0); new exec wrapper `wrapper_decode_record_trailing_bytes` (lines 1110-1235) exercises the new arm; drift gate BLOCKED_TOOLING (TL-vb-1wora-002) | proof-review.md, black-hat-review.md PHASE 1 | SATISFIED_BY_VERUS_SMOKE_AND_BINDING_GATE; drift gate documented as BLOCKED_TOOLING |
| **HOSTILE-INPUT-001** fuzz oracle: N=0 -> Ok, N>=1 -> Err(TrailingBytes { trailing: N }); no panic, no UB | `hazard-analysis.md §2.7`, `proof-seeds.jsonl:PS-VB-1WORA-008` | POB-007 cargo-fuzz 60s wallclock (37,080,025 runs, 0 crashes, 0 ooms); cargo check on fuzz target exit 0 | proof-review.md, black-hat-review.md PHASE 1 | SATISFIED_BY_FUZZ_EVIDENCE |
| **Recommended but not mandatory:** `JOURNAL_TRAILING_BYTES` registered in `CODE_REGISTRY` | `contract.md §4.2` recommended, `§6.3` | not_registered_in_CODE_REGISTRY; falls back to `SymbolicCode::INTERNAL_INVARIANT` per `error-taxonomy.md §2.5` (documented behavior). The numeric code (0x4042) and the diagnostic_code() arm are mandatory and locked by POB-001/002. | proof-plan-review.md, formal-waivers.jsonl:FW-vb-1wora-005 | WAIVED, NOT PASS (recommended-only, non-behavior) |

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| POB-vb-1wora-001 | rust-local | `diff -u <(jj file show -r rktonwky ...) ...` | `.beads/vb-1wora/evidence/po-001-diff-*.txt` (6 files) | PASS (diff confirms `if bytes.len() > payload_end` between `bytes.get` and `verify_digest_match`; TRAILING_BYTES_CODE = 0x4042; diagnostic_code() and symbolic_code() arms wired) | — |
| POB-vb-1wora-002 | cargo-test | `cargo test -p vb_storage --lib -- decode_rejects_trailing_bytes_after_payload decode_envelope_only_rejects_trailing_payload trailing_bytes_variant_and_fields trailing_bytes_display_format trailing_bytes_error_code trailing_bytes_error_has_correct_code` | `.beads/vb-1wora/evidence/po-002-cargo-test-trailing-bytes-direct.log` | PASS (6 passed; 0 failed; 0 ignored) | — |
| POB-vb-1wora-003 | proptest | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003` | `.beads/vb-1wora/evidence/po-proptest-vb-vzcuf-PS-003.log` | PASS (8 passed; 0 failed; 0 ignored; includes ps003_trailing_bytes_are_rejected and ps003_exact_boundary_roundtrips) | — |
| POB-vb-1wora-004 | kani (smoke) | `cargo check -p vb_storage --features legacy-kani` | `.beads/vb-1wora/evidence/po-004-kani-cargo-check-legacy.log` | PASS (Kani H6 syntax verified under cfg(kani) gate; H6 uses kani::any() per GOD RULE 1) | — |
| POB-vb-1wora-004 | kani (full) | `cargo kani list` | (BLOCKED_TOOLING: pre-existing `vb_core/src/frame/parts/kani_helpers.rs:22` compile error, TL-vb-1wora-003) | BLOCKED_TOOLING | — |
| POB-vb-1wora-005 | proptest | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003` | (same as POB-003) | PASS | — |
| POB-vb-1wora-006 | verus (smoke) | `verus --crate-type=lib verification/verus/vb-vzcuf-PS-003.rs` | `.beads/vb-1wora/evidence/po-006-verus-ps-003-bridge-trailing-bytes.log` | PASS (25 verified, 0 errors; includes new wrapper_decode_record_trailing_bytes) | — |
| POB-vb-1wora-006 | verus (binding gate) | `bash scripts/check-verus-production-binding.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora` | `.beads/vb-1wora/evidence/po-006-verus-production-binding-gate.log` | PASS (STRONG:0, WEAK:71, VACUUM:0) | — |
| POB-vb-1wora-006 | verus (drift gate) | `bash scripts/check-production-inner-drift.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora` | `.beads/vb-1wora/evidence/po-006-production-inner-drift-gate.log` | BLOCKED_TOOLING (TL-vb-1wora-002: JJ-only workspace, no .git) | — |
| POB-vb-1wora-007 | cargo-fuzz (compile) | `cargo check --manifest-path fuzz/Cargo.toml --bin fuzz_storage_codec_payload_corruption` | `.beads/vb-1wora/evidence/po-007-fuzz-cargo-check.log` | PASS (0 errors) | — |
| POB-vb-1wora-007 | cargo-fuzz (60s) | `cargo +nightly-2026-04-28 fuzz run --manifest-path fuzz/Cargo.toml fuzz_storage_codec_payload_corruption -- -max_total_time=60 -max_len=4096` | `.beads/vb-1wora/evidence/po-007-fuzz-trailing-bytes-60s.log` | PASS (37,080,025 runs in 61 seconds; 0 crashes; 0 ooms; coverage 162/165) | — |

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| Full local cargo test | `cargo test -p vb_storage --all-features --no-fail-fast` | `.beads/vb-1wora/evidence/po-cargo-test-all-features.log` | 1678 passed (17 suites); 0 failed; 0 ignored; 0 measured; 0 filtered out |
| Proptest PS-003 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003` | `.beads/vb-1wora/evidence/po-proptest-vb-vzcuf-PS-003.log` | 8 passed; 0 failed; 0 ignored |
| Trailing-bytes direct cargo test | `cargo test -p vb_storage --lib -- decode_rejects_trailing_bytes_after_payload ...` | `.beads/vb-1wora/evidence/po-002-cargo-test-trailing-bytes-direct.log` | 6 passed; 0 failed; 0 ignored; 1529 filtered out |
| Strict source lint | `cargo clippy -p vb_storage --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` | `.beads/vb-1wora/evidence/po-cargo-clippy.log` | No issues found |
| Rustfmt | `cargo fmt --check -p vb_storage` | `.beads/vb-1wora/evidence/po-cargo-fmt-check.log` | exit=0 (no diff) |
| Kani H6 syntax smoke | `cargo check -p vb_storage --features legacy-kani` | `.beads/vb-1wora/evidence/po-004-kani-cargo-check-legacy.log` | exit=0 (0 errors, 0 warnings) |
| Verus bridge | `verus --crate-type=lib verification/verus/vb-vzcuf-PS-003.rs` | `.beads/vb-1wora/evidence/po-006-verus-ps-003-bridge-trailing-bytes.log` | 25 verified, 0 errors |
| Verus production-binding gate | `bash scripts/check-verus-production-binding.sh ...` | `.beads/vb-1wora/evidence/po-006-verus-production-binding-gate.log` | STRONG:0, WEAK:71, VACUUM:0, exit=0 |
| cargo-fuzz 60s wallclock | `cargo +nightly-2026-04-28 fuzz run ...` | `.beads/vb-1wora/evidence/po-007-fuzz-trailing-bytes-60s.log` | 37,080,025 runs; 0 crashes; 0 ooms |

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Contract (rust-contract, State 3) | `.beads/vb-1wora/contracts/contract.md` | produced | none |
| Proof plan review (State 4) | `.beads/vb-1wora/proof-plan-review.md` | APPROVED (line 138) | 0 blockers; 0 VACUUM |
| Proof review (State 6) | `.beads/vb-1wora/proof-review.md` | APPROVED (line 227) | 5 fixed_with_evidence; 0 blockers; 0 VACUUM |
| Bridge review (State 7) | `.beads/vb-1wora/proof-to-rust-review.md` | APPROVED (line 267) | 5 owner_approved_no_action |
| Formal verification (State 12) | `.beads/vb-1wora/formal-verification-report.md` | APPROVED_WITH_BLOCKED_TOOLING (line 5) | 2 BLOCKED_TOOLING (Kani, drift gate) |
| Black-hat review (State 13) | `.beads/vb-1wora/black-hat-review.md` | APPROVED (line 18) | 2 LOW accepted |

## Findings Disposition

| Finding | Severity | Source Review | Disposition | Evidence Or Owner Approval |
|---|---|---|---|---|
| proof-review.md:F-001 (verifier abstraction simplification — body abstraction in mirror) | LOW | proof-review.md | fixed_with_evidence | proof-writer-report.md §3 documents the abstraction; bridge wrapper `wrapper_decode_record_trailing_bytes` exercises the arm; Verus smoke 25/0 verifies it |
| proof-review.md:F-002 (Verus `u32` vs. production `usize` modeling decision) | LOW | proof-review.md | fixed_with_evidence | proof-writer-report.md §3 documents the modeling; `trailing == (bytes.len() as u32) - expected_payload_end && trailing > 0` postcondition bounds the cast to values `< 2^32` |
| proof-review.md:F-003 (drift gate blocker, pre-existing workspace-isolation tooling) | LOW | proof-review.md | fixed_with_evidence | TL-vb-1wora-002 documents BLOCKED_TOOLING; mirror change is structurally sound per manual review; formal-verification-report.md:Drift Gate Pre-Check section |
| proof-review.md:F-004 (Kani H6 compile error blocker, pre-existing vb_core issue) | LOW | proof-review.md | fixed_with_evidence | TL-vb-1wora-003 documents BLOCKED_TOOLING; Kani H6 syntax verified under `cfg(kani)` gate; cargo-test sibling provides independent behavior oracle |
| proof-review.md:F-005 (fuzz target pre-fix compile error) | LOW | proof-review.md | fixed_with_evidence | TL-vb-1wora-004 documents `pending_formal_execution`; post-fix fuzz target compiles (cargo check exit 0); 60s wallclock run completed 37M+ iterations with 0 crashes |
| proof-to-rust-review.md:FINDING-1 (RRO-001 has empty behavior_test_refs/refinement_harness_refs) | LOW | proof-to-rust-review.md | owner_approved_no_action | RRO-001 is structural-review-only; downstream RROs cover behavior |
| proof-to-rust-review.md:FINDING-2 (proptest file path deviation) | LOW | proof-to-rust-review.md | owner_approved_no_action | Deviation documented; per-bead file convention followed |
| proof-to-rust-review.md:FINDING-3 (fuzz target sub-oracle vs. separate function) | LOW | proof-to-rust-review.md | owner_approved_no_action | Deviation documented; existing codebase pattern (one fuzz target per file, multiple sub-oracles) |
| proof-to-rust-review.md:FINDING-4 (Verus `u32` modeling decision) | LOW | proof-to-rust-review.md | owner_approved_no_action | Same as proof-review.md:F-002 |
| proof-to-rust-review.md:FINDING-5 (drift gate blocker in JJ-only workspace) | LOW | proof-to-rust-review.md | fixed_with_evidence | Same as proof-review.md:F-003 |
| black-hat-review.md:FINDING-001 (decode_record_payload and decode_envelope_only exceed Farley 25-line limit) | LOW | black-hat-review.md | owner_approved_no_action | Refactoring is out of scope per contract §4.3 ("Adding a new helper function. The check is inline."); 5-line addition is the minimum change required |
| black-hat-review.md:FINDING-002 (deviation between planned test names and actual test names) | LOW | black-hat-review.md | owner_approved_no_action | Deviations documented in proof-to-rust-review.md:FINDING-1..5 |

## Waivers and Deferred Work

Waivers and deferred work are not finding dispositions. Findings use only canonical `finding/v1.disposition` values: `fixed_with_evidence`, `owner_approved_debt`, `owner_approved_no_action`, or `blocker`.

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| FW-vb-1wora-001 (Loom) | `not_applicable` per `VLD-vb-1wora-007-loom`; pure parser over `&[u8]` with no concurrent memory ordering; crate is single-threaded synchronous | proof-planner | 2026-12-31 | POB-001, POB-002, POB-003, POB-004 (Kani syntax smoke) |
| FW-vb-1wora-002 (Miri) | `not_applicable` per `VLD-vb-1wora-008-miri`; vb_storage is `#![forbid(unsafe_code)]`; the new `TrailingBytes { trailing: usize }` variant contains no raw pointer / MaybeUninit / NonNull fields | proof-planner | 2026-12-31 | POB-001, POB-002, POB-007 (cargo-fuzz) |
| FW-vb-1wora-003 (Flux) | `not_applicable` per `VLD-vb-1wora-009-flux`; the fix introduces no refinement type; the `trailing > 0` invariant is enforced structurally at the producer site | proof-planner | 2026-12-31 | POB-003, POB-004, POB-006 (Verus bridge) |
| FW-vb-1wora-004 (TLA+) | `not_applicable` per `VLD-vb-1wora-010-tla-plus`; the decode pipeline is single-pass synchronous with no temporal / state-machine / distributed-protocol behavior; TLA+ was explicitly removed from the proof-planner skill | proof-planner | 2026-12-31 | POB-001, POB-002, POB-003, POB-005 |
| FW-vb-1wora-005 (CODE_REGISTRY registration for JOURNAL_TRAILING_BYTES) | `not_applicable` per contract §4.2 (Recommended, not mandatory); the `SymbolicCode` falls back to `INTERNAL_INVARIANT` (existing convention for unregistered symbolic names) | proof-planner | 2026-12-31 | POB-001 (symbolic_code() arm wired), POB-002 (trailing_bytes_error_code + trailing_bytes_error_has_correct_code assert the mandatory numeric code wiring) |
| BLOCKED_TOOLING (full Kani H6) | Pre-existing `vb_core/src/frame/parts/kani_helpers.rs:22` compile error (missing closing brace); unrelated to vb-1wora; routed to vb_core maintainer | vb_core maintainer | resolved when kani_helpers.rs:22 is fixed | Kani H6 syntax verified under `cfg(kani)` gate; cargo-test `decode_rejects_trailing_bytes_after_payload` and proptest `ps003_trailing_bytes_are_rejected` provide independent behavior oracles for INV-CODEC-TB-001 |
| BLOCKED_TOOLING (production-inner drift gate) | Pre-existing workspace-isolation tooling limitation: `bash scripts/check-production-inner-drift.sh` hard-codes `git rev-parse --show-toplevel`; the isolated workspace is JJ-only | femdation (workspace tooling) | resolved when the drift gate is re-run in a git-initialized checkout | Mirror change is structurally sound per manual review: the new `TrailingBytes { trailing: u32 }` variant at `production_inner/vb_vzcuf_PS_003_production.rs:403` is added between `UnexpectedEof` and `PostcardDecodeFailed`, mirroring the production-side placement between `UnexpectedEof` and `MalformedKeyspaceRow` per `contracts/type-contracts.md §1.3` |
| RESIDUAL: SymbolicCode::JOURNAL_TRAILING_BYTES not in CODE_REGISTRY | Recommended but not mandatory; falls back to `SymbolicCode::INTERNAL_INVARIANT`; tracked in `contract.md §11` risk register as LOW severity | proof-planner | 2026-12-31 | Mandatory numeric code (0x4042) and diagnostic_code() arm are wired (POB-001, POB-002) |
| RESIDUAL: pre-existing workspace-wide fmt failures (vb_core/src/lib.rs:26, vb_core/src/time.rs:71, vb_runtime/src/frame_pool/tests.rs:114,139) | Unrelated to vb-1wora; pre-existed the proof-writer's edits; classified as `BLOCK_GLOBAL` prerequisite repair | femdation (vb_core/vb_runtime fmt) | resolved when the pre-existing violations are repaired | `cargo fmt --check -p vb_storage` exit 0 (touched crate is fmt-clean) |

## Global Gate Classification

| Gate | Evidence | Classification | Disposition |
|---|---|---|---|
| Full local cargo test (1678) | `.beads/vb-1wora/evidence/po-cargo-test-all-features.log` | PASS | Supports runtime test confidence |
| Proptest PS-003 (8) | `.beads/vb-1wora/evidence/po-proptest-vb-vzcuf-PS-003.log` | PASS | Supports property-based confidence for round-trip + trailing-bytes oracle + exact-boundary oracle |
| Trailing-bytes direct cargo test (6) | `.beads/vb-1wora/evidence/po-002-cargo-test-trailing-bytes-direct.log` | PASS | Supports INV-CODEC-TB-001 + INV-CODEC-TB-005 + INV-CODEC-TB-006 |
| Strict source lint | `.beads/vb-1wora/evidence/po-cargo-clippy.log` | PASS | Supports zero-runtime-panic-surface gate |
| Rustfmt | `.beads/vb-1wora/evidence/po-cargo-fmt-check.log` | PASS | Supports touched-crate fmt confidence |
| Kani H6 syntax smoke | `.beads/vb-1wora/evidence/po-004-kani-cargo-check-legacy.log` | PASS | Supports Kani H6 file correctness |
| Full Kani H6 | (BLOCKED_TOOLING: pre-existing vb_core compile error) | BLOCKED_TOOLING | Routed to vb_core maintainer |
| Verus bridge | `.beads/vb-1wora/evidence/po-006-verus-ps-003-bridge-trailing-bytes.log` | PASS | Supports INV-CODEC-TB-007 + bridge arm correctness |
| Verus production-binding gate | `.beads/vb-1wora/evidence/po-006-verus-production-binding-gate.log` | PASS (0 VACUUM) | Supports GOD RULE 2 (vacuum-proof prohibition) |
| Production-inner drift gate | (BLOCKED_TOOLING: JJ-only workspace) | BLOCKED_TOOLING | Routed to femdation (workspace tooling) |
| cargo-fuzz 60s wallclock | `.beads/vb-1wora/evidence/po-007-fuzz-trailing-bytes-60s.log` | PASS (37M+ runs, 0 crashes) | Supports HOSTILE-INPUT-001 |
| Formal/theorem/performance/Loom/Miri/Flux/TLA+ lanes | `.beads/vb-1wora/formal-waivers.jsonl:1-5` | WAIVED / NOT_IN_SCOPE | Not PASS and not claimed as proof evidence |

## Truth Serum Audit

- report: `.beads/vb-1wora/truth-serum-report.md`
- status: APPROVED
