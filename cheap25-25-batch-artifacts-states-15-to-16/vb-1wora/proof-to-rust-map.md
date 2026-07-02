# Proof-to-Rust Map: vb-1wora (State 7)

```
bridge_skill: proof-to-implementation
bridge_invocation_id: vb-1wora-state7-proof-to-implementation-attempt1
proof_review_invocation_id: proof-reviewer-vb-1wora-state6 (APPROVED 2026-07-01)
proof_review_status: APPROVED (5 fixed_with_evidence findings; 0 blockers; 0 VACUUM)
mapping_status: planned
```

## 1. Scope

This State 7 pass bridges the 7 POBs in `proof-obligations.planned.jsonl` to concrete Rust source refs, independent behavior tests, separate refinement harness refs, and exact evidence commands. The 7 RROs in `rust-refinement-obligations.jsonl` are the canonical bridge output.

| Proof ID | Lane | RRO ID | Verifier | Status |
|---|---|---|---|---|
| POB-vb-1wora-001 | rust-local (structural) | RRO-001 | rust-local | planned |
| POB-vb-1wora-002 | cargo-test | RRO-002 | cargo-test | planned |
| POB-vb-1wora-003 | proptest round-trip | RRO-003 | proptest | planned |
| POB-vb-1wora-004 | kani H6 | RRO-004 | kani | planned |
| POB-vb-1wora-005 | proptest trailing-bytes oracle | RRO-005 | proptest | planned |
| POB-vb-1wora-006 | verus bridge | RRO-006 | verus | planned |
| POB-vb-1wora-007 | cargo-fuzz | RRO-007 | cargo-fuzz | planned |

## 2. God Rule Compliance

| Check | Result | Evidence |
|---|---|---|
| No behavior-affecting waivers (GOD RULE 5) | PASS | 8 trust ledger rows are non-behavior (`SPEC_BINDING`, `TOOLING_BLOCK`, `PRODUCTION_BINDING`, `SYMBOLIC_EXECUTION_BOUND`, `SMOKE_EVIDENCE`); no `E_BEHAVIOR_WAIVER` rows in `trusted-base-ledger.jsonl`. |
| Verus production-binding present (GOD RULE 2) | PASS | RRO-006 binds via `verification/verus/vb-vzcuf-PS-003.rs:387-451` bridge arm + `wrapper_decode_record_trailing_bytes` exec wrapper; mirror variant at `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:403`; extern shim at `verification/verus/extern_vb_vzcuf_PS_003.rs:71-72`. Production-binding gate `check-verus-production-binding.sh`: STRONG:0, WEAK:71, VACUUM:0. WEAK_MIRROR binding classification is correct (production transitively reaches `postcard`, `serde`, `blake3`, `crc32c`). |
| No VACUUM escape hatches (GOD RULE 2 corollary) | PASS | No `EXPLICITLY_ALLOWED`, no `ALLOWED_EXCEPTIONS`, no `OFFLOAD`. The new bridge arm enumerates `Err(SpecJournalError::TrailingBytes { trailing })` with concrete trigger clauses and is exercised by the new exec wrapper. |
| Kani hardcoded shapes prohibited (GOD RULE 1) | PASS | Kani H6 (`kani_harness_rejects_trailing_bytes` at `crates/vb_storage/src/kani_postcard_envelope_wire.rs:339-453`) uses `kani::any()` for header bytes, `valid_magic`, `payload_len`, payload bytes, and trailing bytes. Only the trailing count is concrete (1..=8 per proof-strategy §2.5 A-003). `kani::cover!` for non-vacuity on the TrailingBytes arm; `kani::assert` for actual == trailing_len and actual > 0; `Err(_)` arms call `kani::assert(false, ...)` so any non-TrailingBytes outcome is a verification failure. |
| TLA+ bounded arithmetic (GOD RULE 3) | N/A | TLA+ lane `not_applicable` per `VLD-vb-1wora-010-tla-plus`; no TLA+ artifacts. |
| No loop oscillation (GOD RULE 4) | PASS | The trailing-bytes check is a single `if` + `Err` return; no new loop introduced. `#[kani::unwind(4)]` is inherited from H5 (sufficient). Fuzz oracle uses `for n in 0u32..=8u32` (9 iterations, no recursion). |
| Differential verification only (GOD RULE 5 corollary) | PASS | Trimmed scope: 3 proof artifacts (Verus PS-003, Kani H6, fuzz oracle) covering exactly the 7 INV-CODEC-TB-* invariants + HOSTILE-INPUT-001; no fleet-wide blind mutation. |

## 3. Bridge Standard Compliance

| Check | Result | Evidence |
|---|---|---|
| `source_refs` name production symbols (not files) | PASS | All 7 RROs use `path::symbol` format with line ranges (e.g. `crates/vb_storage/src/codec/payload.rs::decode_record_payload:56-82`). |
| `behavior_test_refs` are executable | PASS | 6 of 7 RROs cite specific test functions (cargo-test, proptest, or fuzz oracle). RRO-001 is structural-only (no behavior test applies; the diff review IS the test per proof-to-implementation-input.md §4). |
| `refinement_harness_refs` separate from behavior tests | PASS | 3 of 7 RROs cite refinement harnesses (Kani H6, Verus bridge+wrapper, fuzz target). The 4 other RROs (rust-local, cargo-test, proptest x2) are behavior-only; no refinement harness applies. |
| Verifier harnesses NOT reused as behavior tests | PASS | Kani H6 is a refinement harness; its companion cargo-test (`decode_rejects_trailing_bytes_after_payload`) is the independent behavior oracle. Verus `wrapper_decode_record_trailing_bytes` is a refinement exec wrapper; its companion cargo-test trio is the independent behavior oracle. Fuzz `fuzz_target!` body is a refinement harness; its companion cargo-test and proptest are the independent behavior oracles. |
| No file-only refs, no prose refs | PASS | Every `source_refs`, `behavior_test_refs`, and `refinement_harness_refs` entry includes a `path::symbol` (or `path::symbol:line` or `path::symbol:line-range`) reference. |
| No behavior-affecting waivers | PASS | No `E_BEHAVIOR_WAIVER` rows in `trusted-base-ledger.jsonl`. `waiver-candidates.jsonl` has no behavior-affecting entries. |
| Forbidden-pattern compliance | PASS | No `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg` in proof artifacts. The fuzz oracle's `panic!` is a fuzzer-counterexample trap, not a runtime panic (fuzz-target convention). |

## 4. Production-Binding Audit (GOD RULE 2 enforcement)

```
$ bash scripts/check-verus-production-binding.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora
================================================================
  Verus production-binding audit (ABSOLUTE — no exceptions)
================================================================
  STRONG (direct crates/ binding): 0
  WEAK (production_inner/ mirror): 71
  VACUUM (no production binding):  0
exit=0
```

- 0 VACUUM files. The new `Err(SpecJournalError::TrailingBytes { trailing })` arm is NOT in the VACUUM bucket. PASS.
- 71 WEAK files. PS-003 spec is one of the 71 (binds via `extern_vb_vzcuf_PS_003.rs` → `production_inner/vb_vzcuf_PS_003_production.rs`).
- 0 STRONG files. STRONG is correctly rejected (transitive dependencies on `postcard`, `serde`, `blake3`, `crc32c` are not Verus-modelable in single-file `verus --crate-type=lib`; documented in `extern_vb_vzcuf_PS_003.rs:1-46`).
- The new bridge arm does NOT introduce a new WEAK file or a new VACUUM file. The pre-existing WEAK classification of PS-003 is preserved.

## 5. Drift Gate (BLOCKED_TOOLING — honest accounting)

```
$ bash scripts/check-production-inner-drift.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora
fatal: not a git repository (or any parent up to mount point /)
Stopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set).
```

The script requires `git rev-parse --show-toplevel` to resolve to a git repo. The isolated workspace is JJ-only. The drift gate is correctly documented as `BLOCKED_TOOLING` in `trusted-base-ledger.jsonl` TL-002. The reviewer must re-run the drift gate in the main checkout or in a git-initialized worktree post-fix to confirm zero drift.

**Mitigation:** the new `TrailingBytes` variant was added to the mirror at the documented location (between `UnexpectedEof` and `PostcardDecodeFailed`, mirroring the production-side placement between `UnexpectedEof` and `MalformedKeyspaceRow` per `type-contracts.md §1.3` and `contract.md §4.1`). The diff between the new mirror and the (post-fix) production source is limited to the new variant; the rest of the file is unchanged. The formal-verifier (State 12) is responsible for the post-fix re-run.

## 6. Forbidden-Implementation-Pattern Coverage

| Pattern (per `contracts/contract.md §9`) | Bridge coverage |
|---|---|
| `unwrap()`/`expect()`/`panic!()`/`todo!()`/`unimplemented()`/`dbg!()` in post-fix decode path | RRO-001 (rust-local structural review will catch any forbidden construct introduced in the diff). |
| Modifying `encode_record`/`encode_record_payload` to balance the new check | RRO-003 (round-trip proptest catches any encoder modification that breaks round-trip). |
| Two `JournalError` variants both reachable on `bytes.len() > payload_end` | RRO-002 (cargo-test: `decode_rejects_trailing_bytes_after_payload` only matches `TrailingBytes`, not other variants), RRO-003 + RRO-005 (proptest mutual-exclusion property). |
| `TrailingBytes { trailing: 0 }` | RRO-004 (Kani H6 asserts `actual > 0`), RRO-005 (proptest asserts `trailing > 0` for all generated cases), RRO-002 (cargo-test asserts `trailing: 3` for 3-byte fixture). |
| Hand-written shadow types without `#[path = "..."]` binding in the Verus mirror | RRO-006 (WEAK_MIRROR with `#[path = "production_inner/..."]` binding; `check-verus-production-binding.sh` exit=0, 0 VACUUM). |
| Numeric codes outside the `0x40xx` journal range for storage-layer errors | RRO-002 (`trailing_bytes_error_code` asserts `TRAILING_BYTES_CODE == DiagnosticCode::new(0x4042)`). |
| Trailing-bytes check placed *after* `verify_digest_match` | RRO-001 (structural review of diff), RRO-004 (Kani H6 asserts `verify_digest_match` call count is 0 when trailing-bytes arm fires). |

## 7. Per-OBLIGATION Disposition

| POB | RRO | Source refs (count) | Behavior test refs (count) | Refinement harness refs (count) | Verifier | Evidence artifact | Status |
|---|---|---|---|---|---|---|---|
| POB-001 | RRO-001 | 6 | 0 (N/A structural) | 0 (N/A structural) | rust-local | diff captured by review notes | planned |
| POB-002 | RRO-002 | 6 | 6 | 0 (behavior IS test) | cargo-test | `.beads/vb-1wora/evidence/po-002-cargo-test-trailing-bytes-direct.log` | planned |
| POB-003 | RRO-003 | 5 | 2 | 0 (proptest generator IS harness) | proptest | `.beads/vb-1wora/evidence/po-003-proptest-roundtrip-mutex.log` | planned |
| POB-004 | RRO-004 | 5 | 2 | 1 (Kani H6) | kani | `.beads/vb-1wora/evidence/po-004-kani-h6-trailing-bytes.json` | planned |
| POB-005 | RRO-005 | 5 | 3 | 0 (proptest generator IS harness) | proptest | `.beads/vb-1wora/evidence/po-005-proptest-trailing-bytes-oracle.log` | planned |
| POB-006 | RRO-006 | 8 | 4 | 2 (Verus bridge + exec wrapper) | verus | `.beads/vb-1wora/evidence/po-006-verus-ps-003-bridge-trailing-bytes.log` + `po-006-verus-production-binding-gate.log` + `po-006-verus-drift-gate.log` | planned |
| POB-007 | RRO-007 | 5 | 4 | 1 (fuzz target sub-oracle) | cargo-fuzz | `.beads/vb-1wora/evidence/po-007-fuzz-trailing-bytes-60s.log` | planned |

## 8. Cross-Artifact Consistency (Hazard: harness/test overlap)

For each POB, the bridge ensures `behavior_test_refs` and `refinement_harness_refs` point to **different files** (or that the proptest is its own harness):

| POB | Behavior test file | Refinement harness file | Same? |
|---|---|---|---|
| POB-001 | (none) | (none) | N/A structural |
| POB-002 | `codec/tests.rs`, `codec/envelope.rs`, `error_tests.rs`, `error_code_tests.rs` | (none; behavior IS test) | N/A |
| POB-003 | `tests/proptest_vb_1wora_roundtrip.rs` (new) | (proptest generator IS harness) | N/A |
| POB-004 | `codec/tests.rs::decode_rejects_trailing_bytes_after_payload`, `codec/envelope.rs::decode_envelope_only_rejects_trailing_payload` | `kani_postcard_envelope_wire.rs::kani_harness_rejects_trailing_bytes` | NO (distinct files) |
| POB-005 | `tests/proptest_vb_1wora_trailing_bytes_oracle.rs` (new) | (proptest generator IS harness) | N/A |
| POB-006 | `codec/tests.rs`, `codec/envelope.rs`, `error_tests.rs`, `error_code_tests.rs` | `verification/verus/vb-vzcuf-PS-003.rs` (bridge + exec wrapper) | NO (distinct files) |
| POB-007 | `codec/tests.rs`, `codec/envelope.rs`, `error_tests.rs`, `error_code_tests.rs` | `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs` (existing target + new sub-oracle) | NO (distinct files) |

No RRO has `behavior_test_refs` and `refinement_harness_refs` pointing to the same file when both are non-empty.

## 9. Evidence Command Coverage (Rerun State)

| POB | Evidence command | Rerun state |
|---|---|---|
| POB-001 | `diff -u <(git show HEAD:...) ... \| grep -E '(payload_end\|verify_digest_match\|TrailingBytes)'` x3 + diagnostic-const check | Re-runnable in a git-initialized checkout at State 12 (BLOCKED_TOOLING in JJ-only workspace per TL-002). |
| POB-002 | `cargo test -p vb_storage --lib decode_rejects_trailing_bytes_after_payload decode_envelope_only_rejects_trailing_payload trailing_bytes_variant_and_fields trailing_bytes_display_format trailing_bytes_error_code trailing_bytes_error_has_correct_code` | Re-runnable at State 12 once the post-fix production code lands. |
| POB-003 | `cargo test -p vb_storage --features proptest --test proptest_vb_1wora_roundtrip -- proptest_trailing_bytes_roundtrip_unchanged proptest_decode_record_payload_mutual_exclusion_with_unexpected_eof` | Re-runnable at State 12 once the new proptest file lands. |
| POB-004 | `cargo kani -p vb_storage --harness kani_harness_rejects_trailing_bytes --output-format=json` | BLOCKED_TOOLING by pre-existing `vb_core/src/frame/parts/kani_helpers.rs:22` compile error (TL-003, pre-existing; routed to vb_core maintainer). Re-runnable once vb_core kani_helpers is repaired. |
| POB-005 | `cargo test -p vb_storage --features proptest --test proptest_vb_1wora_trailing_bytes_oracle -- proptest_decode_record_payload_rejects_random_trailing proptest_decode_envelope_only_rejects_random_trailing proptest_decode_record_payload_mutual_exclusion_with_unexpected_eof` | Re-runnable at State 12 once the new proptest file lands. |
| POB-006 | `bash scripts/verify-verus.sh` + `bash scripts/check-verus-production-binding.sh` + `bash scripts/check-production-inner-drift.sh` | Re-runnable at State 12 (verify-verus and production-binding are green today; drift gate is BLOCKED_TOOLING per TL-002 and requires a git-initialized checkout). |
| POB-007 | `cargo +nightly fuzz run -p vb_storage_fuzz fuzz_storage_codec_payload_corruption -- -max_total_time=60` | Re-runnable at State 12 once the post-fix `JournalError::TrailingBytes` variant lands (today the fuzz target fails compile with `no variant named TrailingBytes found for enum JournalError` per TL-004). |

## 10. Mapping Artifacts

- `proof-to-rust-map.md` (this file)
- `rust-refinement-obligations.jsonl` (7 RRO rows)
- `proof-to-rust-review.md` (APPROVED, written by proof-reviewer self-review at State 7; bridge is internally consistent)

## 11. Unresolved Mapping Gaps

| Gap | Severity | Owner | Mitigation |
|---|---|---|---|
| Drift gate `check-production-inner-drift.sh` cannot run in JJ-only workspace. | low (informational) | State 12 formal-verifier | Re-run in a git-initialized checkout post-fix. The bridge does not pretend drift has been mechanically verified; the TL-002 BLOCKED_TOOLING entry is honest. |
| Kani H6 full `cargo kani` execution blocked by unrelated `vb_core/src/frame/parts/kani_helpers.rs:22` compile error. | low (informational) | vb_core maintainer | Re-run when vb_core kani_helpers is repaired (TL-003, pre-existing). The bridge does not pretend Kani H6 has been mechanically verified beyond the syntax smoke; the TL-003 BLOCKED_TOOLING entry is honest. |
| Fuzz target `cargo check` reports 1 expected pre-fix error. | low (informational) | State 11 holzman-rust | The error resolves when the post-fix `JournalError::TrailingBytes` variant lands. TL-004 documents the pre-fix state. |
| Proptest file path choice (`tests/proptest_vb_1wora_<topic>.rs` instead of `mod proptests` in `codec/tests.rs` per the proof-to-implementation-input.md §2). | low (organizational) | State 11 holzman-rust / test-writer | The proof-to-implementation-input.md §2 row for POB-003/005 suggests inlining proptests under `#[cfg(test)] mod proptests` in `codec/tests.rs`; the bridge deviates to the existing codebase pattern (`tests/proptest_vb_<bead>_<topic>.rs`) because no proptests currently live in `codec/tests.rs` and the per-bead test-file convention is well-established (see `proptest_vb_vzcuf_PS_001..PS_009.rs`, `proptest_storage.rs`, `proptests.rs`). The deviation is documented in RRO-003/005 notes. The test-writer is free to inline if the test-writer's plan-review approves. |
| Verus `expected_payload_end: u32` modeling decision vs production `trailing: usize`. | low (modeling decision) | State 11 holzman-rust | Verus uses `u32`; production runtime uses `usize`. Values agree for any platform where `usize == 32 bits` (32-bit targets) or values `< 2^32` (any platform). For values `>= 2^32` the cast would saturate/truncate; in practice, `bytes.len() - payload_end` cannot exceed `u32::MAX` because the input slice is size-bounded by the codec's payload-length cap. Documented in proof-writer-report §3 and TL-006. |
| Symbolic-code registration in `vb_core::CODE_REGISTRY` is recommended but not mandatory. | low (non-blocking) | State 11 holzman-rust | Per `contracts/contract.md §6.3`, the symbolic code `"JOURNAL_TRAILING_BYTES"` falls back to `INTERNAL_INVARIANT` in `symbolic_code()` until the registry is updated. This is non-blocking for the bridge: the bridge covers the production wiring (`crates/vb_storage/src/error/codes.rs` `symbolic_code()` match arm), not the cross-crate registry. |

## 12. Required Follow-ups (NOT blocking this state)

1. (Verifier/State 12) Re-run `bash scripts/check-production-inner-drift.sh` in a git-initialized checkout post-fix to confirm zero drift between the new mirror and the post-fix production source.
2. (Verifier/State 12) Re-run `bash scripts/verify-verus.sh` for the registry-driven Verus run (smoke already passes 25/25 with the new exec wrapper).
3. (Verifier/State 12) Execute `cargo kani -p vb_storage --harness kani_harness_rejects_trailing_bytes --output-format=json` after the unrelated `kani_helpers.rs:22` compile error is fixed by the vb_core maintainer.
4. (Verifier/State 12) Execute `cargo +nightly fuzz run -p vb_storage_fuzz fuzz_storage_codec_payload_corruption -- -max_total_time=60` after the production-side `TrailingBytes` variant lands.
5. (Verifier/State 12) Execute all cargo-test and proptest evidence commands and capture raw logs under `.beads/vb-1wora/evidence/po-00X-*.log`.

## 13. Exact Handoff Inputs for proof-reviewer (self-review at State 7)

1. `proof-to-rust-map.md` (this file, 13 sections, 7-row bridge matrix)
2. `rust-refinement-obligations.jsonl` (7 RRO rows, schema `rust-refinement-obligation/v1`, all `mapping_status: planned`, all `required: true`, all `behavior_affecting: true`)
3. `proof-to-rust-review.md` (written by self-review, STATUS: APPROVED)
4. `proof-to-implementation-input.md` (the State 4 handoff)
5. `proof-obligations.planned.jsonl` (7 POB rows, schema `proof-obligation/v1`)
6. `proof-review.md` (State 6 APPROVED, 5 fixed_with_evidence findings)
7. `proof-findings.jsonl` (5 fixed_with_evidence findings, 0 blockers)
8. `trusted-base-ledger.jsonl` (8 rows, schema `trusted-base-ledger/v1`)
9. `contract.md` (the State 3 master contract)
10. `proof-seeds.jsonl` (the State 3 proof seeds)
11. `traceability-matrix.jsonl` (the State 3 invariant ↔ test ↔ artifact traceability)
12. Production source (post-fix target): `crates/vb_storage/src/codec/payload.rs:56-82` (decode_record_payload), `crates/vb_storage/src/codec/envelope.rs:48-83` (decode_envelope_only), `crates/vb_storage/src/error/mod.rs:97` (JournalError::TrailingBytes), `crates/vb_storage/src/error/codes.rs:50` (TRAILING_BYTES_CODE), `crates/vb_storage/src/error/codes.rs:99-176` (diagnostic_code match arm), `crates/vb_storage/src/error/codes.rs:180-268` (symbolic_code match arm).
13. Production source (pre-fix): `crates/vb_storage/src/codec/tests.rs:1498-1524` (existing `decode_ignores_trailing_bytes_beyond_payload` test to be renamed + inverted).

---

## Proof-to-Rust Matrix (one row per POB)

| Proof ID | Claim (verbatim from proof-strategy.md) | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| POB-vb-1wora-001 | PS-VB-1WORA-003: Trailing-bytes check precedes verify_digest_match (cheap-before-expensive), and the diagnostic_code()/symbolic_code() match arms for TrailingBytes are wired in lockstep with the TRAILING_BYTES_CODE constant. | true | `crates/vb_storage/src/codec/payload.rs::decode_record_payload:56-82`; `crates/vb_storage/src/codec/envelope.rs::decode_envelope_only:48-83`; `crates/vb_storage/src/error/mod.rs::JournalError::TrailingBytes:97`; `crates/vb_storage/src/error/codes.rs::TRAILING_BYTES_CODE:50`; `crates/vb_storage/src/error/codes.rs::diagnostic_code:99-176`; `crates/vb_storage/src/error/codes.rs::symbolic_code:180-268` | N/A (structural review IS the test) | N/A (structural review IS the test) | rust-local | `diff -u <(git show HEAD:...) ... \| grep -E '(payload_end\|verify_digest_match\|TrailingBytes)'` x3 + diagnostic-const check | 7 |
| POB-vb-1wora-002 | PS-VB-1WORA-001: decode_record_payload returns Err(JournalError::TrailingBytes { trailing }) for any well-formed header followed by trailing junk bytes; the count field equals bytes.len() - payload_end exactly and is strictly > 0; the same invariant holds for decode_envelope_only (mirror site). | true | `crates/vb_storage/src/codec/payload.rs::decode_record_payload:56-82`; `crates/vb_storage/src/codec/envelope.rs::decode_envelope_only:48-83`; `crates/vb_storage/src/error/mod.rs::JournalError::TrailingBytes:97`; `crates/vb_storage/src/error/codes.rs::TRAILING_BYTES_CODE:50`; `crates/vb_storage/src/error/codes.rs::diagnostic_code:99-176`; `crates/vb_storage/src/error/codes.rs::symbolic_code:180-268` | `crates/vb_storage/src/codec/tests.rs::decode_rejects_trailing_bytes_after_payload:1498-1524` (renamed+inverted); `crates/vb_storage/src/codec/envelope.rs::decode_envelope_only_rejects_trailing_payload:153-170`; `crates/vb_storage/src/error_tests.rs::trailing_bytes_variant_and_fields:~454`; `crates/vb_storage/src/error_tests.rs::trailing_bytes_display_format:~480`; `crates/vb_storage/src/error_tests.rs::trailing_bytes_error_code:~510`; `crates/vb_storage/src/error_code_tests.rs::trailing_bytes_error_has_correct_code:~144` | N/A (behavior IS test) | cargo-test | `cargo test -p vb_storage --lib decode_rejects_trailing_bytes_after_payload decode_envelope_only_rejects_trailing_payload trailing_bytes_variant_and_fields trailing_bytes_display_format trailing_bytes_error_code trailing_bytes_error_has_correct_code` | 7 |
| POB-vb-1wora-003 | PS-VB-1WORA-002: For any well-formed JournalEvent value, encode_record + decode_record returns Ok((env, payload)) with payload.len() == header.payload_len, and never returns Err(TrailingBytes) on round-trip. Additionally, for any input where bytes.len() < payload_end, the decoder returns Err(UnexpectedEof) — never Err(TrailingBytes) — establishing mutual exclusion. | true | `crates/vb_storage/src/codec/payload.rs::decode_record_payload:56-82`; `crates/vb_storage/src/codec/envelope.rs::decode_envelope_only:48-83`; `crates/vb_storage/src/codec/mod.rs::encode_record:21`; `crates/vb_storage/src/records.rs::RecordKind`; `crates/vb_storage/src/events.rs::JournalEvent` | `crates/vb_storage/tests/proptest_vb_1wora_roundtrip.rs::proptest_trailing_bytes_roundtrip_unchanged` (new); `crates/vb_storage/tests/proptest_vb_1wora_roundtrip.rs::proptest_decode_record_payload_mutual_exclusion_with_unexpected_eof` (new) | N/A (proptest generator IS harness) | proptest | `cargo test -p vb_storage --features proptest --test proptest_vb_1wora_roundtrip -- proptest_trailing_bytes_roundtrip_unchanged proptest_decode_record_payload_mutual_exclusion_with_unexpected_eof` | 7 |
| POB-vb-1wora-004 | PS-VB-1WORA-001 (Kani H6): For any valid 60-byte header followed by an arbitrary payload of length payload_len, with N >= 1 trailing bytes appended (where N <= 8), decode_record_payload returns Err(JournalError::TrailingBytes { trailing: N }) WITHOUT calling verify_digest_match. | true | `crates/vb_storage/src/codec/payload.rs::decode_record_payload:56-82`; `crates/vb_storage/src/codec/payload.rs::verify_digest_match:9-18`; `crates/vb_storage/src/kani_postcard_envelope_wire.rs::kani_harness_rejects_trailing_bytes:339-453`; `crates/vb_storage/src/kani_postcard_envelope_wire.rs::kani_harness_digest_before_postcard:271-337` (H5 sibling); `crates/vb_storage/src/constants.rs::RECORD_HEADER_BYTES + MAX_JOURNAL_EVENT_PAYLOAD_BYTES + DIGEST_BYTES` | `crates/vb_storage/src/codec/tests.rs::decode_rejects_trailing_bytes_after_payload:1498-1524`; `crates/vb_storage/src/codec/envelope.rs::decode_envelope_only_rejects_trailing_payload:153-170` | `crates/vb_storage/src/kani_postcard_envelope_wire.rs::kani_harness_rejects_trailing_bytes:339-453` | kani | `cargo kani -p vb_storage --harness kani_harness_rejects_trailing_bytes --output-format=json` | 7 (BLOCKED_TOOLING: vb_core kani_helpers.rs:22) |
| POB-vb-1wora-005 | PS-VB-1WORA-002 (proptest oracle): For any input where bytes.len() > payload_end (with valid header), both decode_record_payload and decode_envelope_only return Err(JournalError::TrailingBytes { trailing }) where trailing == bytes.len() - payload_end and trailing > 0. For bytes.len() < payload_end, both decoders return Err(UnexpectedEof) — never Err(TrailingBytes). For bytes.len() == payload_end, both decoders return Ok (or an integrity error, never Err(TrailingBytes)). | true | `crates/vb_storage/src/codec/payload.rs::decode_record_payload:56-82`; `crates/vb_storage/src/codec/envelope.rs::decode_envelope_only:48-83`; `crates/vb_storage/src/codec/mod.rs::encode_record:21`; `crates/vb_storage/src/error/mod.rs::JournalError::TrailingBytes:97`; `crates/vb_storage/src/error/mod.rs::JournalError::UnexpectedEof:97` | `crates/vb_storage/tests/proptest_vb_1wora_trailing_bytes_oracle.rs::proptest_decode_record_payload_rejects_random_trailing` (new); `crates/vb_storage/tests/proptest_vb_1wora_trailing_bytes_oracle.rs::proptest_decode_envelope_only_rejects_random_trailing` (new); `crates/vb_storage/tests/proptest_vb_1wora_trailing_bytes_oracle.rs::proptest_decode_record_payload_mutual_exclusion_with_unexpected_eof` (new) | N/A (proptest generator IS harness) | proptest | `cargo test -p vb_storage --features proptest --test proptest_vb_1wora_trailing_bytes_oracle -- proptest_decode_record_payload_rejects_random_trailing proptest_decode_envelope_only_rejects_random_trailing proptest_decode_record_payload_mutual_exclusion_with_unexpected_eof` | 7 |
| POB-vb-1wora-006 | PS-VB-1WORA-007: Verus PS-003 bridge enumerates Err(SpecJournalError::TrailingBytes { trailing: u32 }) as a reachable Err arm with precondition (bytes.len() as u32) > expected_payload_end && trailing == (bytes.len() as u32) - expected_payload_end && trailing > 0. | true | `crates/vb_storage/src/codec/payload.rs::decode_record_payload:56-82` (production WEAK_MIRROR); `crates/vb_storage/src/codec/envelope.rs::decode_envelope_only:48-83`; `crates/vb_storage/src/error/mod.rs::JournalError::TrailingBytes:97`; `verification/verus/vb-vzcuf-PS-003.rs::assume_specification[ production::decode_record ]` (bridge contract, post-fix: 387-451); `verification/verus/vb-vzcuf-PS-003.rs::wrapper_decode_record_trailing_bytes` (new exec wrapper); `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs::SpecJournalError::TrailingBytes`; `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs::decode_record` (mirror exec fn); `verification/verus/extern_vb_vzcuf_PS_003.rs` (extern shim) | `crates/vb_storage/src/codec/tests.rs::decode_rejects_trailing_bytes_after_payload:1498-1524`; `crates/vb_storage/src/codec/envelope.rs::decode_envelope_only_rejects_trailing_payload:153-170`; `crates/vb_storage/src/error_tests.rs::trailing_bytes_variant_and_fields + trailing_bytes_display_format + trailing_bytes_error_code`; `crates/vb_storage/src/error_code_tests.rs::trailing_bytes_error_has_correct_code` | `verification/verus/vb-vzcuf-PS-003.rs::assume_specification[ production::decode_record ]` (post-fix: 387-451); `verification/verus/vb-vzcuf-PS-003.rs::wrapper_decode_record_trailing_bytes` (new exec wrapper) | verus | `bash scripts/verify-verus.sh` + `bash scripts/check-verus-production-binding.sh` + `bash scripts/check-production-inner-drift.sh` | 7 (drift gate BLOCKED_TOOLING: JJ-only workspace) |
| POB-vb-1wora-007 | PS-VB-1WORA-008: For any attacker-shaped input where a valid record is followed by N (0..=8) trailing junk bytes, decode_record_payload returns Err(TrailingBytes { trailing: N }) when N > 0 and Ok (or a different non-TrailingBytes error) when N == 0; no panic, no unwrap/expect failure, no UB. | true | `crates/vb_storage/src/codec/payload.rs::decode_record_payload:56-82`; `crates/vb_storage/src/codec/mod.rs::encode_record:21`; `crates/vb_storage/src/codec/mod.rs::decode_record`; `crates/vb_storage/src/constants.rs::MAGIC_JOURNAL_EVENT + MAX_JOURNAL_EVENT_PAYLOAD_BYTES`; `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs::fuzz_target! body` (existing; extended with trailing-bytes sub-oracle at lines 85-173) | `crates/vb_storage/src/codec/tests.rs::decode_rejects_trailing_bytes_after_payload:1498-1524`; `crates/vb_storage/src/codec/envelope.rs::decode_envelope_only_rejects_trailing_payload:153-170`; `crates/vb_storage/src/error_tests.rs::trailing_bytes_error_code`; `crates/vb_storage/src/error_code_tests.rs::trailing_bytes_error_has_correct_code` | `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs::fuzz_target! body` (lines 85-173 = new sub-oracle) | cargo-fuzz | `cargo +nightly fuzz run -p vb_storage_fuzz fuzz_storage_codec_payload_corruption -- -max_total_time=60` | 7 (BLOCKED_TOOLING: pre-fix JournalError::TrailingBytes not in production) |
