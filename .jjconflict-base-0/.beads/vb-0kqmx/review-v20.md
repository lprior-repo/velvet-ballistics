# Independent Adversarial Test Review: vb-h7rex (v20)

**Reviewer:** LOCAL3090 (independent agent)
**Workspace:** /home/lewis/src/isoloated/vb-policy-tests-v20
**Candidate commit:** 7c3de06eb7a06bc2341c4f39d4e37f718ef316d7
**Baseline:** main 58a1f9580ca8beb32f80118ce08df521688a249a
**Scope:** Static review of storage/admission policy pin tests only. No Runtime dispatch acceptance.

## Status: APPROVED

## Delta Summary

V20 introduces exactly two test-file changes over V19:

1. **New:** `crates/vb_storage/tests/policy_pin_legacy_layout.rs` — legacy layout compatibility tests
2. **Modified:** `crates/workspace_tests/tests/restate_doctor_key_decode_tests.rs` — readonly namespace count 10→11

All 73 V19 source inputs unchanged. Production code unchanged.

## New Legacy Layout Tests

**File:** `crates/vb_storage/tests/policy_pin_legacy_layout.rs`

### Test Architecture

Both tests follow identical structure:
1. Construct explicit flat tuple matching historical postcard layout
2. Encode historical tuple via `encode_record`
3. Encode current typed `JournalEvent` via same path
4. Assert byte-identical output (catches encoder drift)
5. Decode historical bytes via `decode_journal_event`
6. Assert envelope record_kind matches expected tag
7. Assert decoded event FULL-Eq to expected typed event (catches decoder drift)

### Tag 24 (RunAdmission — legacy unbound)

`tag_24_unbound_preserves_historical_layout`:
- Historical payload: `(24_u8, &run, &seq, &artifact_digest, &caps, &policy)` — 6 elements
- Expected layout: `(tag, run, seq, artifact_digest, caps, policy)`
- Policy: Relaxed, Pin: Unverified, Binding: LegacyUnbound
- Uses non-empty capability (`network`, action 42)
- Distinct run ID (777_001), artifact digest ([0x55; 32])

**Assessment:** Correctly encodes the historical unbound admission layout. Byte equality proves encoder preserves field order. Decode verifies decoder accepts historical bytes.

### Tag 36 (RunAdmissionBound — digest-bound)

`tag_36_bound_preserves_historical_layout`:
- Historical payload: `(36_u8, &run, &seq, &artifact_digest, &input_digest, &caps, &policy)` — 7 elements
- Expected layout: `(tag, run, seq, artifact_digest, input_digest, caps, policy)`
- **Input digest positioned before caps** — matches production serializer
- Policy: Strict, Pin: Unverified, Binding: Digest([0x54; 32])

**Assessment:** Correctly encodes the historical bound admission layout. Critical detail: `input_digest` at index 4 (after artifact, before caps) matches production `serialize_bound_admission_payload` field order in `crates/vb_storage/src/events/wire_serialize_admission.rs:127`.

### Production Serializer Verification

Verified field order against actual production serializer:

**Tag 24 path** (`serialize_admission_payload` line 52-53):
```rust
(RunInputBinding::LegacyUnbound, RunPolicyPin::Unverified) =>
    serialize_record_kind_variant(serializer, TAG_RUN_ADMISSION, "RunAdmission", &fields)
```
Where `AdmissionFields` = `(run, seq, artifact, caps, policy)` — matches test.

**Tag 36 path** (`serialize_bound_admission_payload` line 127-139):
```rust
let (run, seq, artifact, caps, policy) = fields;
let payload = (run, seq, artifact, input_digest, caps, policy);
serialize_record_kind_variant(serializer, TAG_RUN_ADMISSION_BOUND, "RunAdmission", &payload)
```
Input digest inserted at index 3, before caps — matches test.

### Helper Function Quality

`check_historic_layout` — generic helper, 15 lines, 3 args, returns `Result<(), JournalError>`. No expect/unwrap/panic/unchecked indexing. Proper generic `<T: serde::Serialize>` bound.

### No Vec-Wrapped Payload Bug

Both tests serialize the raw tuple directly. No intermediate `Vec<u8>` payload construction that could double-encode.

## Readonly Namespace Change

**File:** `crates/workspace_tests/tests/restate_doctor_key_decode_tests.rs`

- Test renamed: `readonly_journal_declared_keyspaces_returns_ten` → `readonly_journal_declared_keyspaces_returns_eleven`
- Expected count: 10 → 11
- Comment added: "Actual count is 11 after emergency_deny keyspace was added (vb-h7rex)."

**Production binding verified:**
- `crates/vb_storage/src/journal/core.rs:162`: `pub const fn declared_keyspaces() -> [&'static str; 11]`
- `crates/vb_storage/src/journal/readonly.rs:65`: `ReadOnlyJournal::declared_keyspaces()` delegates to `FjallJournal::declared_keyspaces()`
- `crates/vb_storage/src/constants.rs:30`: `KEYSPACE_EMERGENCY_DENY` constant exists

The 11th keyspace is `emergency_deny`, added as part of the durable deny overlay in vb-h7rex.

## Evidence Inspection

### ROOT_V20_LEGACY (`.beads/vb-h7rex/evidence/ROOT_V20_LEGACY.json`)
- Exit code: 0
- Duration: 7.584s
- Command: `cargo test -p vb_storage --all-features --test policy_pin_legacy_layout`
- **Result: 2 passed, 0 failed** (verified via stdout)

### ROOT_V20_READONLY (`.beads/vb-h7rex/evidence/ROOT_V20_READONLY.json`)
- Exit code: 0
- Duration: 7.003s
- Command: `cargo test -p velvet-ballistics-workspace-tests --all-features --test restate_doctor_key_decode_tests`
- **Result: 48 passed, 0 failed** (verified via stdout — includes `readonly_journal_declared_keyspaces_returns_eleven`)

### ROOT_V20_FMT (`.beads/vb-h7rex/evidence/ROOT_V20_FMT.json`)
- Exit code: 0
- Duration: 1.483s
- Command: `cargo fmt --all --check`
- **Result: clean**

## Existing Tests Preserved

Verified via grep that all 6 existing wire contract tests in `policy_pin_wire_contract.rs` remain:
- `legacy_unbound_unverified_roundtrips_as_tag_24`
- `digest_bound_unverified_roundtrips_as_tag_36`
- `unbound_pinned_roundtrips_as_tag_39`
- `bound_pinned_roundtrips_as_tag_41`
- `all_admission_record_kinds_have_stable_ids`
- `wire_envelope_kind_matches_event_record_kind`

Not replaced, only supplemented by new legacy layout tests.

## Shared Fixture Status

`crates/vb_storage/src/tests.rs:3017-3018` shared fixture unchanged:
- `policy_pin: RunPolicyPin::Unverified` (legacy)
- `input_digest: RunInputBinding::LegacyUnbound`
- No new pinned variants added (consistent with V19 finding)

Note: Dedicated `policy_pin_wire_contract.rs` covers new pinned variants. Shared fixture missing them is not contract debt — it's a variant-coverage test, not a shared-fixture regression test.

## Historical Layout Gap — Closed

The V19 review noted that existing tests only proved tag values and roundtrips, not that encoder and decoder could not drift together. This gap is now closed:

- **Encoder drift detection:** Byte equality between historical tuple encoding and current event encoding
- **Decoder drift detection:** Successful decode of historical bytes to expected typed event
- **Both drift simultaneously:** Impossible to pass both checks if field order changed in either direction

## Scope and Limits

- **In scope:** Storage/admission policy pin wire contract, durable recovery, legacy layout compatibility
- **Out of scope:** Runtime dispatch/fullv1 acceptance tests (as per brief)
- **Static review only:** Did not run tests; inspected provided raw evidence JSON and source binding
- **V19 status:** Canonical test run failed only on readonly expected 10 vs actual 11 (12205 passed, 2248 unrun, 27 existing skips); V19 was not green

## Conclusion

The two V20 test changes are correct, complete, and properly bound to production code:

1. **Legacy layout tests** independently construct historical flat tuples, verify byte identity against current encoding, and decode historical bytes to expected typed events. Field order verified against actual production serializer. No Vec-wrapped payload bug. Generic helper is clean.

2. **Readonly namespace test** correctly expects 11 keyspace names, matching the production `declared_keyspaces()` return type and the addition of `emergency_deny` in vb-h7rex.

The V19 historical-layout gap is closed. The V19 readonly namespace failure is corrected. All 6 existing wire contract tests remain.

**Status: APPROVED**

No blocking defects found.
