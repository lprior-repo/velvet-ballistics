bead_id: vb-j0m0
bead_title: quality: Add unsafe boundary fuzz harnesses
phase: 2
updated_at: 2026-05-17T20:35:00Z
attempt: 1-of-7

# Research Notes: Unsafe-Adjacent Boundary Mapping

## 1. IPC Frame Boundary (vb_ipc/src/frame.rs)

### Functions Targeted
- `decode_frame_header(bytes: &[u8; IPC_HEADER_LEN])` - Fixed-size header decode
- `decode_frame_payload(header, payload)` - Postcard payload decode after header validation
- `validate_frame_magic(bytes: &[u8])` - Magic byte validation (4 bytes minimum)
- `validate_frame_bounds(header, max_payload)` - Payload length bounds check
- `read_frame_header<R: Read>(reader)` - Read-based header decode
- `read_frame_header_bounded<R: Read>(reader, max_payload)` - Bounded read-based header decode
- `read_frame_payload<R: Read>(reader, header)` - Read-based payload read
- `read_frame_payload_bounded<R: Read>(reader, header, max_payload)` - Bounded read-based payload read
- `encode_frame(command, flags, correlation, payload)` - Frame encoding
- `write_frame<W: Write>(writer, ...)` - Frame writing

### Error Types (IpcError)
- `PayloadTooLarge { actual, limit }` - Payload exceeds limit
- `InvalidMagic { actual }` - Wrong magic bytes
- `UnsupportedVersion { actual }` - Unsupported version
- `UnknownCommand(u16)` - Unknown command ID
- `ReservedNonZero { actual }` - Reserved field non-zero
- `PayloadLengthMismatch { header, actual }` - Header/actual mismatch
- `HeaderEncodeFailed` / `HeaderDecodeFailed` - Encode/decode failures
- `PayloadLengthOutOfRange { actual }` - u32 doesn't fit usize
- `PayloadEncodeFailed` / `PayloadDecodeFailed` - Postcard failures
- `ResponseDecodeFailed` - Response decode failure
- `Full` / `Disconnected` - Queue state errors

### Existing Fuzz Coverage
- `fuzz_ipc_frame` in fuzz/src/lib.rs exists but is a thin wrapper
- No explicit typed error assertion harness for truncated/oversized/malformed inputs

### Gap
Need dedicated harness that:
1. Feeds truncated frames (< IPC_HEADER_LEN bytes)
2. Feeds oversized payloads (header declares large payload_len)
3. Feeds malformed headers (wrong magic, bad version, non-zero reserved)
4. Asserts typed errors are returned (no panic, no OOM)

## 2. Storage Envelope Decoding (vb_storage/src/codec/)

### Functions Targeted
- `decode_record<T>(bytes, expected_magic, max_payload_len)` - Full envelope decode
- `encode_record<T>(magic, kind, sequence, payload, max_payload_len)` - Envelope encode
- `decode_record_header(bytes, expected_magic)` - Header decode
- `encode_record_header(magic, kind, sequence, payload_len)` - Header encode
- `verify_digest_match(payload, expected_digest)` - Digest verification

### Error Types (JournalError)
- `UnexpectedEof` - Truncated data
- `HeaderChecksumMismatch` - Header checksum failure
- `PayloadDigestMismatch` - Payload digest failure
- `PostcardDecodeFailed` - Postcard deserialization failure
- `BadMagic { actual, expected }` - Wrong magic
- `PayloadTooLarge { actual, limit }` - Payload exceeds limit
- `RecordKindFamilyMismatch { magic, kind_id }` - Kind/magic mismatch
- `UnknownRecordKind { id }` - Unknown record kind
- `UnsupportedSchemaVersion { version }` - Unsupported version
- `HeaderLengthMismatch { expected, actual }` - Header length mismatch
- `SequenceOverflow` - Sequence number overflow
- `WrongRun { expected, actual }` - Run ID mismatch
- `SequenceGap { expected, actual }` - Sequence gap

### Existing Fuzz Coverage
- `fuzz_journal_event` exists
- `fuzz_vb_qi37_12_persisted_payload_decode` has truncation/corruption tests
- `fuzz_recovery_decode` exercises recovery decode

### Gap
Need dedicated harness for:
1. Corrupt digest verification
2. Invalid envelope structures
3. Truncated envelope data at various offsets
4. Typed error assertions for each failure mode

## 3. Binary Payload Decoding (vb_storage/src/binary.rs, vb_ipc/src/codec.rs)

### Functions Targeted
- `vb_storage::binary.rs` - Binary payload handling
- `vb_ipc::codec.rs` - IPC codec (754 bytes, minimal)

### Existing Fuzz Coverage
- `fuzz_vb_qi37_12_persisted_payload_decode` covers some binary decode
- `fuzz_accepted_artifact_decode` covers artifact decode

### Gap
Need harness for:
1. Oversized binary payloads (fail before allocation)
2. Malformed encoding attacks (e.g., postcard length prefix attacks)
3. Encoding boundary conditions

## 4. External Input Adapters (vb_boundary_inventory)

### Functions Targeted
- `parse_inventory(data: &[u8])` - Parse boundary inventory from bytes
- `classify_boundary(...)` - Classify boundary types
- `discover_boundaries(...)` - Discover boundaries in workspace
- `validate_inventory(...)` - Validate inventory completeness
- `validate_evidence_reference_bytes(...)` - Validate evidence reference bytes

### Existing Fuzz Coverage
- `boundary_inventory_parser.rs` - Simple libfuzzer target calling `parse_inventory`

### Gap
Need comprehensive harness for:
1. Malformed inventory data
2. Invalid boundary classifications
3. Evidence reference validation with hostile input

## Risk Assessment

| Boundary | Risk Level | Existing Coverage | Gap Severity |
|----------|-----------|------------------|--------------|
| IPC Frame | HIGH | Partial | MEDIUM - needs typed error assertions |
| Storage Envelope | HIGH | Partial | MEDIUM - needs corrupt digest tests |
| Binary Payload | MEDIUM | Partial | LOW - some coverage exists |
| External Input Adapters | MEDIUM | Minimal | HIGH - needs comprehensive harness |

## Verifier Modes Required
- Unit tests with property-based assertions (proptest-style)
- Fuzz smoke tests (cargo-fuzz stdin mode)
- No formal verification required (fuzz is the verification method)
