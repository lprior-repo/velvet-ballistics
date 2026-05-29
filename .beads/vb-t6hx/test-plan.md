# Test Plan — vb-t6hx State 8 (CLI Doctor Scan Tests)

planner_skill: test-planner
planner_invocation_id: test-planner-vb-t6hx-state8-001
plan_state: 8
bead: vb-t6hx
workspace: /home/lewis/isolated/femdation-velvet-ballistics/vb-t6hx
parent_invocation: femdation-controller-vb-t6hx-state8
input_bridge_review: proof-reviewer-vb-t6hx-state7-bridge-001 (APPROVED)
target_test_file: crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs

## Scope

This test plan covers integration/behavior tests for the CLI doctor command's storage scan and decode operations. Tests verify that the doctor command correctly opens storage in read-only mode, scans with bounded limits, decodes journal envelopes, handles skip-decode projection, applies safe numeric filters, reports parse/decode errors with typed error categories, and respects the `--no-color` output mode.

Tests call production APIs in `vb_storage::codec`, `vb_storage::constants`, `vb_storage::error`, and the CLI doctor command dispatch. Tests must not mutate real storage databases; use in-memory fixtures or tempdir-based test databases.

## Production APIs Under Test

| Module | Function/Symbol | Line | Role |
|---|---|---|---|
| `vb_storage::codec::header` | `decode_record_header` | header.rs:26 | Header validation: magic, schema, kind, length, CRC |
| `vb_storage::codec` | `decode_journal_event` | mod.rs:54 | Full envelope → postcard decode + semantic validation |
| `vb_storage::codec` | `decode_record` | mod.rs:35 | Generic envelope → postcard deserialization |
| `vb_storage::constants` | `MAGIC_JOURNAL_EVENT` | constants.rs:52 | Expected magic (0x5642_4A45) |
| `vb_storage::constants` | `RECORD_HEADER_BYTES` | constants.rs:74 | Header size = 60 bytes |
| `vb_storage::constants` | `MAX_JOURNAL_EVENT_PAYLOAD_BYTES` | constants.rs:78 | Max payload = 1,048,576 bytes |
| `vb_storage::constants` | `DIGEST_BYTES` | constants.rs:72 | BLAKE3 digest size |
| `vb_storage::constants` | `RECORD_HEADER_LEN` | constants.rs:46 | Contract header length |
| `vb_storage::constants` | `CURRENT_SCHEMA_VERSION` | constants.rs:48 | Current schema version |
| `vb_storage::error` | `JournalError` | error/mod.rs:20 | All journal error variants |
| `vb_cli::args` | `parse_doctor` | args.rs:1357 | CLI argument parsing for doctor command |
| `vb_cli::app_impl` | `cmd_doctor` | app_impl.rs:5512 | Doctor command execution |

## Test Scenarios

### 1. Read-Only Open

**Property**: Storage opened for doctor scan/get must not mutate keys, events, or metadata.

**Scenarios**:

| ID | Scenario | Steps | Assertions |
|---|---|---|---|
| T8-RO-01 | Doctor scan does not append new events | 1. Seed a temp FjallJournal with N known events. 2. Run doctor scan with limit M. 3. Re-read all events with raw iteration. | Event count unchanged. Event data unchanged. No new sequences appended. |
| T8-RO-02 | Doctor get does not write test entries | 1. Seed a temp FjallJournal with N known events including key K. 2. Run doctor get on key K. 3. Verify journal state unchanged. | Key count unchanged. Key K value unchanged. No write-ahead log mutation. |
| T8-RO-03 | Doctor storage open with invalid keyspace path fails before touching storage | 1. Attempt doctor with nonexistent keyspace path. 2. Observe error message. 3. Verify storage directory is uncreated/untouched. | Error returned. No storage file was created. Error message mentions path. |
| T8-RO-04 | Deterministic read: same scan twice produces identical output | 1. Seed storage with deterministic events. 2. Run scan twice with identical args. 3. Compare outputs byte-for-byte. | Outputs are identical. No timestamps or randomness leak into output. |
| T8-RO-05 | Read-only open rejects write commands | 1. Open storage for doctor read. 2. Attempt to call write-path function (append_event, compact, etc.) through the read-only handle. 3. Verify rejection. | Write attempt returns error or panics via type-system prevention. Error is typed (not a raw panic). |

**Data setup**: Create a tempdir, open a `fjall::Config::new(&path).open()` journal, write N synthetic `JournalEvent` records using `encode_record` + journal append, then pass the path to doctor.

### 2. Bounded Scan

**Property**: Scan emits at most the requested limit rows. Overflow, zero, and missing limits are handled safely.

**Scenarios**:

| ID | Scenario | Steps | Assertions |
|---|---|---|---|
| T8-BS-01 | Scan with limit L ≤ event count returns exactly L rows | 1. Seed storage with 100 events. 2. Run scan with --limit 5. | Output contains exactly 5 rows. No truncation metadata missing. |
| T8-BS-02 | Scan with limit L > event count returns all events | 1. Seed storage with 7 events. 2. Run scan with --limit 100. | Output contains exactly 7 rows. No padding or phantom rows. |
| T8-BS-03 | Scan with limit=0 returns zero rows (not an error) | 1. Seed storage with events. 2. Run scan with --limit 0. | Exit 0. Zero rows in output. No error message. |
| T8-BS-04 | Scan with limit=1 returns 1 row | 1. Seed storage with 50 events. 2. Run scan with --limit 1. | Output contains exactly 1 row. Remaining rows are not silently fetched. |
| T8-BS-05 | Negative limit rejected at parse time | 1. Pass `--limit -5` to doctor. 2. Observe parse error. | Parse error returned before storage open. Error message mentions limit. |
| T8-BS-06 | Non-numeric limit rejected at parse time | 1. Pass `--limit abc` to doctor. 2. Observe parse error. | Parse error returned before storage open. |
| T8-BS-07 | Overflow limit (u64::MAX) handled safely without crash | 1. Pass `--limit 18446744073709551615`. 2. Run scan. | Scans all events OR returns graceful oversize error. No panic, no hang, no OOM. |
| T8-BS-08 | Scan with no --limit flag uses default limit | 1. Seed with 500 events. 2. Scan without --limit. | Uses documented default. Output count ≤ default limit. |

**Data setup**: For each test, seed with appropriate event count. Use the `cmd_doctor` function with `Option<&Path>` and `OutputFormat` arguments.

### 3. Envelope Decode

**Property**: Journal record envelope validation (magic, schema, kind, header length, payload length, header CRC, payload availability, payload digest) occurs before Postcard deserialization. Errors are classified into pre-postcard and post-postcard categories.

**Scenarios**:

| ID | Scenario | Steps | Assertions |
|---|---|---|---|
| T8-ED-01 | Valid record decodes correctly | 1. Encode a valid JournalEvent through the production encode path. 2. Decode with `decode_journal_event`. | Returns Ok((envelope, event)). Envelope metadata correct. Event fields match original. |
| T8-ED-02 | Truncated header (< 60 bytes) yields UnexpectedEof | 1. Create a byte slice of 30 bytes. 2. Call `decode_record_header`. | Err(JournalError::UnexpectedEof). Does not reach postcard. |
| T8-ED-03 | Bad magic yields BadMagic | 1. Build a valid header but set magic to 0xDEADBEEF. 2. Call `decode_record_header`. | Err(JournalError::BadMagic { found: 0xDEADBEEF }). |
| T8-ED-04 | Unknown schema version yields UnsupportedSchemaVersion | 1. Build a valid header with schema_version = 999. 2. Call `decode_record_header`. | Err(JournalError::UnsupportedSchemaVersion { version: 999 }). |
| T8-ED-05 | Unknown record kind yields UnknownRecordKind | 1. Build a valid header with record_kind = 9999. 2. Call `decode_record_header`. | Err(JournalError::UnknownRecordKind { kind: 9999 }). |
| T8-ED-06 | Record kind family mismatch detected | 1. Build header with journal magic but artifact-family kind. 2. Call `decode_record_header`. | Err(JournalError::RecordKindFamilyMismatch). |
| T8-ED-07 | Wrong header_len yields HeaderLengthMismatch | 1. Build header with header_len != RECORD_HEADER_LEN. 2. Call `decode_record_header`. | Err(JournalError::HeaderLengthMismatch). |
| T8-ED-08 | Payload too large yields PayloadTooLarge | 1. Build header with payload_len > max_payload_len. 2. Call `decode_record_header` with bound=1024. | Err(JournalError::PayloadTooLarge { len, max }). |
| T8-ED-09 | Bad header CRC yields HeaderChecksumMismatch | 1. Build valid header, flip one byte in header checksum area. 2. Call `decode_record_header`. | Err(JournalError::HeaderChecksumMismatch). |
| T8-ED-10 | Valid header + truncated payload yields UnexpectedEof | 1. Build valid header with payload_len=100. 2. Provide only 50 bytes of payload. 3. Call `decode_journal_event`. | Err(JournalError::UnexpectedEof). Does not reach PostcardDecodeFailed. |
| T8-ED-11 | Valid header + valid payload + bad digest yields PayloadDigestMismatch | 1. Build valid header + valid payload bytes. 2. Corrupt the digest bytes. 3. Call `decode_journal_event`. | Err(JournalError::PayloadDigestMismatch). |
| T8-ED-12 | Valid envelope + structurally valid but semantically invalid event yields InvalidEvent | 1. Encode a JournalEvent with run_id=0. 2. Call `decode_journal_event`. | Err(JournalError::InvalidEvent). Postcard deserialization succeeded but semantic validation failed. |
| T8-ED-13 | Valid envelope + valid event → success | 1. Encode a correct JournalEvent (valid run_id, seq, attempt). 2. Call `decode_journal_event`. | Ok((envelope, event)). Envelope fields match. |

**Data setup**: Use `vb_storage::codec::encode_record` to create valid records, then manually corrupt specific fields for error cases. Use `vb_storage::codec::decode_record_header` and `vb_storage::codec::decode_journal_event` for decode verification.

### 4. Skip-Decode Projection

**Property**: Projection scan (default mode) must not Postcard-decode every value. Only the header metadata is extracted; the payload body is skipped unless explicitly requested.

**Scenarios**:

| ID | Scenario | Steps | Assertions |
|---|---|---|---|
| T8-SD-01 | Projection scan extracts header metadata without payload decode | 1. Seed storage with events containing various payloads. 2. Run doctor scan in projection (default) mode. 3. Inspect output. | Header fields (seq, run_id hash, kind) present. Payload body NOT fully decoded (preview only). |
| T8-SD-02 | Skip-decode tolerates postcard-invalid payloads | 1. Seed storage with a record whose header is valid but postcard payload is malformed bytes. 2. Run scan in skip-decode mode. | Scan succeeds (no error). Malformed record listed with header metadata. No PostcardDecodeFailed error in output. |
| T8-SD-03 | Explicit decode flag triggers full payload decode | 1. Seed storage with a valid record. 2. Run doctor get with --decode flag. | Full payload decoded. Event fields (run_id, seq, step_id, kind) displayed. |
| T8-SD-04 | Explicit decode on malformed payload reports PostcardDecodeFailed | 1. Seed storage with malformed postcard payload under valid header. 2. Run doctor get with --decode. | Error output contains PostcardDecodeFailed or classified decode error. Does not crash. |
| T8-SD-05 | Batch scan decode vs skip-decode consistency: header metadata identical in both modes | 1. Seed storage with mixed valid/invalid records. 2. Scan in projection mode, record header metadata. 3. Scan in decode mode, record header metadata. | Header metadata (seq, kind) matches between modes for each record. Only payload decode differs. |

### 5. Safe Numeric Filters

**Property**: Numeric filter arguments (sequence ranges, event counts, limits) are parsed safely and reject overflow, negative values, non-numeric strings, and out-of-range inputs.

**Scenarios**:

| ID | Scenario | Steps | Assertions |
|---|---|---|---|
| T8-SN-01 | Sequence range filter: --from 5 --to 10 returns events in [5,10] | 1. Seed storage with events seq=1..20. 2. Run doctor scan --from 5 --to 10. | Output contains events with seq in [5,10] only. No events seq<5 or seq>10. |
| T8-SN-02 | Sequence range filter: --from with no --to scans from start to end | 1. Seed with events seq=1..20. 2. Run doctor scan --from 15. | All events seq>=15 present. Events seq<15 absent. |
| T8-SN-03 | Sequence range filter: --to with no --from scans from beginning to --to | 1. Seed with events seq=1..20. 2. Run doctor scan --to 5. | All events seq<=5 present. Events seq>5 absent. |
| T8-SN-04 | Sequence range: from > to yields empty result (not error) | 1. Seed with events. 2. Run doctor scan --from 10 --to 5. | Exit 0. Zero rows. No error message. |
| T8-SN-05 | Sequence from=0 rejected or handled safely | 1. Seed with events (seq starts at 1). 2. Run doctor scan --from 0. | Either rejected at parse time OR returns events from seq=1. No crash. No UB. |
| T8-SN-06 | Sequence values at u64::MAX handled safely | 1. Run doctor scan --from 18446744073709551615. | Graceful handling: empty result or parse rejection. No panic, no overflow. |
| T8-SN-07 | Negative sequence value rejected at parse time | 1. Pass --from -1 to doctor. | Parse error. Does not open storage. |
| T8-SN-08 | Non-numeric sequence value rejected | 1. Pass --from abc to doctor. | Parse error. Does not open storage. |

### 6. Parse/Decode Errors

**Property**: Error paths are exercised and produce typed, informative error messages. Errors are classified into parse-time (before storage open) and decode-time (during storage scan) categories.

**Scenarios**:

| ID | Scenario | Steps | Assertions |
|---|---|---|---|
| T8-PE-01 | Invalid keyspace path yields useful error | 1. Run doctor with `--db /nonexistent/path/12345`. | Error message mentions path. No panic. Exit code non-zero. |
| T8-PE-02 | Corrupt Fjall journal: bad magic in first record | 1. Create a raw file with bad magic bytes, pretend it's a Fjall keyspace. 2. Run doctor on it. | Error message classified (BadMagic or Fjall-level error). No panic. |
| T8-PE-03 | Corrupt Fjall journal: truncated mid-record | 1. Create a file with partial record data. 2. Run doctor. | Error message classified (UnexpectedEof or Fjall corruption error). No panic. |
| T8-PE-04 | Conflicting flags rejected: --scan and --get together | 1. Run doctor --scan --get abcd1234. | Parse error: conflicting flags. Does not open storage. |
| T8-PE-05 | Missing required arg: --get without key value | 1. Run doctor --get (no key argument). | Parse error: missing argument. Does not open storage. |
| T8-PE-06 | Invalid hex key: odd length | 1. Run doctor --get abc. | Parse error: invalid hex. Does not open storage. |
| T8-PE-07 | Invalid hex key: non-hex characters | 1. Run doctor --get xyz12. | Parse error: invalid hex. Does not open storage. |
| T8-PE-08 | Valid hex key but key not found in storage | 1. Seed storage with known keys. 2. Run doctor --get DEADBEEF (not in storage). | "Not found" or similar message. Exit code non-zero or zero with empty output (deterministic). |
| T8-PE-09 | Multiple flags in valid combination: --scan --limit 5 --from 3 | 1. Seed with events. 2. Run doctor with multiple valid flags. | All flags respected. No conflict error. Correct results. |
| T8-PE-10 | Unknown flag rejected | 1. Run doctor --nonexistent. | Parse error. Does not open storage. |

### 7. No-Color Mode

**Property**: When `--no-color` or `NO_COLOR` environment variable is set, output contains no ANSI escape sequences.

**Scenarios**:

| ID | Scenario | Steps | Assertions |
|---|---|---|---|
| T8-NC-01 | --no-color flag suppresses ANSI escape codes | 1. Seed storage. 2. Run doctor scan with --no-color. 3. Inspect output bytes. | Output contains no ANSI CSI sequences (no `\x1b[`). Plain text only. |
| T8-NC-02 | NO_COLOR=1 environment variable suppresses ANSI codes | 1. Set `NO_COLOR=1`. 2. Run doctor scan. 3. Inspect output bytes. | No ANSI escape codes in output. |
| T8-NC-03 | Default mode (no --no-color, no NO_COLOR) may produce color | 1. Seed storage. 2. Run doctor scan with default settings. 3. Inspect output. | Output may or may not contain ANSI codes depending on terminal detection. Test verifies either outcome is valid (no crash). |
| T8-NC-04 | --no-color and error output: error messages also suppressed | 1. Run doctor with --no-color on nonexistent path. 2. Inspect stderr. | Error messages on stderr contain no ANSI escape codes. |
| T8-NC-05 | --color and --no-color conflict: last wins | 1. Run doctor --color --no-color. 2. Inspect output. | --no-color wins OR parse error for conflicting flags. Deterministic behavior. |
| T8-NC-06 | Color mode in piped output: auto-detects non-TTY | 1. Pipe doctor output through `cat`. 2. Inspect output. | No ANSI codes in piped output (auto-detection). |

## Test Structure

All new tests are added to `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs` as integration test functions. Each test function is named with a consistent prefix for the scenario group:

```
read_only_*        — Read-only open scenarios (T8-RO-*)
bounded_scan_*     — Bounded scan scenarios (T8-BS-*)
envelope_decode_*  — Envelope decode scenarios (T8-ED-*)
skip_decode_*      — Skip-decode projection scenarios (T8-SD-*)
safe_numeric_*     — Safe numeric filter scenarios (T8-SN-*)
parse_decode_*     — Parse/decode error scenarios (T8-PE-*)
no_color_*         — No-color mode scenarios (T8-NC-*)
```

### Test Organization

Tests requiring a real FjallJournal database (read-only open, bounded scan, skip-decode projection, safe numeric filters, parse/decode errors for corrupt journals, no-color mode) use tempdir-based test databases seeded with known records via `vb_storage::codec::encode_record` and direct journal append.

Tests that only exercise the codec decode path (envelope decode validation errors) use pure in-memory byte buffers calling `decode_record_header` or `decode_journal_event` directly, without a database.

### Dependencies

- `tempfile` — tempdir creation for test databases
- `fjall` — journal open/write/read for seed data
- `vb_storage::codec` — `encode_record`, `decode_record_header`, `decode_journal_event`
- `vb_storage::constants` — `MAGIC_JOURNAL_EVENT`, `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`, `RECORD_HEADER_BYTES`, `CURRENT_SCHEMA_VERSION`
- `vb_storage::error::JournalError` — error variant matching
- `vb_storage::events::JournalEvent` — event construction for seed data
- `vb_cli::app_impl::cmd_doctor` — doctor command execution (for CLI-level tests)
- `vb_cli::args::parse_doctor` — argument parsing (for parse-error tests)

### Test Count Summary

| Group | Count | Database Required |
|---|---|---|
| Read-only open | 5 | Yes (FjallJournal) |
| Bounded scan | 8 | Yes |
| Envelope decode | 13 | No (in-memory bytes) |
| Skip-decode projection | 5 | Yes |
| Safe numeric filters | 8 | Partially (parse tests = no DB) |
| Parse/decode errors | 10 | Partially |
| No-color mode | 6 | Yes |
| **Total** | **55** | |

## Evidence Commands

```bash
# Run all doctor storage scan tests
cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- --nocapture

# Run specific groups
cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- read_only
cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- bounded_scan
cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- envelope_decode
cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- skip_decode
cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- safe_numeric
cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- parse_decode
cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- no_color
```

## Contract Traceability

All scenarios trace to the bead's contract clauses from `contract.md`:

| Contract Clause | Test Groups |
|---|---|
| Functional Contract 3: Invalid keyspace/hex/numeric/conflicting flags fail before opening storage | parse_decode, safe_numeric |
| Functional Contract 4: Storage scan/get uses read-only capability, must not mutate | read_only |
| Functional Contract 5: Scan emits at most the requested ScanLimit rows | bounded_scan |
| Functional Contract 7: Large values render as bounded previews with truncation hint | bounded_scan, skip_decode |
| Functional Contract 9: Projection scan defaults to skip-decode, must not decode every value | skip_decode |
| Functional Contract 10: Envelope decode validates envelope before Postcard | envelope_decode |
| UI Contract: --no-color and NO_COLOR suppress ANSI escape codes | no_color |

## Proof/Refinement Coverage Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| PO-vb-t6hx-R02 | Scan rows never exceed limit | Yes | `decode_record_header` (header.rs:26) | `proptest_doctor_scan_rows_never_exceed_limit` | `restate_doctor_storage_scan_decode_tests.rs` | proptest | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests` | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- proptest_doctor_scan_rows_never_exceed_limit` |
| PO-vb-t6hx-R05 | Invalid hex rejected before storage open | Yes | `decode_record_header` (header.rs:26) | `proptest_invalid_hex_rejected_before_storage_open` | `restate_doctor_storage_scan_decode_tests.rs` | proptest | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests` | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- proptest_invalid_hex_rejected_before_storage_open` |
| PO-vb-t6hx-R08 | Envelope decode errors before postcard | Yes | `decode_journal_event` (mod.rs:54) | `proptest_envelope_decode_errors_before_postcard` | `restate_doctor_storage_scan_decode_tests.rs` | proptest | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests` | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- proptest_envelope_decode_errors_before_postcard` |
| PO-vb-t6hx-R12 | Large value preview truncated with hint | Yes | `decode_record_header` (header.rs:26) | `proptest_large_value_preview_truncated_with_hint` | `restate_doctor_storage_scan_decode_tests.rs` | proptest | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests` | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- proptest_large_value_preview_truncated_with_hint` |
| PO-vb-t6hx-R15 | Projection scan skips malformed decode | Yes | `decode_record_header` (header.rs:26) + `decode_journal_event` (mod.rs:54) | `proptest_projection_scan_skips_malformed_decode` | `restate_doctor_storage_scan_decode_tests.rs` | proptest | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests` | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- proptest_projection_scan_skips_malformed_decode` |
| PO-vb-t6hx-R18 | Readonly inventory unchanged | Yes | `decode_journal_event` (mod.rs:54) | `proptest_doctor_storage_readonly_inventory_unchanged` | `restate_doctor_storage_scan_decode_tests.rs` | proptest | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests` | `cargo test -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests -- proptest_doctor_storage_readonly_inventory_unchanged` |
| PO-vb-t6hx-R03 | Doctor scan args fuzz | Yes | `decode_record_header` (header.rs:26) | `vb_t6hx_doctor_scan_args` | `fuzz/fuzz_targets/vb_t6hx_doctor_scan_args.rs` | cargo-fuzz | `cargo +nightly fuzz run --sanitizer none vb_t6hx_doctor_scan_args -- -max_total_time=3` | `cargo +nightly fuzz run --sanitizer none vb_t6hx_doctor_scan_args` |
| PO-vb-t6hx-R06 | Doctor get args fuzz | Yes | `decode_record_header` (header.rs:26) + `decode_journal_event` (mod.rs:54) | `vb_t6hx_doctor_get_args` | `fuzz/fuzz_targets/vb_t6hx_doctor_get_args.rs` | cargo-fuzz | `cargo +nightly fuzz run --sanitizer none vb_t6hx_doctor_get_args -- -max_total_time=3` | `cargo +nightly fuzz run --sanitizer none vb_t6hx_doctor_get_args` |
| PO-vb-t6hx-R09 | Envelope decode fuzz | Yes | `decode_journal_event` (mod.rs:54) | `vb_t6hx_envelope_decode` | `fuzz/fuzz_targets/vb_t6hx_envelope_decode.rs` | cargo-fuzz | `cargo +nightly fuzz run --sanitizer none vb_t6hx_envelope_decode -- -max_total_time=3` | `cargo +nightly fuzz run --sanitizer none vb_t6hx_envelope_decode` |
| PO-vb-t6hx-R10 | Doctor decode CLI fuzz | Yes | `decode_journal_event` (mod.rs:54) | `vb_t6hx_doctor_decode_cli` | `fuzz/fuzz_targets/vb_t6hx_doctor_decode_cli.rs` | cargo-fuzz | `cargo +nightly fuzz run --sanitizer none vb_t6hx_doctor_decode_cli -- -max_total_time=3` | `cargo +nightly fuzz run --sanitizer none vb_t6hx_doctor_decode_cli` |
| PO-vb-t6hx-R13 | Bounded preview fuzz | Yes | `decode_record_header` (header.rs:26) | `vb_t6hx_bounded_preview` | `fuzz/fuzz_targets/vb_t6hx_bounded_preview.rs` | cargo-fuzz | `cargo +nightly fuzz run --sanitizer none vb_t6hx_bounded_preview -- -max_total_time=3` | `cargo +nightly fuzz run --sanitizer none vb_t6hx_bounded_preview` |
| PO-vb-t6hx-R16 | Projection skip decode fuzz | Yes | `decode_record_header` (header.rs:26) + `decode_journal_event` (mod.rs:54) | `vb_t6hx_projection_skip_decode` | `fuzz/fuzz_targets/vb_t6hx_projection_skip_decode.rs` | cargo-fuzz | `cargo +nightly fuzz run --sanitizer none vb_t6hx_projection_skip_decode -- -max_total_time=3` | `cargo +nightly fuzz run --sanitizer none vb_t6hx_projection_skip_decode` |
| PO-vb-t6hx-R07 | Postcard envelope wire Kani | Yes | `decode_journal_event` (mod.rs:54) | `kani_postcard_envelope_wire` | `crates/vb_storage/src/kani_postcard_envelope_wire.rs` | kani | `cargo kani --only-codegen -p vb_storage` | `cargo kani -p vb_storage --harness kani_postcard_envelope_wire` |
| PO-vb-t6hx-R01 | Scan limit Kani | Yes | `cmd_doctor` (app_impl.rs) | `kani_vb_t6hx_scan_limit` | `crates/vb_cli/src/kani_vb_t6hx_scan_limit.rs` | kani | BLOCKED (CLI_KANI_MODULE_BLOCKER) | N/A |
| PO-vb-t6hx-R04 | Hex key Kani | Yes | `cmd_doctor` (app_impl.rs) | `kani_vb_t6hx_hex_key` | `crates/vb_cli/src/kani_vb_t6hx_hex_key.rs` | kani | BLOCKED (CLI_KANI_MODULE_BLOCKER) | N/A |
| PO-vb-t6hx-R11 | Bounded preview Kani | Yes | `cmd_doctor` (app_impl.rs) | `kani_vb_t6hx_bounded_preview` | `crates/vb_cli/src/kani_vb_t6hx_bounded_preview.rs` | kani | BLOCKED (CLI_KANI_MODULE_BLOCKER) | N/A |
| PO-vb-t6hx-R14 | Skip decode Kani | Yes | `cmd_doctor` (app_impl.rs) | `kani_vb_t6hx_skip_decode` | `crates/vb_cli/src/kani_vb_t6hx_skip_decode.rs` | kani | BLOCKED (CLI_KANI_MODULE_BLOCKER) | N/A |
| PO-vb-t6hx-R17 | Readonly doctor Kani | Yes | `cmd_doctor` (app_impl.rs) | `kani_vb_t6hx_readonly_doctor` | `crates/vb_cli/src/kani_vb_t6hx_readonly_doctor.rs` | kani | BLOCKED (CLI_KANI_MODULE_BLOCKER) | N/A |

## Non-Goals

- Performance benchmarks (deferred to perf lane)
- Cross-platform terminal detection nuances (TTY detection = OS detail)
- Fjall corruption recovery (storage-layer concern, not CLI doctor concern)
- Scanning across multiple keyspaces in one invocation
- Network-remote or distributed journal access
- Concurrent doctor access (read-only is inherently safe)
- Canonicalizing error message text (messages may evolve)
- Testing every JournalError variant exhaustively (representative classes only)

## Prerequisites for Test Execution

1. `tempfile` crate available in workspace_tests `Cargo.toml`
2. `fjall` crate available in workspace_tests `Cargo.toml`
3. `vb_storage` and `vb_cli` as dependencies of workspace_tests
4. `vb_core` available for `RunId` and `EventSeq` construction in seed data
5. Postcard available for serialization

## Handoff

This test plan covers 55 scenarios across 7 groups. Each group maps to specific contract clauses. New tests are added to the existing `restate_doctor_storage_scan_decode_tests.rs` file using consistent naming prefixes. Tests that require a real database use tempdir-based FjallJournals. Codec-level tests use in-memory byte buffers.

The 6 existing proptest properties (R02, R05, R08, R12, R15, R18) are preserved and complemented, not replaced, by these integration tests. The proptest properties remain the primary property-test evidence; these integration tests add scenario-specific coverage for CLI behavior.
