# Test Plan Review: `journal_event` Fuzz Target (LETHAL-7)

## VERDICT: REJECTED

**Mode 1 — Plan Inquisition**. No implementation exists yet. Pure adversarial doc analysis.

---

## Axis 1 — Contract Parity

### Functions in plan (behavior inventory):

| # | Function | Behavior Count |
|---|---------|---------------|
| F1 | `parse_event(data: &[u8]) -> Result<JournalEvent, JournalError>` | B1–B6 |
| F2 | `JournalEvent::is_valid() -> bool` | B7–B10 |

**Finding — LETHAL: `JournalError` variant name mismatch with actual enum.**
- B5 + scenario "parse_event rejects future schema version" expect `JournalError::SchemaVersionTooNew`
- Actual `JournalError` enum (error/mod.rs:75–79) has `UnsupportedSchemaVersion`
- **No such variant `SchemaVersionTooNew` exists in the code.** The plan invents a contract clause that does not match any actual error variant.

### Error variant coverage (from plan's BDD scenarios vs. actual JournalError enum):

| Scenario | Expected Error | Actual Variant? |
|----------|---------------|-----------------|
| Wrong magic | `JournalError::BadMagic { expected: 0x5642_4A45, actual: <given> }` | ✅ EXISTS (error/mod.rs:70) |
| Truncated header | `JournalError::UnexpectedEof` | ✅ EXISTS (error/mod.rs:124) |
| Truncated payload | `JournalError::UnexpectedEof` | ✅ EXISTS |
| Corrupt payload | `JournalError::PayloadDigestMismatch` | ✅ EXISTS (error/mod.rs:121) |
| Future schema | `JournalError::SchemaVersionTooNew` | ❌ DOES NOT EXIST |
| Unknown RecordKind | `JournalError::UnknownRecordKind` | ✅ EXISTS (error/mod.rs:90) |

**LETHAL: `SchemaVersionTooNew` is not a `JournalError` variant.**

---

## Axis 2 — Assertion Sharpness

### Scenario review:

**Scenario "parse_event accepts valid SlotWrittenEvent" (line 60–69)**
```
Then: Returns Ok(SlotWrittenEvent)
 And: The returned event.is_valid() is true
```
- `is_valid() == true` — **MAJOR** (no exact field values asserted)
- No assertion on `run`, `seq`, `slot`, `attempt`, `value` fields

**Scenario "parse_event rejects wrong magic" (line 72–77)**
```
Then: Returns Err(JournalError::BadMagic { expected: 0x5642_4A45, actual: <given> })
```
- ✅ Exact error variant — GOOD

**Scenario "parse_event rejects truncated header" (line 79–85)**
```
Then: Returns Err(JournalError::UnexpectedEof { expected: 60, actual: <len> })
```
- ✅ Exact error variant

**Scenario "parse_event rejects corrupt payload" (line 96–102)**
```
Then: Returns Err(JournalError::PayloadDigestMismatch)
```
- ✅ Exact error variant

**Scenario "parse_event rejects future schema version" (line 104–110)**
```
Then: Returns Err(JournalError::SchemaVersionTooNew { .. })
```
- ❌ `SchemaVersionTooNew` does not exist in `JournalError` enum
- The correct variant is `UnsupportedSchemaVersion`

**Scenario "parse_event rejects all-zero bytes" (line 112–119)**
```
Then: Returns Err (not panic)
 And: Error is one of: BadMagic, UnknownRecordKind, UnexpectedEof, PayloadDigestMismatch
```
- **LETHAL**: `is_err()` without exact variant. Vague — any of 4 errors is acceptable.
- `not panic` is not an assertion on the return value

**Scenario "All 18 JournalEvent variants parse correctly" (line 121–132)**
```
Then: Each returns Ok(event) where event.is_valid() == true
```
- `is_valid() == true` — **MAJOR** for each variant

---

## Axis 3 — Trophy Allocation

**Functions to cover: 2** (`parse_event`, `JournalEvent::is_valid()`)

| Layer | Planned | Required | Status |
|-------|---------|----------|--------|
| Unit tests | **0** | ≥ 10 (5× rule) | **LETHAL** |
| Integration | 2 (existing) | — | ✅ |
| Fuzz | 1 | 1 | ✅ |
| Kani | 0 (existing) | — | ✅ |

**LETHAL — Zero unit tests for 2 public functions.**
- Rule: `planned unit test count < 5× public function count` → **LETHAL**
- 2 pub fns × 5 = 10 unit tests minimum
- Plan explicitly allocates **0 unit tests** ("No unit tests needed — all behavior is exercised via the fuzz target")
- Fuzz and integration tests do not substitute for unit test density
- The plan's own rationale: "No unit tests needed" is self-serving and violates the trophy allocation rule

**LETHAL — Parser/deserializer with no dedicated unit tests.**
- `parse_event` is a critical parser entry point with complex error paths
- Relying solely on 1 fuzz target + 2 existing integration tests provides insufficient mutation survivability at the unit level

---

## Axis 4 — Boundary Completeness

### `parse_event` boundaries:

| Boundary | Explicitly Named? |
|----------|-------------------|
| Minimum valid input (60B header + 0 payload) | ❌ MINOR |
| Maximum valid input (60B header + 1_048_576 payload) | ❌ MINOR |
| One-below-minimum (59 bytes) | ❌ MINOR |
| One-above-maximum (60B header + 1_048_577 payload) | ❌ MINOR |
| Empty / zero bytes | ✅ (all-zero bytes scenario) |
| Overflow potential (payload_len declared > actual) | ❌ MINOR (covered by truncated payload scenario but not named as "overflow") |

**MINOR × 5 = 1 MAJOR** — missing explicit boundary names

### `JournalEvent::is_valid()` boundaries:

| Boundary | Explicitly Named? |
|----------|-------------------|
| run_id == ZERO → false | ✅ B8 |
| seq == EventSeq::MAX → false (SlotWrittenEvent) | ✅ B9 |
| attempt == 0 where required → false | ✅ B10 |
| run_id != ZERO, valid seq, valid attempt → true | ❌ MINOR |

**Summary: 5 missing explicit boundary specifications → MAJOR**

---

## Axis 5 — Mutation Survivability

**Mental mutation apply:**

| Mutation | Caught by which test in plan? |
|----------|------------------------------|
| Remove header magic validation → wrong magic returns Ok instead of BadMagic | ❌ No unit test catches this. Fuzz corpus has wrong-magic seeds but no assertion on exact variant in the fuzz body. |
| Remove payload digest verification → corrupt payload returns Ok | ❌ No unit test. Fuzz catches via corpus but body only asserts `is_valid()`. |
| Remove schema version check → future schema returns Ok | ❌ No unit test. |
| Return `Ok(Default::default())` instead of real event | ❌ Fuzz asserts `is_valid()` but not field values. Scenario "All 18 variants" checks `is_valid() == true` but not exact field contents. |
| Swap `record_kind` with `schema_version` bytes | ❌ No test catches structural swap at the unit level. |

**MAJOR: Each of these 5 mutations has no specific unit test assertion that would catch it.**
The integration proptests round-trip through encode/decode which would catch swap mutations, but the plan relies on fuzz corpus seeds rather than explicit unit test assertions.

---

## Axis 6 — Evidence Plan Audit

### GIVEN blocks review:

| Scenario | GIVEN state explicit? |
|----------|----------------------|
| SlotWrittenEvent valid | ✅ Valid record encoded with MAGIC_JOURNAL_EVENT, RecordKind=SlotWritten |
| Wrong magic | ✅ Explicitly names 4 wrong magic values (MAGIC_BLOB, MAGIC_SNAPSHOT, 0xFFFF_FFFF, 0x0000_0000) |
| Truncated header | ✅ Bytes shorter than 60 |
| Truncated payload | ✅ Valid header + fewer than N payload bytes |
| Corrupt payload | ✅ Valid header + mutated payload |
| Future schema | ✅ schema_version = CURRENT_SCHEMA_VERSION + 1 |
| All-zero bytes | ✅ All-zero bytes of length >= 60 |
| All 18 variants | ✅ Lists all 18 variants explicitly |

### Bounded/reproducible inputs:
- ✅ Corpus seeds are named with explicit content descriptions
- ✅ Fuzz input is `&[u8]` (unbounded byte slice — acceptable for fuzz)

### Side effects in setup:
- N/A — no implementation yet, this is a plan review

---

## LETHAL FINDINGS

1. **`SchemaVersionTooNew` is not a `JournalError` variant** (test-plan-journal-event-fuzz.md:76, 109)
   - `JournalError::UnsupportedSchemaVersion { version: u16 }` exists at error/mod.rs:75
   - The plan's BDD scenarios and behavior inventory B5 reference a non-existent error variant
   - **Must be corrected to `UnsupportedSchemaVersion` before implementation**

2. **Zero unit tests violates 5× trophy allocation rule** (test-plan-journal-event-fuzz.md:9, 54)
   - 2 pub fns × 5 = 10 unit tests minimum required
   - Plan allocates 0 unit tests
   - Fuzz target + 2 existing integration tests do not substitute for unit test density

3. **Scenario "parse_event rejects all-zero bytes" uses `is_err()` as sole assertion** (test-plan-journal-event-fuzz.md:117)
   - Returns "one of: BadMagic, UnknownRecordKind, UnexpectedEof, PayloadDigestMismatch"
   - `is_err()` without exact variant → **LETHAL per Axis 2**

---

## MAJOR FINDINGS (5)

1. **Scenario "parse_event accepts valid SlotWrittenEvent" — `is_valid() == true` without field values** (line 68)
   - No assertion on `run`, `seq`, `slot`, `attempt`, `value` fields
   - A swapped-field or default-valued event would pass

2. **Scenario "All 18 JournalEvent variants parse correctly" — `is_valid() == true` without field values** (line 131)
   - Same weakness as above, replicated across all 18 variants
   - Each variant's specific fields not asserted

3. **Missing unit-level boundary assertions for `parse_event` overflow** (test-plan-journal-event-fuzz.md:54)
   - No unit test asserts behavior at `MAX_JOURNAL_EVENT_PAYLOAD_BYTES` (1_048_576)
   - No unit test asserts `MAX_JOURNAL_EVENT_PAYLOAD_BYTES + 1`

4. **Missing unit-level mutation coverage for schema version check removal**
   - No unit test explicitly verifies `UnsupportedSchemaVersion` is returned for schema_version > 1
   - The "future schema v2" scenario is BDD only — no unit test enforces it

5. **Mutation survivability gap: `decode_record` return value mutation not caught at unit level**
   - If `parse_event` wrapped `decode_record` but accidentally returned `Ok(Default::default())`, no unit test would catch the specific field-level values being wrong

---

## MINOR FINDINGS (4/5 threshold)

1. **Boundary Completeness: `parse_event` minimum valid input not explicitly named as boundary case**
2. **Boundary Completeness: `parse_event` maximum valid input not explicitly named as boundary case**
3. **Boundary Completeness: `parse_event` one-above-maximum not explicitly named as boundary case**
4. **Boundary Completeness: `is_valid()` — no positive validity case with all valid fields named explicitly**

---

## MANDATE — What must exist before resubmission

1. **Fix `SchemaVersionTooNew` → `UnsupportedSchemaVersion`** in all BDD scenarios and behavior inventory (B5, scenario "parse_event rejects future schema version")

2. **Add minimum 10 unit tests** covering:
   - Each error variant assertion (exact `Err(JE::Variant)`)
   - `is_valid()` for each of the 18 variants with exact field values
   - Boundary: `parse_event` at 60B (min header), 59B (one-below-min), 0B (empty)
   - Boundary: `parse_event` with `MAX_JOURNAL_EVENT_PAYLOAD_BYTES` and `+1` overflow
   - Positive validity: all structural fields of a `SlotWrittenEvent` assert with exact values

3. **Fix scenario "parse_event rejects all-zero bytes"** — replace "one of" with a specific error variant that is deterministic for all-zero input, or add additional preconditions to narrow which error is expected

4. **Add explicit `is_valid()` positive case unit test** asserting `true` for a fully-valid event with all fields correctly populated

---

## Summary

| Severity | Count | Blocking? |
|----------|-------|-----------|
| LETHAL | 3 | YES |
| MAJOR | 5 | YES |
| MINOR | 4 | NO |

**REJECTED.** The plan allocates zero unit tests for 2 pub fns, references a non-existent `JournalError` variant, and uses `is_err()` as a vague sole assertion. The plan cannot be approved in this state.

Resubmit after correcting all 3 LETHAL findings and the top MAJOR findings.
