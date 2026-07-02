# Proof → Implementation Input — vb-1wora

**Bead:** `vb-1wora` — Codec: reject trailing bytes after declared record payload (P1 bug)
**State:** 4 (proof-planner handoff)
**Downstream consumers:** `proof-to-implementation` (State 7), `test-planner` / `test-writer` (States 5–6), `holzman-rust` / `landing-skill` (States 8–12).

This file is the bridge from the 7 proof obligations (POB-vb-1wora-001..007) to the production source sites, test fixtures, and gate commands that the proof-writer and proof-to-implementation skills must author / verify. It does **not** approve the bridge — that is `proof-to-implementation`'s job at State 7.

---

## 1. Production Source Sites (per obligation)

| Production site | Path:line | Obligation | What the production site must do |
|---|---|---|---|
| `decode_record_payload` | `crates/vb_storage/src/codec/payload.rs:56-82` | POB-001, POB-002, POB-003, POB-004, POB-005, POB-006, POB-007 | Insert the trailing-bytes check between `bytes.get(payload_start..payload_end).ok_or(UnexpectedEof)?;` (line 71 area) and `verify_digest_match(payload, header.payload_digest)?;` (line 76 area). The check must be `if bytes.len() > payload_end { return Err(JournalError::TrailingBytes { trailing: bytes.len() - payload_end }); }`. |
| `decode_envelope_only` | `crates/vb_storage/src/codec/envelope.rs:48-83` | POB-002, POB-005 | Mirror the same check at the same position relative to `verify_digest_match` (line 78 area). |
| `JournalError::TrailingBytes` | `crates/vb_storage/src/error/mod.rs:~97` (between `UnexpectedEof` and `MalformedKeyspaceRow`) | POB-002, POB-006 | Add `#[error("trailing bytes after declared payload: {trailing}")] TrailingBytes { trailing: usize }`. |
| `TRAILING_BYTES_CODE` | `crates/vb_storage/src/error/codes.rs:~50` (next to `UNEXPECTED_EOF_CODE`) | POB-001, POB-002 | Add `pub const TRAILING_BYTES_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);`. |
| `diagnostic_code()` match arm | `crates/vb_storage/src/error/codes.rs:99-176` | POB-001, POB-002 | Add `Self::TrailingBytes { .. } => Self::TRAILING_BYTES_CODE,`. |
| `symbolic_code()` match arm | `crates/vb_storage/src/error/codes.rs:180-268` | POB-001 | Add `Self::TrailingBytes { .. } => "JOURNAL_TRAILING_BYTES",`. |
| Verus bridge arm | `verification/verus/vb-vzcuf-PS-003.rs:387-451` (`assume_specification[ production::decode_record ]` ensures) | POB-006 | Add `Err(SpecJournalError::TrailingBytes { trailing }) => { &&& (bytes.len() as u32) > expected_payload_end &&& trailing == (bytes.len() as u32) - expected_payload_end &&& trailing > 0 },` arm. The `expected_payload_end` is a top-level bridge parameter (mirroring `header_ok`). |
| Verus mirror variant | `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:335-413` (`SpecJournalError` enum) | POB-006 | Add `TrailingBytes { trailing: u32 }` variant; update enumeration comment at lines 280-327 to include the new variant. |
| Drift-gate header | `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:1-26` | POB-006 | Update the binding-ledger at lines 63-95 to note the new mirror variant is regenerated from `crates/vb_storage/src/error/mod.rs:~97`. |

## 2. Test Sites (per obligation)

| Test site | Path:line | Obligation | Assertion |
|---|---|---|---|
| `decode_rejects_trailing_bytes_after_payload` | `crates/vb_storage/src/codec/tests.rs:1498-1524` (renamed from `decode_ignores_trailing_bytes_beyond_payload`) | POB-002 | `assert!(matches!(result, Err(JournalError::TrailingBytes { trailing: 3 })))` on the existing `0xFF 0xFE 0xFD` 3-byte fixture (after a valid `JournalEvent::RunCancelled` record). |
| `decode_envelope_only_rejects_trailing_payload` | `crates/vb_storage/src/codec/envelope.rs:153-170` (sibling of `decode_envelope_only_rejects_truncated_payload`) | POB-002 | `assert!(matches!(result, Err(JournalError::TrailingBytes { trailing: 4 })))` on a valid record + 4 appended bytes. |
| `trailing_bytes_variant_and_fields` | `crates/vb_storage/src/error_tests.rs:~454` (new, mirrors `InvalidGateCount` pattern at 454-511) | POB-002 | Field round-trip and pattern-match on `TrailingBytes { trailing: 5 }`. |
| `trailing_bytes_display_format` | `crates/vb_storage/src/error_tests.rs:~480` (new, mirrors `InvalidGateCount` pattern at 454-511) | POB-002 | Display contains "trailing" and the byte count. |
| `trailing_bytes_error_code` | `crates/vb_storage/src/error_tests.rs:~510` (new, mirrors `InvalidGateCount` pattern at 454-511) | POB-002 | `err.diagnostic_code() == TRAILING_BYTES_CODE` and `TRAILING_BYTES_CODE == DiagnosticCode::new(0x4042)`. |
| `trailing_bytes_error_has_correct_code` | `crates/vb_storage/src/error_code_tests.rs:~144` (new, mirrors `payload_too_large_error_has_correct_code`) | POB-002 | `JournalError::TrailingBytes { trailing: 100 }.diagnostic_code() == JournalError::TRAILING_BYTES_CODE`. |
| Audit header update | `crates/vb_storage/src/error_tests.rs:14-62` | POB-001 | Move `TrailingBytes` from the `Untested variants:` block to the `Tested variants:` block. |
| `proptest_trailing_bytes_roundtrip_unchanged` | `crates/vb_storage/src/codec/tests.rs` (new, under `#[cfg(test)] mod proptests`) | POB-003 | 1024 random `JournalEvent` values: encode → decode returns `Ok((env, payload))` with `payload.len() == header.payload_len`. |
| `proptest_decode_record_payload_mutual_exclusion_with_unexpected_eof` | `crates/vb_storage/src/codec/tests.rs` (new, under `#[cfg(test)] mod proptests`) | POB-003, POB-005 | 1024 inputs with `bytes.len() < payload_end`: returns `Err(UnexpectedEof)`, never `Err(TrailingBytes)`. |
| `proptest_decode_record_payload_rejects_random_trailing` | `crates/vb_storage/src/codec/tests.rs` (new) | POB-005 | 1024 inputs with `bytes.len() - payload_end ∈ [1, 32]`: returns `Err(TrailingBytes { trailing: N })` with `N == bytes.len() - payload_end`. |
| `proptest_decode_envelope_only_rejects_random_trailing` | `crates/vb_storage/src/codec/tests.rs` (new) | POB-005 | Mirror of above for `decode_envelope_only`. |
| Kani H6 `kani_harness_rejects_trailing_bytes` | `crates/vb_storage/src/kani_postcard_envelope_wire.rs:~340` (after H5 at line 337) | POB-004 | For any valid header + `payload_len` + `N ∈ [1, 8]` trailing bytes: result is `Err(TrailingBytes { trailing: N })` and `DIGEST_CALL_COUNT == 0`. |
| `fuzz_target_trailing_bytes` | `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs:~70` (new function, alongside existing fuzz loops at lines 43-66) | POB-007 | Build valid record + append `N ∈ [0, 8]` random bytes → assert `Err(TrailingBytes)` when `N > 0` and `Ok` when `N == 0`. |

## 3. Gate Commands (per obligation)

| Obligation | Gate command | Expected exit code | Evidence path |
|---|---|---|---|
| POB-001 (rust-local) | `diff -u <(git show HEAD:crates/vb_storage/src/codec/payload.rs) crates/vb_storage/src/codec/payload.rs \| grep -E '(payload_end\|verify_digest_match\|TrailingBytes)' ; diff -u <(git show HEAD:crates/vb_storage/src/codec/envelope.rs) crates/vb_storage/src/codec/envelope.rs \| grep -E '(payload_end\|verify_digest_match\|TrailingBytes)' ; diff -u <(git show HEAD:crates/vb_storage/src/error/codes.rs) crates/vb_storage/src/error/codes.rs \| grep -E '(TRAILING_BYTES_CODE\|Self::TrailingBytes)'` | 0 (grep finds the new code) | captured in review notes; no separate log file |
| POB-002 (cargo test) | `cargo test -p vb_storage --lib decode_rejects_trailing_bytes_after_payload decode_envelope_only_rejects_trailing_payload trailing_bytes_variant_and_fields trailing_bytes_display_format trailing_bytes_error_code trailing_bytes_error_has_correct_code` | 0 | `.beads/vb-1wora/evidence/po-002-cargo-test-trailing-bytes-direct.log` |
| POB-003 (proptest) | `cargo test -p vb_storage --features proptest --lib proptest_trailing_bytes_roundtrip_unchanged proptest_decode_record_payload_mutual_exclusion_with_unexpected_eof` | 0 | `.beads/vb-1wora/evidence/po-003-proptest-roundtrip-mutex.log` |
| POB-004 (kani) | `cargo kani -p vb_storage --harness kani_harness_rejects_trailing_bytes --output-format=json` | 0 | `.beads/vb-1wora/evidence/po-004-kani-h6-trailing-bytes.json` |
| POB-005 (proptest) | `cargo test -p vb_storage --features proptest --lib proptest_decode_record_payload_rejects_random_trailing proptest_decode_envelope_only_rejects_random_trailing` | 0 | `.beads/vb-1wora/evidence/po-005-proptest-trailing-bytes-oracle.log` |
| POB-006 (verus) | `bash scripts/verify-verus.sh` ; `bash scripts/check-verus-production-binding.sh` ; `bash scripts/check-production-inner-drift.sh` | 0, 0, 0 | `.beads/vb-1wora/evidence/po-006-verus-ps-003-bridge-trailing-bytes.log`, `po-006-verus-production-binding-gate.log`, `po-006-verus-drift-gate.log` |
| POB-007 (cargo-fuzz) | `cargo +nightly fuzz run -p vb_storage_fuzz fuzz_target_trailing_bytes -- -max_total_time=60` | 0 | `.beads/vb-1wora/evidence/po-007-fuzz-trailing-bytes-60s.log` |

## 4. Independent Behavior Tests (per obligation)

These tests are independent of the implementation under test and serve as the "oracle" that the implementation must match. The test-writer owns authoring these; the proof-to-implementation skill verifies that each obligation has at least one independent behavior test.

| Obligation | Independent behavior test | Path | Why independent |
|---|---|---|---|
| POB-001 (rust-local) | N/A (structural review is the test) | — | — |
| POB-002 (cargo test) | `decode_rejects_trailing_bytes_after_payload` | `crates/vb_storage/src/codec/tests.rs:1498-1524` | Uses `decode_record_payload` directly with a hand-crafted `0xFF 0xFE 0xFD` fixture; does not depend on the implementation of the trailing-bytes check itself. |
| POB-003 (proptest) | `proptest_trailing_bytes_roundtrip_unchanged` + `proptest_decode_record_payload_mutual_exclusion_with_unexpected_eof` | `crates/vb_storage/src/codec/tests.rs` (new) | Uses `encode_record` to produce the fixture (independent of `decode_record_payload`'s internal check), then decodes and asserts the outcome. The mutual-exclusion proptest uses `bytes.len() < payload_end` directly without involving the encoder. |
| POB-004 (kani) | `kani_harness_rejects_trailing_bytes` | `crates/vb_storage/src/kani_postcard_envelope_wire.rs:~340` | Uses `kani::any()` to construct the input symbolically (independent of any specific fixture); the stub `verify_digest_match` is the only implementation-dependent piece, and the stub is in the harness file itself, not in production. |
| POB-005 (proptest) | `proptest_decode_record_payload_rejects_random_trailing` + `proptest_decode_envelope_only_rejects_random_trailing` | `crates/vb_storage/src/codec/tests.rs` (new) | Uses `encode_record` to produce a valid record (independent of `decode_record_payload`'s internal check) and proptest's `Vec<u8>` arbitrary generator for trailing bytes. |
| POB-006 (verus) | The Verus bridge `assume_specification[ production::decode_record ]` exec wrapper at `verification/verus/vb-vzcuf-PS-003.rs:480+` | `verification/verus/vb-vzcuf-PS-003.rs` | The exec wrapper invokes `production::decode_record` (which is the WEAK_MIRROR-bound production function) and asserts the `ensures` clause; the bridge arm is the oracle. |
| POB-007 (cargo-fuzz) | `fuzz_target_trailing_bytes` | `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs:~70` | Uses `encode_record` to produce the fixture (independent of `decode_record_payload`'s internal check) and libFuzzer's random byte generator for trailing bytes. |

## 5. Bridge Notes

- **Production-binding mechanism for POB-006:** `WEAK_MIRROR` via `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs` (already mirrored from `crates/vb_storage/src/codec/mod.rs:1-100`, `codec/payload.rs`, `codec/header.rs`, `codec/kind_parity.rs`). Drift gate: `scripts/check-production-inner-drift.sh` (zero-drift tolerance). Production-binding gate: `scripts/check-verus-production-binding.sh` (bridge arm enumeration parity). The exec wrapper at `vb-vzcuf-PS-003.rs:480+` invokes `production::decode_record` (via the extern shim at `verification/verus/extern_vb_vzcuf_PS_003.rs`) and asserts the `ensures` clause.
- **Cross-crate boundary:** the symbolic-code registration in `crates/vb_core/src/diagnostic.rs::CODE_REGISTRY` is recommended-only and is NOT a bridge obligation. If the registration is added, it is a one-line `("JOURNAL_TRAILING_BYTES", DiagnosticCode::new(0x4042))` append to the registry slice.
- **Kani H6 reuse:** H6 inherits the `#[kani::unwind(4)]` and `cargo kani -p vb_storage --harness ...` invocation pattern from H5 (`crates/vb_storage/src/kani_postcard_envelope_wire.rs:271-337`). The stub-and-count technique for proving step-ordering is standard.
- **Proptest reuse:** `proptest::collection::vec(kani::any::<u8>(), 0..=32)` is the existing pattern for generating variable-length trailing byte arrays in this codebase.

## 6. Forbidden Implementation Patterns

These patterns are forbidden by `contracts/contract.md §9` and `contracts/hazard-analysis.md §2.7`. The proof-to-implementation skill MUST reject any patch that introduces them.

| Pattern | Why forbidden | Detected by |
|---|---|---|
| `unwrap()`, `expect()`, `panic!()`, `todo!()`, `dbg!()` in the post-fix decode path. | AGENTS.md doctrine. | `cargo clippy -- -D warnings` (source lint) |
| Modifying `encode_record` / `encode_record_payload` to "balance" the new check. | Encoder is correct; modifying it risks round-trip breakage. | POB-003 + POB-005 round-trip properties |
| Two `JournalError` variants both reachable on `bytes.len() > payload_end`. | Violates mutual-exclusion invariant `INV-CODEC-TB-009`. | POB-002, POB-003, POB-005 |
| `TrailingBytes { trailing: 0 }`. | Violates `INV-CODEC-TB-005`. | POB-004 (Kani), POB-005 (proptest) |
| Hand-written shadow types without `#[path = "..."]` binding in the Verus mirror. | GOD RULE 2 (vacuum-proof prohibition). | `scripts/check-verus-production-binding.sh` (POB-006) |
| Numeric codes outside the `0x40xx` journal range for storage-layer errors. | Existing convention. | POB-002 (`trailing_bytes_error_has_correct_code`) |
| Trailing-bytes check placed *after* `verify_digest_match`. | Violates `INV-CODEC-TB-003`. | POB-001 (structural), POB-004 (Kani) |

## 7. Handoff to proof-to-implementation (State 7)

- The bridge map above (sections 1–4) is the input. The proof-to-implementation skill must verify that every obligation's `target` field has at least one independent behavior test, and that the gate command captures the right evidence.
- The `production_binding` block in `proof-obligations.planned.jsonl:POB-vb-1wora-006` is the WEAK_MIRROR declaration; the proof-to-implementation skill must verify the drift gate (`scripts/check-production-inner-drift.sh`) is wired into the CI pipeline and that the exec wrapper at `vb-vzcuf-PS-003.rs:480+` actually invokes the production function.
- All other obligations are non-Verus and do not require production-binding declarations.