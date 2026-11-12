# vb-s4hfc: Exact Error Repair Handoff (V8)

## Summary

Repaired all generic `is_err()`/`is_ok()` assertions in `crates/vb_storage/tests/policy_decision_full_record.rs` to use exact `JournalError` variant matching. All 15 tests pass.

## Changed Assertions

### 1. malformed_payload_rejected
- **Before:** `assert!(result.is_err(), "malformed payload should be rejected")`
- **After:** `match result { Err(JournalError::PostcardDecodeFailed(_)) => {} other => { return Err(...) } }`
- **Actual error path:** `postcard::take_from_bytes::<JournalEvent>` fails → legacy decode fails → `JournalError::PostcardDecodeFailed(stable_error)`

### 2. truncated_payload_rejected
- **Before:** `assert!(result.is_err(), "truncated payload should be rejected")`
- **After:** `match result { Err(JournalError::UnexpectedEof) => {} other => { return Err(...) } }`
- **Actual error path:** `header.get(..RECORD_HEADER_BYTES)` fails (record < 60 bytes) → `JournalError::UnexpectedEof`

### 3. envelope_kind_mismatch_rejected
- **Before:** `assert!(result.is_err(), "envelope/payload kind mismatch should be rejected")`
- **After:** `match result { Err(JournalError::RecordKindPayloadMismatch { envelope_kind: 10, payload_kind: 38 }) => {} other => { return Err(...) } }`
- **Actual error path:** Stable decode succeeds with PolicyDecisionEvent (tag 38), kind mismatch detected, legacy decode fails → `JournalError::RecordKindPayloadMismatch { envelope_kind: 10, payload_kind: 38 }`

### 4. run_envelope_mismatch_protected
- **Before:** `assert!(encode_result.is_err(), "run envelope mismatch should be rejected at encode; got Ok")`
- **After:** `match encode_result { Err(JournalError::Encode(postcard::Error::SerdeSerCustom)) => {} other => { return Err(...) } }`
- **Actual error path:** `serialize_policy_decision` checks `decision.ticket.run != *run`, returns `serde::ser::Error::custom(...)` → postcard wraps as `SerdeSerCustom` → `JournalError::Encode(SerdeSerCustom)`
- **Limitation:** postcard v1.1.3 `SerdeSerCustom` variant carries no message content, so exact run IDs cannot be asserted

### 5. policy_decode_rejects_mismatched_run
- **Before:** Positive control asserted 4 fields; negative control used `if result.is_ok() { return Err(...) }`
- **After:** Positive control asserts full `JournalEvent` equality via `assert_eq!(decoded, expected)`; negative control uses `match result { Err(postcard::Error::SerdeDeCustom) => {} other => { return Err(...) } }`
- **Actual error path:** `deserialize_policy_decision` checks `decision.ticket.run != run`, returns `de::Error::custom(...)` → postcard wraps as `SerdeDeCustom`
- **Limitation:** postcard v1.1.3 `SerdeDeCustom` variant carries no message content, so exact run IDs cannot be asserted

## Files Changed

- `crates/vb_storage/tests/policy_decision_full_record.rs` — 5 assertions replaced with exact variant matching; added `use vb_storage::JournalError;` import
- No production code changes
- No original historical test changes

## Test Evidence

- Command: `cargo test -p vb_storage --test policy_decision_full_record`
- Result: All 15 tests PASS
- Evidence prefix: `.beads/vb-s4hfc/evidence/EXACT_ERRORS_V8_02`
- Exit code: 0
- Duration: 0.317 seconds

## Known Limits

1. postcard v1.1.3 custom error variants (`SerdeSerCustom`, `SerdeDeCustom`) discard the original serde error message. Exact run ID assertions (999 vs 100) are therefore replaced with variant-only matching.
2. The malformed payload test accepts any `PostcardDecodeFailed` with any nested error; the specific postcard error variant depends on the garbage bytes.
3. No unsafe, unwrap, expect, panic, unchecked index/slice/arithmetic/cast, lint allowances, or ignored tests introduced.
