# vb-ct31x V2 Repair Handoff

## Changes Made

### Source Files Modified

1. **`crates/vb_storage/src/journal/core.rs`** — Removed inline EmergencyDenyState enum, EMERGENCY_DENY_* constants, emergency_deny_state() method, activate_emergency_deny() method, and public emergency_deny_keyspace() accessor. Now imports EmergencyDenyState from sibling module.

2. **`crates/vb_storage/src/journal/emergency_deny.rs`** (NEW) — Dedicated module with EmergencyDenyState enum (Inactive/Active, no seq), fixed key `active`, exact one-byte version 1 marker `[1]`, emergency_deny_state() and activate_emergency_deny() methods. Activation validates existing marker first, always performs strict-batch commit+SyncAll.

3. **`crates/vb_storage/src/journal/mod.rs`** — Added module declaration and public export for emergency_deny.

4. **`crates/vb_storage/src/batch/putters.rs`** — Changed put_emergency_deny() from public to pub(crate), removed key parameter (bound to fixed key `active` internally), changed value type from [u8; 9] to [u8; 1].

5. **`fuzz/src/journal_target/errors.rs`** — Added emergency deny error variants to exhaustive match.

6. **`fuzz/fuzz_targets/journal_decode.rs`** — Added emergency deny error variants to exhaustive match.

7. **`fuzz/fuzz_targets/decode_record.rs`** — Added emergency deny error variants to exhaustive match.

8. **`fuzz/tests/proptest_journal_error_exhaustiveness.rs`** — Added emergency deny error variants to exhaustive match.

### Test Files Modified

9. **`crates/vb_storage/src/journal/emergency_deny_tests.rs`** (NEW) — 11 tests covering all requirements. All tests use Result return and ?/let-else patterns. Zero .expect(), .unwrap(), .panic(), unchecked indexing, or unchecked arithmetic.

## Gates Run

- **DENY_V2_09**: `cargo test -p vb_storage emergency_deny` — 11/11 passed
- **DENY_V2_11**: `moon ci :fmt :source-length :lint-src` — passed (all 9 tasks completed)

## Key Decisions

1. **Marker encoding simplified**: Changed from 9-byte (version + u64 seq) to exact 1-byte version marker. No sequence counter, so repeated activation is genuinely idempotent with full equality.

2. **Public API simplified**: Removed `emergency_deny_keyspace()` public accessor. Batch marker putter is now pub(crate) with fixed key bound internally.

3. **Activation always persists**: Even if already Active, activation performs strict-batch commit+SyncAll because prior failed persistence can leave visible but not durably confirmed data.

4. **Validate before write**: Activation reads and validates existing marker first, rejecting corrupt/unsupported state without overwriting it.

5. **Test hygiene**: All 11 new tests use Result return, ?/let-else patterns, and .first()/.get() for safe indexing. No .expect(), .unwrap(), or unchecked array indexing.

## File Line Counts

- `crates/vb_storage/src/journal/core.rs`: 280 non-blank lines (within 300 limit)
- `crates/vb_storage/src/batch/putters.rs`: 299 lines (within 300 limit)
- `crates/vb_storage/src/error/codes.rs`: 299 lines (within 300 limit)

## Gaps / Open Items

- No Runtime changes made (as required).
- Original run event tests preserved unchanged.
- Keyspace count updated to 11 (mechanical).
- Contract document updated to reflect simplified schema.
- No bead closure performed.
- No `bd`/`git`/`jj` mutations performed.
