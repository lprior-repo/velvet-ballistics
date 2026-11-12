bead_id: vb-j0m0
bead_title: quality: Add unsafe boundary fuzz harnesses
phase: 12
updated_at: 2026-05-17T21:00:00Z
attempt: 1-of-7

# Black Hat Review: Unsafe Boundary Fuzz Harnesses

## Review Scope
- Four new fuzz harnesses: IPC frame boundary, storage envelope boundary, binary payload boundary, external input adapter boundary
- All harnesses target unsafe-adjacent boundaries identified in vb-y1zq boundary inventory
- Contract: malformed external input must return typed errors without panic, OOM, or unchecked indexing

## Contract Parity Check

### R1: IPC Frame Boundary
- PASS: Truncated frames (< 4 bytes) return `HeaderDecodeFailed`
- PASS: Wrong magic returns `InvalidMagic` or `HeaderDecodeFailed`
- PASS: Oversized payload_len triggers `PayloadTooLarge` or `PayloadLengthOutOfRange`
- PASS: Non-zero reserved field triggers `ReservedNonZero`
- PASS: Unsupported version triggers `UnsupportedVersion`
- PASS: Unknown command triggers `UnknownCommand`
- PASS: Payload length mismatch triggers `PayloadLengthMismatch`
- PASS: Valid frame decodes successfully
- PASS: All error variants are typed (`assert_typed_ipc_error` is exhaustive)

### R2: Storage Envelope Boundary
- PASS: Empty input returns `UnexpectedEof`
- PASS: Truncated header returns `UnexpectedEof` or `HeaderLengthMismatch`
- PASS: Wrong magic returns `BadMagic`
- PASS: Corrupt checksum returns `HeaderChecksumMismatch`
- PASS: Corrupt digest returns `PayloadDigestMismatch`
- PASS: Oversized payload returns `PayloadTooLarge`
- PASS: Invalid record kind returns `UnknownRecordKind` or `RecordKindFamilyMismatch`
- PASS: Valid envelope decodes successfully
- PASS: All error variants are typed (`assert_typed_journal_error` is exhaustive over 40+ variants)

### R3: Binary Payload Boundary
- PASS: Empty input returns `UnexpectedEof`
- PASS: Small max_payload_len triggers `PayloadTooLarge`
- PASS: Wrong magic triggers `BadMagic` or `RecordKindFamilyMismatch`
- PASS: All error paths return typed errors

### R4: External Input Adapter Boundary
- PASS: Empty input returns error (not panic)
- PASS: Malformed inventory returns typed error
- PASS: `validate_evidence_reference_bytes` never panics
- PASS: All error variants are typed (`assert_typed_boundary_error` is exhaustive)

## Farley Rigor Check
- PASS: Each harness has explicit typed error assertions (not just "no panic")
- PASS: Error assertion functions are exhaustive over all enum variants
- PASS: Bounded input policy enforced (no unbounded allocations from input-declared sizes)
- PASS: All length conversions use `try_from` or `checked_*` operations

## Holzman Rust Big 6 Check
- PASS: No `unsafe` in new fuzz harness code
- PASS: No `unwrap` or `expect` in new fuzz harness code
- PASS: No `panic!` in new fuzz harness code (assertions are for test oracles, not production)
- PASS: No `todo!` or `unimplemented!` in new fuzz harness code
- PASS: No `dbg!` in new fuzz harness code
- PASS: No unchecked indexing (all array access uses `.get()` or bounds-checked operations)

## Scott Wlaschin DDD Check
- PASS: Error types are discriminated unions (enums) with typed variants
- PASS: Parse-don't-validate pattern: all boundary functions return `Result<T, E>`
- PASS: No boolean blindness: each error variant carries specific context

## Bitter Truth Simplicity Check
- PASS: Each harness is focused on a single boundary
- PASS: No over-engineering: harnesses follow existing fuzz pattern
- PASS: Binary targets are minimal stdin-driven wrappers

## Defects
None. All contract requirements are met.

STATUS: APPROVED
