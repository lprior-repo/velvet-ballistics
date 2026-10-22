# vb-h7rex Tests V11 Handoff

## Test Cases Added

### Gap 1: Runtime recovery policy pin mismatch (vb_runtime)

**File:** `crates/vb_runtime/tests/policy_pin_durable_recovery.rs`

**New test:** `admission_policy_pin_mismatch_not_reachable_with_current_semantics`

Documents a production limitation: the `admission_policy_pin_mismatch` error path in `validate_recovered_policy_pin` is not reachable with current journal semantics. The checkpoint validation (`validate_retry_checkpoint_against_durable_chain`) checks value equality between supplied and durable checkpoints, but the admission validation hydrates from the journal's `RunAdmission` event (which always has the correct pin). Since the `RunAdmission` event is written before the checkpoint and cannot be modified via the PUBLIC `append_strict` API, no test scenario can trigger this specific error path.

The test forges a checkpoint with a wrong policy pin and persists it as the durable checkpoint. The checkpoint validation passes (supplied matches durable), recovery proceeds, and the hydrated admission from the journal has the correct pin — so recovery succeeds instead of failing with `admission_policy_pin_mismatch`.

**Existing tests still pass:** `strict_admission_policy_pin_survives_durable_recovery`, `journaled_admission_policy_pin_survives_durable_recovery`, `legacy_version_1_checkpoint_is_explicitly_rejected`, `tampered_policy_pin_rejected_by_recover_retry`

### Gap 2: Storage wire contract (vb_storage)

**File:** `crates/vb_storage/tests/policy_pin_wire_contract.rs` (new)

**Tests:**
- `legacy_unbound_unverified_roundtrips_as_tag_24`
- `digest_bound_unverified_roundtrips_as_tag_36`
- `unbound_pinned_roundtrips_as_tag_39`
- `bound_pinned_roundtrips_as_tag_41`
- `all_admission_record_kinds_have_stable_ids`
- `wire_envelope_kind_matches_event_record_kind`
- `malformed_record_kind_bytes_rejected`
- `truncated_payload_rejected`

All four admission variants (tags 24, 36, 39, 41) roundtrip with FULL-Eq. Distinct run/seq/artifact/input/policy identities are used. Malformed/truncated payload exact errors are tested.

## Contract Correction

**File:** `.beads/vb-h7rex/contract.md`

Corrected to state that both tags 39 and 41 are used:
- Tag 39: `RunAdmissionPinned` (LegacyUnbound + Pinned policy)
- Tag 41: `RunAdmissionBoundPinned` (Digest-bound + Pinned policy)

Also added: "RunCheckpointV1 version 2 payloads reject version 1 before decode"

## Commands Run

1. `cargo test --test policy_pin_wire_contract` → PASS (10 tests)
2. `cargo test --test policy_pin_durable_recovery` → PASS (5 tests)
3. `cargo +nightly-2026-04-28 fmt --all --check` → PASS

## Evidence

- `.beads/vb-h7rex/evidence/TEST_V11_01.*` — storage wire contract tests
- `.beads/vb-h7rex/evidence/TEST_V11_02.*` — runtime policy pin durable recovery tests
- `.beads/vb-h7rex/evidence/TEST_V11_03.*` — cargo fmt check

## Limits

The `admission_policy_pin_mismatch` error path is documented as unreachable with current journal semantics. This is a potential production bug that should be investigated separately: `validate_retry_checkpoint_against_durable_chain` does not validate the checkpoint's admission policy pin against the durable `RunAdmission` event, and the recovery flow hydrates the admission from the journal events rather than the checkpoint.

## No Approval

Tests authored and verified. Not approving or closing the bead; awaiting root independent review.