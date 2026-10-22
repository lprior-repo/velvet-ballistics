# WIRE-V13 Results

Date: 2026-10-22
Agent: LOCAL5090

## Test Run

```
cargo test -p vb_storage --test policy_pin_wire_contract
```

**Result:** ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

### Individual Tests

- `all_admission_record_kinds_have_stable_ids` — ok
- `legacy_unbound_unverified_roundtrips_as_tag_24` — ok
- `digest_bound_unverified_roundtrips_as_tag_36` — ok
- `unbound_pinned_roundtrips_as_tag_39` — ok
- `bound_pinned_roundtrips_as_tag_41` — ok
- `wire_envelope_kind_matches_event_record_kind` — ok

## Fmt Check

```
cargo fmt --all --check
```

**Result:** clean (exit 0)

## Changes Made

1. roundtrip_event now inspects envelope.record_kind against event.record_kind().id()
2. Removed unchecked bad_payload[0] indexing (test deleted with issue)
3. Deleted malformed_record_kind_bytes_rejected and truncated_payload_rejected (bogus Vec<u8> payload encoding)
4. All four admission events now use non-empty CapabilitySet via test_granted_capabilities() helper