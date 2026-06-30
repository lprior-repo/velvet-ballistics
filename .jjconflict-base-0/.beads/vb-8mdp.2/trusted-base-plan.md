# Trusted Base Plan: vb-8mdp.2 Budget-Before-Decode

## Trusted Surfaces

### 1. Rust Type System (Trusted)
- `decode_record_header(header: &[u8], ...)` — the `&[u8]` signature guarantees no allocation inside the function
- `#[forbid(unsafe_code)]` in vb_storage codec — no unsafe blocks that could bypass borrowing rules
- **Trusted because**: Rust's borrow checker is verified by the Rust compiler; we trust the compiler's borrow checker

### 2. Fjall Keyspace API (Trusted External)
- `fjall::Keyspace::get(key) -> Option<&[u8]>` — returns borrowed bytes, no allocation
- **Trusted because**: Fjall is a well-tested external crate; the API contract is that get() returns borrowed data
- **Risk**: If Fjall changes behavior and starts returning owned data, decode_optional could allocate before budget gate
- **Mitigation**: Kani proof traces decode_optional path and proves no allocation before budget gate

### 3. CRC32C Implementation (Trusted External)
- `crc32c::crc32c(&[u8]) -> u32` — external crate, assumed correct
- **Trusted because**: crc32c is a well-known standard; the crate is widely used
- **Risk**: None — CRC computation has no allocation risk

### 4. BLAKE3 Implementation (Trusted External)
- `blake3::hash(&[u8]) -> Digest` — external crate, assumed correct
- **Trusted because**: blake3 is a well-tested implementation
- **Risk**: None — hashing has no allocation risk

### 5. Postcard Deserialization (Trusted External)
- `postcard::from_bytes<T>(&[u8]) -> Result<T, _>` — external crate
- **Trusted because**: postcard is the standard serde format for embedded systems
- **Risk**: Postcard may internally allocate for Vec fields in T
- **Mitigation**: Bounded slice (payload_len <= max) is passed to postcard; postcard can only allocate up to slice size

### 6. Rust Standard Library (Trusted)
- `Option<T>::ok_or()`, `Result<T, E>::?` operator, slice indexing
- **Trusted because**: Rust standard library is verified by extensive testing and miri on core

## Assumptions

### A1: Fjall get() Returns Borrowed Data
```
keyspace.get(key) -> Option<&[u8]>
```
- **Assumption**: Fjall always returns borrowed data from get(), never allocating
- **Evidence**: Fjall API documentation; no owned Vec in return type
- **Risk if violated**: decode_optional could allocate before budget gate
- **Proof dependency**: PO-007 (Kani decode_optional path)

### A2: No Unsafe Code in Codec Path
```
#![forbid(unsafe_code)] in vb_storage/src/codec/
```
- **Assumption**: No unsafe blocks in decode_record_header, decode_record_payload, decode_record
- **Evidence**: Code review; #![forbid(unsafe_code)] enforced at compile time
- **Risk if violated**: Unsafe could bypass borrow rules
- **Proof dependency**: Type system enforcement

### A3: checked_add Correctly Handles Overflow
```
payload_end = payload_start.checked_add(payload_len_usize)
```
- **Assumption**: checked_add returns None on overflow, not panicking
- **Evidence**: Rust standard library behavior is well-tested
- **Risk if violated**: Overflow could cause wraparound, leading to incorrect slice bounds
- **Proof dependency**: PO-005 (Kani overflow check)

### A4: RecordKind Enum is Exhaustively Defined
```
RecordKind enum has known variants: WorkflowSource=1, CompiledIr=2, RunHeader=3, Snapshot=30, Blob=40, ...
```
- **Assumption**: All record_kind values are covered in the enum; unknown values are caught by validate_known_kind
- **Evidence**: RecordKind enum definition; validate_known_kind handles unknown variants
- **Risk if violated**: Unknown record_kind could bypass validation
- **Proof dependency**: PO-009 (Kani unknown kind)

### A5: MAX_*_BYTES Constants are Type-Specific Bounds
```
MAX_SNAPSHOT_BYTES = 67108864 (64 MiB)
MAX_BLOB_BYTES = 67108864 (64 MiB)
MAX_JOURNAL_EVENT_PAYLOAD_BYTES = 1048576 (1 MiB)
```
- **Assumption**: These constants are enforced at encode time (payload_len_u32) and decode time (budget gate)
- **Evidence**: payload_len_u32 at encode; budget gate at decode
- **Risk if violated**: Corrupt write could set payload_len > max without detection
- **Proof dependency**: TLA+ budget workflow invariant (PO-020)

## Model Reductions

### R1: TLA+ Model Reduces Payload Length Domain
- **Reduction**: TLA+ model uses 32-bit integers for payload_len but bounds model checking to representative values
- **Justification**: Exhaustively checking u32::MAX values is infeasible; representative boundary values (0, max-1, max, max+1, large) are sufficient to prove the budget invariant
- **Evidence**: TLC covers boundary conditions; Kani covers arbitrary u32 values

### R2: Kani Unwind Bound
- **Reduction**: Kani uses unwind bounds for loops (if any); may not explore all execution paths
- **Justification**: decode_record_header has no loops; all paths are direct branches; unwind bound = 1 sufficient
- **Evidence**: Code has no for/while loops

### R3: Verus Proof Reduces to Single Function
- **Reduction**: Verus spec fn models only decode_record_header, not the full call chain
- **Justification**: decode_record_header is the budget gate; other functions are orthogonal
- **Evidence**: decode_record_header is the single point of budget enforcement

## Known Gaps

### G1: Fjall KV Separation Interaction
- **Gap**: KeyspaceProfile::Cold and Blob enable KV separation (value stored separately from key)
- **Impact**: Fjall retrieves value separately; could theoretically allocate beyond max during retrieval
- **Mitigation**: Budget gate is applied to the RETRIEVED value bytes; Fjall retrieval doesn't pre-allocate based on header
- **Risk Level**: Low — Fjall retrieves exact bytes; no pre-allocation based on declared length

### G2: Postcard Internal Allocation
- **Gap**: Postcard may internally allocate for Vec<T> fields inside deserialized struct
- **Impact**: Actual memory usage could exceed payload_len for complex types
- **Mitigation**: Slice passed to postcard is bounded to payload_len bytes; max_payload_len is the per-type constant
- **Risk Level**: Medium — Acceptable for now; postcard operates within bounded slice

## Non-Negotiable Constraints

1. **No pre-budget Vec allocation in decode_record_header** — enforced by Rust type system + Kani proof
2. **Budget gate at line 48 must execute before any payload access** — enforced by code order + Kani proof
3. **All 9 keyspace prefixes must remain pairwise distinct** — enforced by TLA+ model
4. **No unsafe code in codec path** — enforced by #![forbid(unsafe_code)]

## Waiver Candidates

### W1: Postcard Internal Over-Allocation
- **Claim**: Postcard may internally allocate more than payload_len bytes for complex types
- **Impact**: Not a security issue since slice is bounded; may cause memory pressure for large deserialized types
- **Compensating evidence**: max_payload_len is per-type constant limiting maximum slice size
- **Expiry**: None — known acceptable behavior
- **Status**: Not a waiver; documented as known gap G2