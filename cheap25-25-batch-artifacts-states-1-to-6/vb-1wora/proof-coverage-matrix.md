# Proof Coverage Matrix — vb-1wora

Maps each contract clause / invariant from `contracts/contract.md` and `contracts/hazard-analysis.md` to proof obligations and verifier lanes. Built from `proof-obligations.planned.jsonl` and `verifier-lane-decisions.jsonl`.

## 1. Invariant-Level Coverage

### INV-CODEC-TB-001 — `decode_record_payload` returns `Err(TrailingBytes { trailing })` iff `bytes.len() > payload_end`

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|---|---|---|---|
| `contracts/contract.md §5.1` (post-fix return table, row 3) | POB-vb-1wora-002 | cargo-test (`decode_rejects_trailing_bytes_after_payload`) | planned |
| `contracts/contract.md §5.1` (post-fix return table, row 3) | POB-vb-1wora-004 | kani (`kani_harness_rejects_trailing_bytes` in `kani_postcard_envelope_wire.rs`) | planned |
| `contracts/contract.md §5.1` (hostile-input perspective) | POB-vb-1wora-007 | cargo-fuzz (`fuzz_target_trailing_bytes`) | planned |

### INV-CODEC-TB-002 — `decode_record_payload` returns `Ok((env, payload))` only if `bytes.len() == payload_end`

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|---|---|---|---|
| `contracts/contract.md §5.1` (post-fix return table, row 2) | POB-vb-1wora-003 | cargo-test + proptest (round-trip property) | planned |
| `contracts/contract.md §5.1` (post-fix return table, row 2) | POB-vb-1wora-005 | proptest (random byte-append oracle) | planned |

### INV-CODEC-TB-003 — Trailing-bytes check precedes `verify_digest_match`

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|---|---|---|---|
| `contracts/contract.md §5.3` (ordering) | POB-vb-1wora-001 | rust-local (structural diff review) | planned |
| `contracts/contract.md §5.3` (ordering) | POB-vb-1wora-004 | kani (`kani_harness_rejects_trailing_bytes` counts `verify_digest_match` calls and asserts 0) | planned |

### INV-CODEC-TB-004 — `decode_envelope_only` obeys the same `TrailingBytes` invariant

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|---|---|---|---|
| `contracts/contract.md §4.1` (mirror site) | POB-vb-1wora-002 | cargo-test (`decode_envelope_only_rejects_trailing_payload` sibling test) | planned |
| `contracts/contract.md §4.1` (mirror site) | POB-vb-1wora-005 | proptest (`proptest_decode_envelope_only_rejects_random_trailing`) | planned |

### INV-CODEC-TB-005 — `TrailingBytes { trailing: usize }` reachable only when `trailing > 0`

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|---|---|---|---|
| `contracts/contract.md §5.2` | POB-vb-1wora-002 | cargo-test (`trailing_bytes_variant_and_fields` field round-trip + `trailing_bytes_error_has_correct_code` constant assertion) | planned |
| `contracts/contract.md §5.2` | POB-vb-1wora-004 | kani (H6 reachability claim: arm unreachable when `bytes.len() <= payload_end`) | planned |
| `contracts/contract.md §5.2` | POB-vb-1wora-005 | proptest (input length exactly `payload_end` ⇒ no `TrailingBytes`) | planned |

### INV-CODEC-TB-006 — `TRAILING_BYTES_CODE == DiagnosticCode::new(0x4042)` and the variant maps to it

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|---|---|---|---|
| `contracts/contract.md §6.2` | POB-vb-1wora-001 | rust-local (diagnostic_code() and symbolic_code() match-arm diff review) | planned |
| `contracts/contract.md §6.2` | POB-vb-1wora-002 | cargo-test (`trailing_bytes_error_has_correct_code` in `error_code_tests.rs`) | planned |

### INV-CODEC-TB-007 — Verus PS-003 bridge enumerates `Err(SpecJournalError::TrailingBytes { trailing: u32 })` as a reachable arm

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|---|---|---|---|
| `contracts/contract.md §7.2` (bridge `ensures` arm) | POB-vb-1wora-006 | verus (PS-003 bridge `assume_specification[ production::decode_record ]` adds the new arm; `bash scripts/verify-verus.sh` succeeds; `scripts/check-verus-production-binding.sh` passes; `scripts/check-production-inner-drift.sh` passes) | planned |

### REFINE-MUTEX-001 (from proof-seeds.jsonl) — `TrailingBytes` and `UnexpectedEof` mutually exclusive

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|---|---|---|---|
| `contracts/contract.md §6.4` | POB-vb-1wora-003 | cargo-test (existing round-trip suite + new proptest) | planned |
| `contracts/contract.md §6.4` | POB-vb-1wora-005 | proptest (`proptest_decode_record_payload_mutual_exclusion_with_unexpected_eof`) | planned |

### HOSTILE-INPUT-001 — Fuzz target exercises the trailing-bytes path

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|---|---|---|---|
| `contracts/hazard-analysis.md §2.7` + `contracts/proof-seeds.jsonl:PS-VB-1WORA-008` | POB-vb-1wora-007 | cargo-fuzz (`fuzz_target_trailing_bytes` in `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs`) | planned |

### ROUND-TRIP-001 — encode_record + decode_record round-trip is unchanged

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|---|---|---|---|
| `contracts/proof-seeds.jsonl:PS-VB-1WORA-010` | POB-vb-1wora-003 | cargo-test (existing `roundtrip_*` suite) | planned |
| `contracts/proof-seeds.jsonl:PS-VB-1WORA-010` | POB-vb-1wora-005 | proptest (`proptest_trailing_bytes_roundtrip_unchanged`) | planned |

## 2. Per-Lane Coverage Summary

| Lane | Proof Obligations | Invariants Covered |
|---|---|---|
| rust-local | POB-vb-1wora-001 | INV-CODEC-TB-003 (ordering), INV-CODEC-TB-006 (diagnostic wiring) |
| cargo-test | POB-vb-1wora-002, POB-vb-1wora-003 | INV-CODEC-TB-001, INV-CODEC-TB-002, INV-CODEC-TB-004, INV-CODEC-TB-005, REFINE-MUTEX-001, ROUND-TRIP-001 |
| proptest | POB-vb-1wora-003, POB-vb-1wora-005 | INV-CODEC-TB-002, INV-CODEC-TB-004, INV-CODEC-TB-005, REFINE-MUTEX-001, ROUND-TRIP-001 |
| kani | POB-vb-1wora-004 | INV-CODEC-TB-001, INV-CODEC-TB-003, INV-CODEC-TB-005 |
| verus | POB-vb-1wora-006 | INV-CODEC-TB-007 (with WEAK_MIRROR production binding) |
| cargo-fuzz | POB-vb-1wora-007 | INV-CODEC-TB-001 (hostile perspective), HOSTILE-INPUT-001 |

## 3. Per-Invariant Coverage Summary

| Invariant | rust-local | cargo-test | proptest | kani | verus | cargo-fuzz | Total Lanes |
|---|---|---|---|---|---|---|---|
| INV-CODEC-TB-001 | — | ✅ | — | ✅ | — | ✅ | 3 |
| INV-CODEC-TB-002 | — | ✅ | ✅ | — | — | — | 2 |
| INV-CODEC-TB-003 | ✅ | — | — | ✅ | — | — | 2 |
| INV-CODEC-TB-004 | — | ✅ | ✅ | — | — | — | 2 |
| INV-CODEC-TB-005 | — | ✅ | ✅ | ✅ | — | — | 3 |
| INV-CODEC-TB-006 | ✅ | ✅ | — | — | — | — | 2 |
| INV-CODEC-TB-007 | — | — | — | — | ✅ | — | 1 (verus-only, WEAK_MIRROR bridge) |
| REFINE-MUTEX-001 | — | ✅ | ✅ | — | — | — | 2 |
| HOSTILE-INPUT-001 | — | — | — | — | — | ✅ | 1 (fuzz-only) |
| ROUND-TRIP-001 | — | ✅ | ✅ | — | — | — | 2 |

## 4. Coverage Gaps

**No gaps.** Every invariant in `contracts/contract.md §5.2` and `contracts/proof-seeds.jsonl` (PS-VB-1WORA-001..010) is bound to at least one proof obligation. The Verus-only invariant `INV-CODEC-TB-007` is single-lane by necessity: the production-binding gate (`scripts/check-verus-production-binding.sh`) and the drift gate (`scripts/check-production-inner-drift.sh`) are the only mechanisms that can express "the spec enumerates a new production variant," because Verus is the only verifier that binds the bridge `ensures` to the production source.

## 5. Legend

- ✅ = Invariant is covered by at least one obligation in this lane.
- — = Invariant is not covered by any obligation in this lane (intentional; not a gap).
- `planned` = obligation exists in `proof-obligations.planned.jsonl` with `status: planned`; the verifier has not yet executed.