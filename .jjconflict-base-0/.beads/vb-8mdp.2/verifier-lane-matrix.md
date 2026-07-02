# Verifier Lane Matrix: vb-8mdp.2 Budget-Before-Decode

## Proof Seed → Verifier Lane Mapping

| Proof Seed | Title | Verifier Lane | Status | Artifact |
|------------|-------|---------------|--------|----------|
| vb-8mdp-2-ps-001 | PayloadTooLarge before any allocation | **Kani** | planned | kani_budget_before_decode.rs |
| vb-8mdp-2-ps-002 | Never panics on any input length | **Kani** | planned | kani_budget_before_decode.rs |
| vb-8mdp-2-ps-003 | decode_record_payload never slices beyond budget | **Kani** | planned | kani_budget_payload.rs |
| vb-8mdp-2-ps-004 | checked_add prevents overflow in payload_end | **Kani** | planned | kani_budget_payload.rs |
| vb-8mdp-2-ps-005 | Magic family constraint enforced first | **Kani** | planned | kani_budget_magic.rs |
| vb-8mdp-2-ps-006 | Unknown record_kind rejected | **Kani** | planned | kani_budget_magic.rs |
| vb-8mdp-2-ps-007 | Budget check at line 48 returns PayloadTooLarge | **Kani** | planned | kani_budget_before_decode.rs |
| vb-8mdp-2-ps-008 | Header length must equal 60 | **Kani** | planned | kani_budget_before_decode.rs |
| vb-8mdp-2-ps-009 | Wrong schema version rejected | **Kani** | planned | kani_budget_magic.rs |
| vb-8mdp-2-ps-010 | CRC mismatch rejected | **Kani** | planned | kani_budget_magic.rs |
| vb-8mdp-2-ps-011 | Corrupt payload digest rejected | **Kani** | not_applicable | kani_digest_checks_vb_2bzz.rs (vb-3t44) |
| vb-8mdp-2-ps-012 | decode_optional no allocation before decode_record_header | **Kani** | planned | kani_recovery_hydrate.rs |
| vb-8mdp-2-ps-013 | decode_journal_event semantic validity | **Kani + proptest** | planned | kani_codec.rs + security_tests.rs |
| vb-8mdp-2-ps-014 | snapshot respects MAX_SNAPSHOT_BYTES budget | **Kani + proptest** | planned | kani_recovery_hydrate.rs + snapshot_tests.rs |
| vb-8mdp-2-ps-015 | blob respects MAX_BLOB_BYTES budget | **Kani + proptest** | planned | kani_recovery_hydrate.rs + blob_tests.rs |
| vb-8mdp-2-ps-016 | decode_record_header is total (no panic) | **Verus** | planned | header.rs Verus specs |
| vb-8mdp-2-ps-017 | payload_len type invariant after budget check | **Verus** | planned | header.rs Verus specs |
| vb-8mdp-2-ps-018 | Keyspace prefix distinctness | **TLA+** | planned | specs/constants.tla |
| vb-8mdp-2-ps-019 | Budget-before-decode workflow invariant | **TLA+** | planned | specs/budget_before_decode.tla |
| vb-8mdp-2-ps-020 | Fuzz target never panics | **Kani + fuzz** | planned | fuzz/decode_record.rs + kani harness |

## Non-Applicable Lanes

| Lane | Proof Seed | Reason |
|------|------------|--------|
| Flux RS | ps-017 | Flux requires nightly-only setup; Verus provides equivalent payload_len invariant proof |
| Miri | ps-001 | Miri detects UB but cannot prove absence of pre-budget allocation (logic error); Kani proves this |
| Loom | ps-012 | decode_optional is single-threaded; no concurrent access patterns exist |

## Lane Summary

| Lane | Count | Status |
|------|-------|--------|
| Kani | 15 | planned |
| Verus | 2 | planned |
| TLA+ | 2 | planned |
| proptest | 3 | planned |
| fuzz | 1 (shared) | planned |
| Flux RS | 1 | not_applicable |
| Miri | 1 | not_applicable |
| Loom | 1 | not_applicable |

## Artifact Naming Convention

| File | Purpose |
|------|---------|
| `kani_budget_before_decode.rs` | Budget gate proofs: PayloadTooLarge before allocation, line 48 |
| `kani_budget_payload.rs` | Payload slice bounds, overflow check |
| `kani_budget_magic.rs` | Magic, schema, kind, CRC ordering proofs |
| `kani_recovery_hydrate.rs` | decode_optional, snapshot, blob budget proofs |
| `specs/constants.tla` | Keyspace prefix distinctness |
| `specs/budget_before_decode.tla` | Budget-before-decode workflow invariant |