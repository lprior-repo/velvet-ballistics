# Proof Coverage Matrix — vb-8mdp.1

Maps each contract clause (from `contract.md`) to proof obligations and verifier lanes.

## Contract: `IpcFrameHeader::decode`

### P1 — Magic First (VB-IPC-DECODE-002)

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|-----------------|-----------------|---------------|--------|
| P1: InvalidMagic returned before UnsupportedVersion | VB-IPC-DECODE-002 | Kani (kani_harness_ipc_decode_order) | existing |
| P1: InvalidMagic for any bytes[0..4] != IPC_MAGIC | VB-IPC-DECODE-001 | Kani (new harness) + Verus | partial |

### P2 — Version After Magic (VB-IPC-DECODE-003)

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|-----------------|-----------------|---------------|--------|
| P2: UnsupportedVersion when magic==IPC_MAGIC && version!=IPC_VERSION | VB-IPC-DECODE-003 | Kani (kani_harness_ipc_magic_before_version) | existing |
| P2: version!=1 rejected regardless of command validity | VB-IPC-VERSION-001 | Kani + proptest | existing |
| P2: version checked before command extraction | VB-IPC-DECODE-003 | Verus spec fn | new |

### P3 — Command After Version (VB-IPC-DECODE-004)

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|-----------------|-----------------|---------------|--------|
| P3: UnknownCommand only when magic+version valid | VB-IPC-DECODE-004 | Kani (new harness) + Verus | partial |
| P3: command ∈ {1..16} enforced by from_u16 | VB-IPC-COMMAND-001 | Kani + proptest | existing |
| P3: as_u16/from_u16 inverses for 1..16 | VB-IPC-COMMAND-002 | Kani + proptest | existing |

### P4 — Reserved Before Payload (VB-IPC-DECODE-006)

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|-----------------|-----------------|---------------|--------|
| P4: ReservedNonZero returned before PayloadTooLarge | VB-IPC-DECODE-006 | Kani (kani_harness_ipc_reserved_nonzero_before_payload_len) | existing |
| P4: ReservedNonZero only when steps 1-3 pass | VB-IPC-DECODE-004 | Kani (new harness) + Verus | partial |
| P4: reserved field non-zero → protocol error | VB-IPC-DECODE-004 | Kani + proptest | partial |

### P5 — PayloadLen Bound After Structural Checks (VB-IPC-DECODE-005)

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|-----------------|-----------------|---------------|--------|
| P5: PayloadTooLarge only when steps 1-5 pass | VB-IPC-DECODE-005 | Kani (kani_ipc_header_rejects_oversize_payload) | existing |
| P5: u32→usize overflow caught before bounds check | VB-IPC-DECODE-007 | Kani | existing |
| P5: PayloadTooLarge includes actual and limit | VB-IPC-DECODE-005 | Kani (existing harness checks) | existing |

### P6 — Ok Result Contains All Fields

| Contract Clause | Proof Obligation | Verifier Lane | Status |
|-----------------|-----------------|---------------|--------|
| P6: Ok result field preservation | VB-IPC-DECODE-001 | Kani (kani_ipc_header_preserves_all_fields) | existing |
| P6: encode→decode roundtrip | VB-IPC-POSTCARD-002 | Kani + proptest | existing |

## Contract: Decode Order Theorem

| Sub-claim | Proof Obligation | Verifier Lane | Status |
|-----------|-----------------|---------------|--------|
| STEP 1: magic checked first | VB-IPC-DECODE-002 | Kani + TLA+ | partial |
| STEP 2: version checked second | VB-IPC-DECODE-003 | Kani + Verus | partial |
| STEP 3: command checked third | VB-IPC-DECODE-004 | Kani + Verus | partial |
| STEP 4: reserved checked fourth | VB-IPC-DECODE-006 | Kani | existing |
| STEP 5: correlation read (no failure) | N/A (u64 always valid) | N/A | N/A |
| STEP 6: payload_len checked last | VB-IPC-DECODE-005 | Kani | existing |
| Corollary: error priority ordering | VB-IPC-DECODE-002/003/004/006 | Kani | partial |

## Contract: Partial Frame Server Behavior

| Invariant | Proof Obligation | Verifier Lane | Status |
|-----------|-----------------|---------------|--------|
| Read buffer bounded by READ_CHUNK_BYTES per poll | VB-IPC-SERVER-001 | TLA+ | new |
| No pre-allocation (payload_len not used for Vec::with_capacity before decode) | VB-IPC-SERVER-002 | TLA+ | new |
| Frame complete before dispatch | VB-IPC-SERVER-004 | TLA+ | new |
| Partial header: no error, wait | VB-IPC-FRAGMENT-001 | TLA+ + proptest | new |
| Partial payload: no allocation, wait | VB-IPC-FRAGMENT-002 | TLA+ + proptest | new |

## Contract: Oversize Payload Rejection

| Invariant | Proof Obligation | Verifier Lane | Status |
|-----------|-----------------|---------------|--------|
| Header-only oversize rejection (no payload bytes read) | VB-IPC-SERVER-003 | Kani + TLA+ | new |
| Server disconnects on PayloadTooLarge | VB-IPC-SERVER-003 | TLA+ | new |
| decode returns PayloadTooLarge before any allocation | VB-IPC-DECODE-005 | Kani | existing |

## Contract: Frame Decode Order (Railway)

| Railway Step | Proof Obligation | Verifier Lane | Status |
|-------------|-----------------|---------------|--------|
| read_buffer.len() >= 24? | VB-IPC-FRAGMENT-001 | TLA+ + proptest | new |
| decode header (total fn) | VB-IPC-DECODE-001 | Kani + Verus | partial |
| header valid? → check payload_len | VB-IPC-DECODE-005 | Kani | existing |
| payload_len <= max? | VB-IPC-DECODE-005 | Kani | existing |
| frame_total_len <= buf? | VB-IPC-SERVER-004 | TLA+ | new |
| extract payload, dispatch | VB-IPC-SERVER-004 | TLA+ | new |

## Protocol Constants

| Constant | Proof Obligation | Verifier Lane | Status |
|----------|-----------------|---------------|--------|
| IPC_HEADER_LEN == 24 | VB-IPC-FRAME-001 | Kani (compile-time assert) | existing |
| IPC_MAGIC == 0x5642_4C54 | VB-IPC-MAGIC-001 | Kani | existing |
| IPC_MAGIC != 0 and != 0xFFFF_FFFF | VB-IPC-MAGIC-002 | Kani | existing |
| IPC_VERSION == 1 | VB-IPC-MAGIC-003 | Kani | existing |
| MaxPayloadBytes::DEFAULT == 1_048_576 | VB-IPC-BOUNDED-002 | Kani | existing |

## Coverage Summary

| Category | Total Proof Seeds | Kani | Verus | TLA+ | Proptest |
|----------|------------------|------|-------|------|----------|
| Decode order (VB-IPC-DECODE-*) | 7 | 7 ✅ | 3 ✅ | 0 | 0 |
| Server behavior (VB-IPC-SERVER-*) | 4 | 1 ✅ | 0 | 4 ✅ | 0 |
| Fragmented frame (VB-IPC-FRAGMENT-*) | 2 | 0 | 0 | 2 ✅ | 2 ✅ |
| Oversize rejection (VB-IPC-SERVER-003) | 1 | 1 ✅ | 0 | 1 ✅ | 0 |
| Constants/magic/version | 6 | 6 ✅ | 0 | 0 | 0 |
| Encode/decode roundtrip | 3 | 3 ✅ | 0 | 0 | 2 ✅ |

**Legend**: ✅ = covered (existing or new planned), partial = partially covered, blank = not applicable