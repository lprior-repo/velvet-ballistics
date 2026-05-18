bead_id: vb-j0m0
bead_title: quality: Add unsafe boundary fuzz harnesses
phase: 3
updated_at: 2026-05-17T20:40:00Z
attempt: 1-of-7

# Contract Specification: Unsafe Boundary Fuzz Harnesses

## Requirements

### R1: IPC Frame Boundary Fuzz Harness
- **Input**: Arbitrary byte sequences representing IPC frames
- **Boundary Functions**: `decode_frame_header`, `validate_frame_magic`, `validate_frame_bounds`, `decode_frame_payload`
- **Test Cases**:
  - R1.1: Empty input (0 bytes) - must return `HeaderDecodeFailed`
  - R1.2: Truncated header (1-3 bytes) - must return `HeaderDecodeFailed`
  - R1.3: Partial header (4 to IPC_HEADER_LEN-1 bytes) - must return `HeaderDecodeFailed`
  - R1.4: Valid header with wrong magic - must return `InvalidMagic`
  - R1.5: Valid header with oversized payload_len - must return `PayloadTooLarge` or `PayloadLengthOutOfRange`
  - R1.6: Valid header with non-zero reserved field - must return `ReservedNonZero`
  - R1.7: Valid header with unsupported version - must return `UnsupportedVersion`
  - R1.8: Valid header with unknown command - must return `UnknownCommand`
  - R1.9: Valid header with payload length mismatch - must return `PayloadLengthMismatch`
  - R1.10: Valid frame with valid payload - must decode successfully
- **Invariants**: No panic, no OOM, no unchecked indexing for any input

### R2: Storage Envelope Decoding Fuzz Harness
- **Input**: Arbitrary byte sequences representing storage envelopes
- **Boundary Functions**: `decode_record`, `decode_record_header`, `verify_digest_match`
- **Test Cases**:
  - R2.1: Empty input - must return `UnexpectedEof`
  - R2.2: Truncated header (< 60 bytes) - must return `UnexpectedEof` or `HeaderLengthMismatch`
  - R2.3: Wrong magic bytes - must return `BadMagic`
  - R2.4: Corrupt header checksum - must return `HeaderChecksumMismatch`
  - R2.5: Corrupt payload digest - must return `PayloadDigestMismatch`
  - R2.6: Oversized payload (exceeds max_payload_len) - must return `PayloadTooLarge`
  - R2.7: Invalid record kind - must return `UnknownRecordKind` or `RecordKindFamilyMismatch`
  - R2.8: Valid envelope with valid payload - must decode successfully
  - R2.9: Postcard decode failure - must return `PostcardDecodeFailed`
- **Invariants**: No panic, no OOM, no unchecked indexing for any input

### R3: Binary Payload Decoding Fuzz Harness
- **Input**: Arbitrary byte sequences representing binary payloads
- **Boundary Functions**: `decode_record` with various record types
- **Test Cases**:
  - R3.1: Oversized payload declaration - must fail before allocation
  - R3.2: Malformed postcard encoding - must return `PostcardDecodeFailed`
  - R3.3: Length prefix attack (huge length in small buffer) - must return typed error
  - R3.4: Encoding boundary conditions (empty payload, single byte, max size)
- **Invariants**: No panic, no OOM, no unchecked indexing for any input

### R4: External Input Adapter Fuzz Harness
- **Input**: Arbitrary byte sequences representing boundary inventory data
- **Boundary Functions**: `parse_inventory`, `validate_evidence_reference_bytes`
- **Test Cases**:
  - R4.1: Empty input - must return typed error
  - R4.2: Malformed inventory syntax - must return `InventoryParseFailure`
  - R4.3: Invalid boundary class - must return `UnknownBoundaryClass`
  - R4.4: Missing required fields - must return `IncompleteDiscoveryInput`
  - R4.5: Valid inventory - must parse successfully
- **Invariants**: No panic, no OOM, no unchecked indexing for any input

## Type/Domain Model

### Error Type Discipline
All boundary functions must return `Result<T, E>` where `E` is a typed error enum:
- IPC: `IpcError` (13 variants, all typed with diagnostic codes)
- Storage: `JournalError` (13 variants, all typed)
- Boundary Inventory: `BoundaryInventoryError` (12 variants, all typed)

### Fuzz Oracle Pattern
Each fuzz harness follows this pattern:
```rust
pub fn fuzz_<boundary>(data: &[u8]) {
    let result = boundary_function(data);
    match result {
        Ok(value) => assert_invariants(value),
        Err(e) => assert_typed_error(e),
    }
}
```

### Bounded Input Policy
- Maximum input size: 4096 bytes (MAX_FUZZ_PAYLOAD)
- No unbounded allocations based on input-declared sizes
- All length conversions use `checked_*` or `try_from` with typed errors

## Verification Layers

| Layer | Method | Coverage |
|-------|--------|----------|
| Unit Tests | Proptest-style property tests | Error path coverage |
| Fuzz Smoke | cargo-fuzz stdin mode | Arbitrary input coverage |
| Typed Error Assertions | Exhaustive match on error variants | Contract parity |

## Traceability

| Requirement | Boundary | Error Type | Fuzz Target |
|-------------|----------|------------|-------------|
| R1 | IPC Frame | IpcError | ipc_frame_fuzz_boundary |
| R2 | Storage Envelope | JournalError | storage_envelope_fuzz_boundary |
| R3 | Binary Payload | JournalError | binary_payload_fuzz_boundary |
| R4 | External Input | BoundaryInventoryError | external_input_adapter_fuzz |
