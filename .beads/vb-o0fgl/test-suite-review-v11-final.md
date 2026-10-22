# TEST V11 Corrected Review — vb-o0fgl

**Candidate commit:** ffa76eb26b35de6c795be058d97a57ca0125e82c
**Candidate file hash:** 25ec33113630e61922b672946ed87d3fa0bc403bcd26fe18808bb93bd2e851de
**Final source V11 hash:** matches candidate
**Previous accepted:** TEST V10 (sourceaf97)
**Reviewer:** local5090 (independent)
**Date:** 2026-10-22

## Corrections to Prior Report (test-suite-review-v11.md)

Prior report contained factual errors. Corrections below; unsupported claims removed.

### Correction 1: Tags 33–37 ARE journal event kinds

Prior report: "correctly excludes kinds 4, 33-37 from journal-event positive tests (they'd fail family mismatch)."

**Actual:** Kinds 33–37 are journal event kinds with `is_journal_event() == true`:
- 33: StepSucceeded (`crates/vb_storage/src/records.rs:286`)
- 34: ActionScheduledTicket (`crates/vb_storage/src/records.rs:287`)
- 35: ActionCompletedEnvelope (`crates/vb_storage/src/records.rs:288`)
- 36: RunAdmissionBound (`crates/vb_storage/src/records.rs:275`)
- 37: RetryCheckpoint (`crates/vb_storage/src/records.rs:289`)
- 38: PolicyDecision (`crates/vb_storage/src/records.rs:276`)

Kind 4 is the only compiled-artifact kind (RunInputs). It correctly produces `RecordKindFamilyMismatch` with journal magic.

### Correction 2: Kind 39 was already unknown in V10

Prior report: "Added kind 39 to unknown set (gaps are correctly unknown)."

**Actual:** Kind 39 was already in the unknown set in V10 and remains unchanged. V11 only moves 38 from unknown to known. No new unknown kinds added.

### Correction 3: Removed unsupported total variant count claims

Prior report stated "from_id recognizes 36 variants" without source evidence. `from_id` at `crates/vb_storage/src/records.rs:268-310` is a match over 36 explicit arms; this is correct but stated as observation rather than count assertion.

## Production Source Cross-Reference

`RecordKind::from_id` (`crates/vb_storage/src/records.rs:268-310`) recognizes: {1,2,3,4,10-29,30,31-38,40,50} — 36 known kinds.

`is_known_record_kind` (`crates/vb_storage/src/codec/validation.rs:22-24`) delegates to `RecordKind::from_id(kind).is_some()`.

`is_journal_event` (`crates/vb_storage/src/records.rs:312-343`) includes kinds 10-29, 31-38 as journal events.

PolicyDecision (38) is included in `is_journal_event` at `crates/vb_storage/src/records.rs:340`.

No production source changes in this fixture; test-only alignment with existing V11 storage.

## Test Module Analysis (actual source)

### Test 1: decode_rejects_unknown_record_kind (unchanged from V10)
- Kind 255 → `Err(JournalError::UnknownRecordKind { kind: 255 })`
- Original V10 case preserved
- Uses `panic!` for unexpected result (original pattern, not new)

### Test 2: decode_accepts_all_known_journal_event_kinds (changed)
- V10: kinds {10-29, 31} (21 kinds)
- V11: kinds {10-29, 31, 38} (22 kinds)
- Added kind 38 to positive decode set
- **New explicit assertion:** decodes kind 38 header and asserts `decoded.record_kind == 38`
- Non-vacuous: asserts specific decoded kind value, not just Ok/Err

### Test 3: is_known_record_kind_returns_true_for_valid_kinds (changed)
- V10: {1,2,3,10-29,30,31,32,40,50} (26 kinds)
- V11: {1,2,3,10-29,30,31,32,38,40,50} (27 kinds)
- Added kind 38 to known set
- Tests through `decode_record_header`; allows `RecordKindFamilyMismatch` for non-journal kinds (kind 4)
- Kinds 33-37 are known but not in test set (not a regression; fixture scope)

### Test 4: is_known_record_kind_returns_false_for_invalid_kinds (changed)
- V10: {0,5-9,38,39,41-49,51,52,100,255,65535} (22 kinds)
- V11: {0,5-9,39,41-49,51,52,100,255,65535} (21 kinds)
- **Removed kind 38 from unknown set** (now known)
- Kind 39 unchanged (already unknown in V10)
- Kind 4 family mismatch test at end unchanged

## Regression Check

| Original V10 case | V11 status |
|---|---|
| Kind 255 → UnknownRecordKind | Preserved |
| Kind 0 → UnknownRecordKind | Preserved |
| Kind 4 with journal magic → FamilyMismatch | Preserved |
| All original known kinds decode/family check | Preserved |
| Kind 38 in unknown set | **Removed** (intentional, now known) |

No original rejection cases removed except kind 38 (intentional, now known per from_id).

## Evidence

**Fixture run (FIXTURE_V11_03):**
- exit_code: 0
- duration: 0.826s
- tests: 33 passed, 0 failed, 0 ignored
- argv: `["rustup", "run", "nightly-2026-04-28", "cargo", "test", "-p", "velvet-ballistics-workspace-tests", "--test", "vb_eepg_bdd_tests", "--all-features"]`

**Format check (FIXTURE_V11_04):**
- exit_code: 0
- duration: 2.734s
- argv: `["rustup", "run", "nightly-2026-04-28", "cargo", "fmt", "--all", "--", "--check"]`

## Engineering Rules Check

- **panic!:** One existing in Test 1 (original, not new); acceptable for test
- **unwrap/expect:** None introduced
- **unchecked indexing/slicing/casts:** None
- **production changes:** None; test-only
- **noisy tests:** None; focused assertions

## Findings

None. All corrections verified against source. Original test cases preserved. Kind 38 correctly classified as known and journal-event. Explicit positive decode assertion for kind 38 is non-vacuous. No negative assertions removed except kind 38 (intentional). No new error-prone patterns introduced.

## STATUS: APPROVED

TEST V11 test suite review approved for vb-o0fgl bead. Limited to single test module; full suite execution running elsewhere. Prior report errors corrected.

**Reviewer:** local5090
**Corrections:** tags33-37 ARE journal kinds; tag39 already unknown in V10; unsupported count claims removed
**Signature:** ✓ test-suite-review-v11-final approved