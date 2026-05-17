# Verification Layers

## Boundary

### Verus-owned Kernel
All emitter pure functions in vb_ui_model/src/emitter.rs:
- `encode_yaml` - YAML text serialization
- `encode_postcard` - Binary envelope construction
- `decode_postcard` - Binary envelope parsing
- `validate_no_ansi` - ANSI escape detection
- Header construction/validation functions

### TLA+ Temporal Model
- **None** - Deterministic codec transformation, no temporal behavior

### Theorem Projection
- **None** - Verus sufficient for all Rust-local pure invariants

### Runtime Shell
- stdout/stderr write operations
- CLI argument parsing (args.rs)
- Command dispatch (main.rs)

### External Systems Excluded from Formal Proof
- BLAKE3 library (trusted implementation)
- CRC32C library (trusted implementation)
- serde_yaml (trusted serialization)
- postcard (trusted encoding)

## Layer Assignment

| Contract Clause | Verification Layers |
|----------------|---------------------|
| PRE-EMIT-001 | proptest + cargo-fuzz |
| PRE-EMIT-002 | kani + proptest |
| PRE-EMIT-003 | kani + proptest |
| PRE-EMIT-004 | proptest + kani |
| PRE-EMIT-005 | unit tests |
| POST-EMIT-001 | snapshot tests + YAML roundtrip proptest |
| POST-EMIT-002 | snapshot tests + postcard roundtrip proptest |
| POST-EMIT-003 | kani + unit tests |
| POST-EMIT-004 | kani + unit tests |
| POST-EMIT-005 | kani + unit tests |
| POST-EMIT-006 | kani + unit tests |
| POST-EMIT-007 | proptest + kani |
| POST-EMIT-008 | snapshot tests |
| INV-EMIT-001 | kani + unit tests |
| INV-EMIT-002 | kani + unit tests |
| INV-EMIT-003 | kani + unit tests |
| INV-EMIT-004 | kani + unit tests |
| INV-EMIT-005 | kani + unit tests |
| INV-EMIT-006 | waiver (BLAKE3 infallible) |
| INV-EMIT-007 | waiver (CRC32C infallible) |

## Verus Scope
- **Not applicable** - No Verus proof obligations currently defined for emitter.rs
- Existing proof obligations in proof-obligations.jsonl use Kani and proptest

## TLA+ Scope
- **Not applicable** - No temporal behavior

## Theorem Scope
- **Not applicable** - Verus covers all pure invariants

## Snapshot Test Requirements
- YAML output snapshot tests for status command
- Postcard output snapshot tests for status command
- Text output snapshot tests for status command
- Format validation against expected schema_version, kind fields

## Waivers
| Waiver ID | Clause | Reason | Compensating Evidence |
|-----------|--------|--------|------------------------|
| WAIVER-EMIT-002 | INV-EMIT-006 | BLAKE3 digest computation infallible | Unit tests cover digest paths, COV-001 >90% |
| WAIVER-EMIT-003 | INV-EMIT-007 | CRC32C computation infallible | Unit tests cover CRC paths, COV-001 >90% |
| WAIVER-EMIT-004 | ERR-YamlEncodeFailed | serde_yaml::to_string infallible for OutputEnvelope types | PROP-004 YAML roundtrip, COV-001 >90% |
