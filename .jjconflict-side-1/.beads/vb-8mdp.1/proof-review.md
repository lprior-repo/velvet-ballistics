# Proof Review — vb-8mdp.1

**Bead**: vb-8mdp.1 — Add IPC fragmented-frame and oversize-message tests
**Reviewer**: proof-reviewer
**Reviewer Skill**: proof-reviewer
**Reviewer Invocation**: proof-reviewer:vb-8mdp.1:20260525
**Isolated Workspace**: /home/lewis/src/vb-go-skill/p0-wave-20260525/vb-8mdp-1
**Artifacts Dir**: /home/lewis/src/velvet-ballistics/.beads/vb-8mdp.1

---

## Provenance

- proof-writer-report.md: reviewed
- proof-evidence.md: reviewed
- verification/tla/IPCServerFragmentation.tla: reviewed
- verification/tla/IPCOversizeRejection.tla: reviewed
- verification/verus/ipc_decode_order.vir: reviewed
- crates/vb_ipc/src/kani_ipc_decode_additional.rs: reviewed
- crates/vb_ipc/src/proptest_ipc_decode.rs: reviewed
- trusted-base-ledger.jsonl: reviewed (12 entries)
- proof-plan-review.md: APPROVED (prior review)

---

## Tooling Status

| Tool | Status | Evidence |
|------|--------|----------|
| TLA+ (IPCServerFragmentation.tla) | BLOCKED_TOOLING | tla2tools.jar not found |
| TLA+ (IPCOversizeRejection.tla) | BLOCKED_TOOLING | tla2tools.jar not found |
| Kani (all harnesses) | BLOCKED_TOOLING | Disk quota exceeded |
| Verus (ipc_decode_order.vir) | PENDING_FORMAL_EXECUTION | No cargo verify run evidence |
| Proptest | SMOKE_ONLY | cargo test -p vb_ipc: 692 tests PASS |
| Rustfmt | PASS | formatting check on all artifacts |
| cargo check | PASS | vb_ipc compiles successfully |

---

## Lethal Findings

### Finding: VACUOUS_VERUS_PROOF-001
**Severity**: LETHAL
**Artifact**: `verification/verus/ipc_decode_order.vir`, lines 190-199
**Obligation**: VB-IPC-DECODE-001-VERUS-001
**Evidence**:
```verus
proof fn decode_never_panics(bytes: &[u8], max_payload: usize)
    requires
        bytes.len() >= 24,
        max_payload > 0,
    ensures
        decode_header_spec(bytes, max_payload).is_ok()
        || decode_header_spec(bytes, max_payload).is_err(),
{
    // decode_header_spec is purely arithmetic — no panics possible
    // All operations are bitwise OR and comparison
}
```
**Problem**: This proof is a **logical tautology**. `P.is_ok() || P.is_err()` is always true for any `Result<P, Q>` by the nature of Result. This proves nothing about whether `decode` panics.

**Required Fix**: The proof must show that `decode_header_spec` (or the actual `IpcFrameHeader::decode`) cannot panic. The ensures clause must assert something meaningful like `compatible(decode_header_spec)` or `no_panic_path(decode)`. The existing comment "no panics possible" is an assertion, not a proof.

**Obligation ID**: VB-IPC-DECODE-001-VERUS-001

---

### Finding: PROPTEST_SCOPE_MISMATCH-001
**Severity**: LETHAL
**Artifact**: `crates/vb_ipc/src/proptest_ipc_decode.rs`, lines 115-157
**Obligation**: VB-IPC-FRAGMENT-001-PROPTEST-001
**Evidence**: The test module is named `fragment_partial_header_proptests` and claims to test "partial header (0..23 bytes), no decode error returned." However:
- Line 128-134: Tests with 0 bytes by creating `[0u8; 24]` — this is a FULL header of zeros, not a partial header
- Line 136-156: Tests lengths 1..23 but passes full 24-byte arrays, not slices of length `len`

**Problem**: `IpcFrameHeader::decode` takes `&[u8; 24]`. To test partial headers, the test must pass a slice/array of length `len` where `len < 24`. The current tests pass full 24-byte arrays regardless of `len`.

**Required Fix**: For `partial_header_0_bytes_no_decode_attempt`, use a 0-byte slice (not 24-byte array of zeros). For `partial_header_1_to_23_bytes_decode_returns_error`, pass an array of length `len`, not 24.

**Obligation ID**: VB-IPC-FRAGMENT-001-PROPTEST-001

---

### Finding: PROPTEST_SCOPE_MISMATCH-002
**Severity**: LETHAL
**Artifact**: `crates/vb_ipc/src/proptest_ipc_decode.rs`, lines 159-211
**Obligation**: VB-IPC-FRAGMENT-002-PROPTEST-001
**Evidence**: The module `fragment_partial_payload_proptests` claims to test "valid header + partial payload, no allocation." However:
- Line 171-179: Tests zero payload decode (valid header, no payload)
- Line 181-195: Tests that `decode` doesn't read payload bytes (header-only test)
- Line 197-210: Tests oversize rejection at decode time

**Problem**: None of these tests actually test the server-side allocation behavior. The TLA+ spec `NoAllocationBeforePayloadReady` proves no allocation in `WaitingPayload` state. The proptest tests only verify `IpcFrameHeader::decode` behavior, not server accumulation and allocation.

**Required Fix**: Either rename the test module to reflect what it actually tests (header field validation), or add actual server-side tests with mock socket/buffer that verify no allocation occurs with partial payload.

**Obligation ID**: VB-IPC-FRAGMENT-002-PROPTEST-001

---

## Non-Lethal Findings

### Finding: TLA_INVARIANT_WEAK-001
**Severity**: ADVISORY
**Artifact**: `verification/tla/IPCServerFragmentation.tla`, lines 162-165
**Evidence**:
```tla
DispatchOnlyInDispatching ==
    \A c \in CLIENTS:
        dispatch_count[c] > 0
            => \E prior \in Nat: prior < dispatch_count[c]
```
**Problem**: This invariant is **trivially true** for any incrementing counter. It provides no meaningful safety guarantee. Notably, this invariant is NOT listed in the CFG file's INVARIANTS section, so TLC never checks it anyway.

**Required Fix**: Remove or replace with a meaningful invariant like "dispatch only occurs in Dispatching state" or "every state transition to Disconnected has a prior Dispatching state."

---

### Finding: KANI_UNWIND_UNJUSTIFIED-001
**Severity**: ADVISORY
**Artifact**: `crates/vb_ipc/src/kani_ipc_decode_additional.rs`, line 25
**Evidence**: `#[kani::unwind(6)]` on `kani_ipc_decode_total_fn`
**Problem**: No justification for unwinding to 6. The decode function has 7 gate checks. Unwind should be at least 7 or the choice of 6 must be justified.

---

### Finding: KANI_ASSUME_MISSING-001
**Severity**: ADVISORY
**Artifact**: `crates/vb_ipc/src/kani_ipc_decode_additional.rs`, lines 44-87
**Evidence**: `kani_harness_decode_order_version_before_command` does not use `kani::assume` to constrain bytes[0..4] to valid magic
**Problem**: The harness relies on `kani::any()` generating all byte combinations including invalid magic. When magic is invalid, the decode returns `InvalidMagic` immediately, which means the version-before-command ordering isn't actually tested for that path.

**Impact**: Low — the harness still works correctly, it just means some generated inputs short-circuit before reaching the version check.

---

### Finding: VERUS_SPEC_BINDING_UNVERIFIED-001
**Severity**: ADVISORY
**Artifact**: `verification/verus/ipc_decode_order.vir`
**Evidence**: TL-009 in trusted-base-ledger.jsonl: "manual_review_required"
**Problem**: The Verus spec functions (`le32_to_cpu`, `le16_to_cpu`, etc.) are defined mathematically in the spec but not formally verified to match the actual `byteorder` reads in `frame_types.rs`. TL-009 correctly defers to manual review, but no manual review evidence is provided in the artifact.

**Impact**: The spec is likely correct (mathematical bitwise ops match byteorder), but this hasn't been formally established.

---

## Trust Ledger Review

| Entry | Category | Assessment |
|-------|----------|------------|
| TL-001 | COMPILE_TIME_CONSTRAINT | VALID — `[u8; 24]` type enforces 24 bytes |
| TL-002 | COMPILE_TIME_CONSTANT | VALID — IPC_MAGIC literal |
| TL-003 | COMPILE_TIME_CONSTANT | VALID — IPC_VERSION literal |
| TL-004 | TYPE_INVARIANT | VALID — NoZeroUsize enforced |
| TL-005 | SAFE_RUST | VALID — #![forbid(unsafe_code)] confirmed |
| TL-006 | SYMBOLIC_EXECUTION_BOUND | VALID — kani::any() on [u8; 24] |
| TL-007 | MODEL_REDUCTION | VALID — single client is intentional simplification |
| TL-008 | ABSTRACTION | VALID — Seq models accumulation |
| TL-009 | SPEC_BINDING | DEFECT — manual review required but no evidence |
| TL-010 | ASSUMPTION | VALID — READ_CHUNK_BYTES abstraction reasonable |
| TL-011 | PURE_FUNCTION | VALID — decode is pure |
| TL-012 | ASSUMPTION | VALID — byteorder reads return Result |

---

## Summary Assessment

### BLOCKED_TOOLING (Acceptable with Evidence)

| Obligation | Tool | Blocker | Evidence Available |
|------------|------|---------|-------------------|
| VB-IPC-SERVER-002-TLA-001 | TLA+ | tla2tools.jar missing | TLA+ spec well-formed, TypeOK+invariants defined |
| VB-IPC-SERVER-003-TLA-001 | TLA+ | tla2tools.jar missing | TLA+ spec well-formed, TypeOK+invariants defined |
| VB-IPC-FRAGMENT-001-TLA-001 | TLA+ | tla2tools.jar missing | TLA+ spec well-formed, TypeOK+invariants defined |
| VB-IPC-FRAGMENT-002-TLA-001 | TLA+ | tla2tools.jar missing | TLA+ spec well-formed, TypeOK+invariants defined |
| VB-IPC-SERVER-004-TLA-001 | TLA+ | tla2tools.jar missing | TLA+ spec well-formed, TypeOK+invariants defined |
| VB-IPC-DECODE-001-KANI-001 | Kani | Disk quota exceeded | Harness uses kani::any(), structurally sound |
| VB-IPC-DECODE-003-KANI-001 | Kani | Disk quota exceeded | Harness structurally sound |
| VB-IPC-DECODE-004-KANI-001 | Kani | Disk quota exceeded | Harness structurally sound |
| VB-IPC-SERVER-003-KANI-001 | Kani | Disk quota exceeded | Harness structurally sound |

### Lethal Findings Block Approval

1. **VACUOUS_VERUS_PROOF-001**: `decode_never_panics` is a tautology — must be repaired
2. **PROPTEST_SCOPE_MISMATCH-001**: `fragment_partial_header_proptests` doesn't test partial headers — must be repaired
3. **PROPTEST_SCOPE_MISMATCH-002**: `fragment_partial_payload_proptests` doesn't test allocation — must be repaired

---

## Verdict

**STATUS: REJECTED**

**Reasons**:
1. Vacuous Verus proof (LETHAL) — `decode_never_panics` proves nothing
2. Proptest tests don't match their stated obligations (LETHAL)
3. TLA+ specs well-formed but not model-checked (BLOCKED_TOOLING acceptable)
4. Kani harnesses structurally sound but not executed (BLOCKED_TOOLING acceptable)

**BLOCKED_TOOLING exceptions** are granted for TLA+ (tooling unavailable) and Kani (disk quota). However, the **vacuous proof** and **scope mismatches** are proof defects that require repair.

---

**Reviewer**: proof-reviewer
**Invocation ID**: proof-reviewer:vb-8mdp.1:20260525
**Timestamp**: 2026-05-25
**Review Artifacts**: proof-review.md, proof-findings.jsonl
**STATUS: REJECTED**