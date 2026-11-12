# Legacy Layout Compatibility Tests (vb-h7rex)

## Purpose

The existing `policy_pin_wire_contract.rs` tests verify tag values and FULL-Eq
roundtrips, but both encoder and decoder could drift together (e.g., swap field
order symmetrically) and still pass. These four new tests pin the exact historical
postcard byte layout for tag 24 and tag 36 admission events.

## Test File

`crates/vb_storage/tests/policy_pin_legacy_layout.rs` — four focused tests.

## Coverage

### Tag 24 (RunAdmission — legacy unbound)

**Layout:** 5-tuple `(run, seq, artifact_digest, caps, policy)`

- `tag_24_unbound_encoder_preserves_historical_layout`: Encode the historical
  5-tuple directly via `encode_record`; assert byte-identical output to
  encoding the current typed `JournalEvent::RunAdmission` (Unverified pin).
  Catches encoder field-order drift.

- `tag_24_unbound_decoder_accepts_historical_layout`: Wrap the historical
  5-tuple in a tag 24 envelope; assert the current decoder produces the
  expected `JournalEvent` with `RunPolicyPin::Unverified`. Catches decoder
  field-order drift.

### Tag 36 (RunAdmissionBound — digest-bound)

**Layout:** 6-tuple `(run, seq, artifact_digest, input_digest, caps, policy)`

Same two-way verification pattern: encoder byte identity and decoder
acceptance of the historical layout, with `input_digest` at its exact
historical position before `caps`.

## Key Properties

1. **Independent tuple construction:** Historical payloads are built as
   explicit tuples — never through the current `JournalEvent` serializer.
   This ensures the tests don't trivially pass if both sides drift
   together.

2. **Non-empty capabilities:** Uses a concrete capability (`network`,
   action 42) rather than an empty set, exercising the full tuple layout.

3. **Distinct digests:** Different byte patterns for run, policy, input,
   and artifact digests to expose positional swaps.

4. **Full byte equality:** Asserts exact byte identity between historical
   and current encoding, not just decode roundtrip.

5. **Concrete expected events:** Expected `JournalEvent` fixtures are
   constructed from spec constants, not extracted from encoded bytes.

## Verification Strategy

The tests guarantee that any future field-reordering, type change, or
tag layout modification in the admission serializers will fail at least
one of the four checks — even if the change is symmetric between encoder
and decoder.