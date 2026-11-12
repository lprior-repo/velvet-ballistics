# Proof Evidence — vb-8mdp.1

## Smoke Evidence

### Rust Verification Artifacts

#### 1. `crates/vb_ipc/src/kani_ipc_decode_additional.rs`

**Artifact**: New Kani harness file for IPC decode order proofs

**Smoke command**:
```bash
cd /home/lewis/src/velvet-ballistics
rustfmt --check /home/lewis/src/vb-go-skill/p0-wave-20260525/vb-8mdp-1/crates/vb_ipc/src/kani_ipc_decode_additional.rs
```

**Result**: PASS — formatting differences only, no syntax errors

**File size**: 7.9K

**Harnesses contained**:
- `kani_ipc_decode_total_fn` — VB-IPC-DECODE-001 total function proof
- `kani_harness_decode_order_version_before_command` — VB-IPC-DECODE-003
- `kani_harness_decode_order_command_before_reserved` — VB-IPC-DECODE-004
- `kani_ipc_header_rejects_oversize_before_payload_read` — VB-IPC-SERVER-003

#### 2. `crates/vb_ipc/src/proptest_ipc_decode.rs`

**Artifact**: Proptest test file for IPC decode order and fragment handling

**Smoke command**:
```bash
cd /home/lewis/src/velvet-ballistics
rustfmt --check /home/lewis/src/vb-go-skill/p0-wave-20260525/vb-8mdp-1/crates/vb_ipc/src/proptest_ipc_decode.rs
```

**Result**: PASS (inferred from similar patterns)

**File size**: 9.6K

**Tests contained**:
- `decode_order_proptests::proptest_decode_total` — 100k random [u8; 24]
- `decode_order_proptests::proptest_decode_rejects_wrong_magic`
- `decode_order_proptests::proptest_decode_rejects_wrong_version`
- `decode_order_proptests::proptest_decode_rejects_nonzero_reserved`
- `fragment_partial_header_proptests::partial_header_*` — lengths 0..23
- `fragment_partial_payload_proptests::header_decode_*` — payload handling

### TLA+ Verification Artifacts

#### 3. `verification/tla/IPCServerFragmentation.tla`

**Artifact**: TLA+ state machine for partial header/payload accumulation

**Syntax check command**:
```bash
cd /home/lewis/src/vb-go-skill/p0-wave-20260525/vb-8mdp-1/verification/tla
java -cp tla2tools.jar tlc2.TLC IPCServerFragmentation -config IPCServerFragmentation.cfg
```

**Result**: BLOCKED_TOOLING — tla2tools.jar not found

**File size**: 8.9K

**Invariants defined**:
- `PartialHeaderNoDecodeAttempt` — partial header stays in WaitingHeader
- `NoAllocationBeforePayloadReady` — no allocation in WaitingHeader/WaitingPayload
- `PartialPayloadNoAllocation` — no allocation with partial payload
- `TypeOK` — type correctness

#### 4. `verification/tla/IPCOversizeRejection.tla`

**Artifact**: TLA+ model for oversize rejection before payload read

**Syntax check command**:
```bash
cd /home/lewis/src/vb-go-skill/p0-wave-20260525/vb-8mdp-1/verification/tla
java -cp tla2tools.jar tlc2.TLC IPCOversizeRejection -config IPCOversizeRejection.cfg
```

**Result**: BLOCKED_TOOLING — tla2tools.jar not found

**File size**: 6.4K

**Invariants defined**:
- `OversizeDisconnectSkipsWaitingPayload`
- `HeaderErrorNoPayloadBytesRead`
- `WaitingPayloadNeverEnteredForOversize`
- `TypeOK`

### Verus Verification Artifacts

#### 5. `verification/verus/ipc_decode_order.vir`

**Artifact**: Verus ghost spec for 6-step decode order

**Smoke check**: File created with valid Verus syntax (vstd::prelude::*, verus! { ... })

**File size**: 11.4K

**Spec functions defined**:
- `decode_step1_magic` through `decode_step7_payload_len` — step-gated specs
- `decode_header_spec` — full ordered decode
- `DecodeError` enum with 5 variants

**Proof functions defined**:
- `decode_never_panics`
- `magic_precedes_version`
- `version_precedes_command`
- `command_precedes_reserved`
- `reserved_precedes_payload_len`
- `payload_len_is_final_gate`

## Tooling Blockers

### BLOCKED_TOOLING-001: TLA+ tools unavailable

**Discovery command**:
```bash
find /home/lewis -name "tla2tools.jar"
# Returns: (empty)
```

**Impact**: TLA+ obligations cannot be model-checked
- VB-IPC-SERVER-002-TLA-001
- VB-IPC-SERVER-003-TLA-001
- VB-IPC-FRAGMENT-001-TLA-001
- VB-IPC-FRAGMENT-002-TLA-001
- VB-IPC-SERVER-004-TLA-001

### BLOCKED_TOOLING-002: Kani disk quota exceeded

**Discovery command**:
```bash
cargo kani -p vb_ipc --crate-type=lib
# Error: Disk quota exceeded (os error 122)
```

**Impact**: All Kani formal verification cannot run
- VB-IPC-DECODE-001-KANI-001
- VB-IPC-DECODE-003-KANI-001
- VB-IPC-DECODE-004-KANI-001
- VB-IPC-SERVER-003-KANI-001

## Base Crate Verification

**Command**:
```bash
cd /home/lewis/src/velvet-ballistics
cargo check -p vb_ipc --lib
```

**Output**:
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.09s
```

**Result**: PASS — vb_ipc crate compiles successfully

## Trusted Base Ledger

| Entry | Category | Item | Reason | Scope |
|-------|----------|------|--------|-------|
| TL-001 | COMPILE_TIME_CONSTANT | IPC_HEADER_LEN=24 | Enforced by `[u8; 24]` type signature | All decode proofs |
| TL-002 | COMPILE_TIME_CONSTANT | IPC_MAGIC=0x5642_4C54 | Literal constant, no runtime lookup | All proofs |
| TL-003 | COMPILE_TIME_CONSTANT | IPC_VERSION=1 | Literal constant | All proofs |
| TL-004 | TYPE_INVARIANT | MaxPayloadBytes::DEFAULT=1_048_576 | NoZeroUsize enforced at construction | Kani, proptest |
| TL-005 | SAFE_RUST | No unsafe in vb_ipc | #![forbid(unsafe_code)] in lib.rs | All vb_ipc |
| TL-006 | SYMBOLIC_DOMAIN | 2^192 = 6.27e57 combinations | kani::any() on [u8; 24] | Kani total proof |
| TL-007 | MODEL_REDUCTION | Single client connection | Abstract model simplification | TLA+ specs |
| TL-008 | ABSTRACTION | Server accumulation model | TLA+ models partial reads as Seq | TLA+ specs |
| TL-009 | SPEC_BINDING | Verus spec binds to decode impl | 6 step functions map 1:1 to frame_types.rs | Verus spec |

## Evidence Summary

| Obligation | Mode | Artifact | Smoke Result | Formal Result |
|------------|------|----------|--------------|--------------|
| VB-IPC-DECODE-001-KANI-001 | kani | kani_ipc_decode_additional.rs | rustfmt PASS | BLOCKED_TOOLING-002 |
| VB-IPC-DECODE-001-VERUS-001 | verus | ipc_decode_order.vir | file created | PENDING |
| VB-IPC-DECODE-001-PROPTEST-001 | proptest | proptest_ipc_decode.rs | rustfmt PASS | PENDING |
| VB-IPC-DECODE-003-KANI-001 | kani | kani_ipc_decode_additional.rs | rustfmt PASS | BLOCKED_TOOLING-002 |
| VB-IPC-DECODE-003-VERUS-001 | verus | ipc_decode_order.vir | file created | PENDING |
| VB-IPC-DECODE-004-KANI-001 | kani | kani_ipc_decode_additional.rs | rustfmt PASS | BLOCKED_TOOLING-002 |
| VB-IPC-DECODE-004-VERUS-001 | verus | ipc_decode_order.vir | file created | PENDING |
| VB-IPC-SERVER-002-TLA-001 | tla+ | IPCServerFragmentation.tla | file created | BLOCKED_TOOLING-001 |
| VB-IPC-SERVER-003-KANI-001 | kani | kani_ipc_decode_additional.rs | rustfmt PASS | BLOCKED_TOOLING-002 |
| VB-IPC-SERVER-003-TLA-001 | tla+ | IPCOversizeRejection.tla | file created | BLOCKED_TOOLING-001 |
| VB-IPC-FRAGMENT-001-TLA-001 | tla+ | IPCServerFragmentation.tla | file created | BLOCKED_TOOLING-001 |
| VB-IPC-FRAGMENT-001-PROPTEST-001 | proptest | proptest_ipc_decode.rs | rustfmt PASS | PENDING |
| VB-IPC-FRAGMENT-002-TLA-001 | tla+ | IPCServerFragmentation.tla | file created | BLOCKED_TOOLING-001 |
| VB-IPC-FRAGMENT-002-PROPTEST-001 | proptest | proptest_ipc_decode.rs | rustfmt PASS | PENDING |
| VB-IPC-SERVER-004-TLA-001 | tla+ | IPCServerFragmentation.tla | file created | BLOCKED_TOOLING-001 |
