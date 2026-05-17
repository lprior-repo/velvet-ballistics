# Theorem Kernel Projection

## Boundary

### TLA+-owned Temporal Model
- **None** - No temporal model applies (see tla-spec.md)

### Verus-owned Rust Core
All pure Rust-local proof obligations are owned by Verus:
- `encode_yaml`: YAML serialization invariant
- `encode_postcard`: Binary header construction invariants
- `decode_postcard`: Binary header validation invariants
- `validate_no_ansi`: ANSI escape sequence detection
- `PostcardHeader::validate`: Bounded allocation enforcement

### Theorem-owned Kernel
- **None** - Verus covers all Rust-local pure critical behavior
- No algebraic state transitions requiring Lean extraction
- No protocol lattices beyond Verus expressibility
- No arithmetic bounds requiring proof assistant extraction

### Rust/Runtime Shell
- I/O: stdout/stderr writes are shell behavior, not kernel
- Serialization: serde_yaml/postcard are external codec libraries
- No async scheduling, no networking, no filesystem in emit path

### External Systems Excluded from Theorem Proof
- BLAKE3 and CRC32C libraries (treated as trusted external implementations)
- serde_yaml (treated as trusted serialization framework)
- postcard (treated as trusted encoding framework)

## Theorem-Owned Clauses
- **None** - Verus is sufficient for all Rust-local pure invariants

## Theorem Obligations
- **None** - No theorem kernel required

## Waivers
- **WAIVER-KERNEL-001**: BLAKE3 digest computation is infallible for bounded payload sizes. blake3::Hasher::finalize has no error path. Compensating evidence: existing unit tests cover digest computation paths.
- **WAIVER-KERNEL-002**: CRC32C computation via crc32c::crc32c is infallible for bounded input. Function takes byte slice, returns u32 directly with no error path. Compensating evidence: existing unit tests cover CRC computation paths.
- **WAIVER-KERNEL-003**: YAML serialization via serde_yaml::to_string cannot fail for OutputEnvelope types which use Serialize derive with primitive fields only. Compensating evidence: PROP-004 (YAML round-trip test) exercises serialization without error.
