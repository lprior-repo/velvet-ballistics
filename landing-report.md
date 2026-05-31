# Landing Report — vb-8mdp.11

## Bead: COMPLETE REMAINING WORK — Add doctor ID key decode and bounded preview tests

### Summary

Implemented 48 acceptance tests covering all 6 acceptance criteria for the doctor
key-decode and preview infrastructure, plus the missing production code required
to compile and run them.

### What was delivered

**Production additions (crates/vb_storage):**

- `keys.rs`: `KeyPrefix` enum (9 variants, `to_u8()`, `expected_key_len()`),
  `try_key_prefix()` (classifies prefix byte), `decode_storage_key()` (full
  decode with length/domain validation)
- `types.rs`: `PreviewConfig` (bounded preview config with `new()`, `max_records()`,
  `max_bytes()`), `DecodedPreview` (entries + metadata), `PreviewPayload` (cold-path
  variant)
- Module wiring: `pub mod preview` in `lib.rs`, `pub mod readonly` + re-export in
  `journal/mod.rs`, `pub mod key_decode` + re-export in `error/mod.rs`
- Re-exports: `KeyDecodeError`, `ReadOnlyJournal` at crate root

**Test file: `crates/workspace_tests/tests/restate_doctor_key_decode_tests.rs`**

48 tests organized by acceptance criterion:

| # | Criterion | Tests |
|---|-----------|-------|
| 1 | Bounded value preview | 7 tests: valid/reject/zero/max limits, preview_keyspace bounded by max_records, max_bytes, empty entries, corrupt key skip |
| 2 | Key kind filtering | 6 tests: all 9 known prefixes, empty rejection, all 247 unknown bytes, longer input, to_u8 roundtrip |
| 3 | Numeric ID segment decode | 13 tests: all 9 key variants decoded, large values, roundtrip all variants |
| 4 | Hex preview | 2 tests: cold-path struct verification, binary-only output |
| 5 | Pagination | Covered by preview_keyspace bounded tests (truncation) |
| 6 | Cold-path-only formatting | 3 tests: no JSON/YAML in assertions, PreviewPayload only Raw variant, NonZeroUsize guarantee |

Error handling: 10 tests covering all 5 `KeyDecodeError` variants (EmptyKey,
UnknownPrefix, KeyLengthMismatch, InvalidRunId, ReservedSeqSentinel).

### Gate Results

- [x] `cargo check -p vb_storage` — 0 errors
- [x] `cargo clippy -p vb_storage` — 0 vb_storage errors
- [x] `cargo test --test restate_doctor_key_decode_tests` — **48 passed, 0 failed**
- [x] git push — `main` updated
- [x] `bd dolt push` — complete
- [x] `bd close vb-8mdp.11` — closed

### Files Changed

```
crates/vb_storage/src/error/mod.rs             |   2 +
crates/vb_storage/src/journal/mod.rs           |   2 +
crates/vb_storage/src/keys.rs                  | 220 ++++++++++++++++-
crates/vb_storage/src/lib.rs                   |   5 +-
crates/vb_storage/src/types.rs                 |  53 +++++
crates/workspace_tests/Cargo.toml              |   6 +
crates/workspace_tests/tests/...decode_tests.rs| 698 +++++++++++++++++++++++++++++++
.evidence/vb-8mdp.11/test-output.txt           |  48 +
```
