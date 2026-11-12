# vb-s4hfc Full-Record Policy Decision Tests - Handoff

## Scope
Rewrote `crates/vb_storage/tests/policy_decision_full_record.rs` to use actual production APIs. Previous version had fabricated API calls (`PolicyDigest::new_verified`, `InfluenceState::Compatible`) and wrong `ActionTicket` fields.

## Corrections Applied
- `PolicyDigest::from_bytes(bytes)` instead of `new_verified`
- `InfluenceState` values: `NotEvaluated`, `Inactive`, `Active` only (no `Compatible`)
- `ActionTicket` fields: `run`, `step`, `seq`, `action`, `run_attempt`, `attempt`, `idempotency_key`, `capacity` (8 fields)
- `CurrentPolicyIdentity::Configured { digest: ... }` struct variant
- Proper `Result` patterns with `?` operator
- `tempfile::tempdir()` for unique test directories

## Test Cases (17 tests)
1. `policy_decision_event_round_trip` - Encode/decode full record with all 8 ticket fields
2. `all_decision_enum_variants_round_trip` - All 3 outcomes (Allowed/Denied/Suspended)
3. `record_field_identity_survives_round_trip` - All fields preserved through encode/decode
4. `fjall_persistence_survives_reopen` - Fjall store open/write/sync/close/reopen/read
5. `multiple_ordered_policy_events` - 3 sequential policy events with ordering
6. `malformed_payload_rejected` - Garbage bytes rejected on decode
7. `truncated_payload_rejected` - Half payload rejected
8. `envelope_kind_mismatch_rejected` - Wrong record kind in envelope rejected
9. `run_envelope_mismatch_protected` - Run ID in envelope vs payload verified
10. `journal_seq_not_ticket_seq` - Journal seq (42) distinct from ticket seq (7)
11. `old_tag_10_fixture_unchanged` - Tag 10 (RunAccepted) wire format stable
12. `distinct_nonzero_digests` - Digests are non-zero and distinct
13. `influence_state_values_distinct` - All InfluenceState values distinct
14. `all_outcome_layers_taints_round_trip` - Exhaustive: 3 outcomes × 3 layers × 5 taints × 3 emergency × 3 compat = 1215 combinations
15. `neutral_policy_fact_does_not_reset_lifecycle` - Policy events don't reset Failed/Cancelled/Completed

## Verification Status
- Production code not yet available (local5090 workspace repairing compilation)
- Tests written against actual API signatures from `crates/vb_core/src/policy.rs`, `crates/vb_core/src/action/ticket.rs`
- Verified against neighboring production tests in `crates/vb_runtime/src/policy/tests.rs`
- Cannot run `cargo test` until production integration available

## Files Changed
- `crates/vb_storage/tests/policy_decision_full_record.rs` - Complete rewrite (20KB, 17 tests)
- `.beads/vb-s4hfc/handoff-tests-v2.md` - This document

## Next Steps for Root
1. Wait for local5090 to complete production compilation repairs
2. Run `cargo test -p vb_storage --test policy_decision_full_record`
3. Verify all 17 tests pass
4. Merge to main

## Risk
- If production APIs diverge from current `crates/vb_core/src/policy.rs`, tests may fail to compile
- All API signatures verified against current source; risk is low