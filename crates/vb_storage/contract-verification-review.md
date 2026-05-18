# Contract Review: vb_storage

**Agent:** 84 / Round 9
**Scope:** vb_storage/src/
**Focus:** Journal event ordering · Key encoding parse-not-validate · Codec magic+CRC before parse · Recovery replay divergence

---

## Temporal Model Boundary

| Layer | Owned behavior |
|-------|----------------|
| TLA+ | Journal event sequence ordering, attempt monotonicity, snapshot+tail alignment |
| Verus | Codec header decode pure logic, sequence arithmetic, key encoding invariants |
| Kani | Header bounds, magic/CRC rejection before allocation |
| Fuzz/proptest | Codec byte streams, journal event round-trips |

---

## 1. Journal Event Ordering — Temporal Invariant

### Contract clauses

- **SEQ-INV-001**: Events for a given `RunId` must appear in strictly monotonically increasing `EventSeq` order during replay (`journal/replay.rs:44-46`)
- **STEP-INV-001**: Within a given attempt, `StepStarted` step indices must be non-decreasing (`recovery/replay/core.rs:66-79`)
- **ATTEMPT-INV-001**: Only the latest attempt number's state-affecting events contribute to recovery state; older attempts are filtered (`recovery/replay/core.rs:39-49`)
- **SNAP-INV-001**: `tail_events` in snapshot+tail replay must all have `seq > snapshot_seq` (`recovery/replay/core.rs:176-187`)

### TLA+ coverage

| Spec | Covers | Gap |
|------|--------|-----|
| `specs/tla/RecoveryReplay.tla` | Non-idempotent action re-execution prevention | No step ordering; action-only |
| `specs/tla/AttemptTracking.tla` | Stale completion rejection via `latest_attempt` map | Finite bound `Len(journal) <= 3`; does not cover snapshot+tail |
| — | SEQ-INV-001 (contiguous seq validation) | **MISSING TLA+** — only tested inline in `events_for_run_from` loop |
| — | STEP-INV-001 (non-decreasing step) | **MISSING TLA+** — only implemented in `replay_events` |
| — | SNAP-INV-001 (tail > snapshot seq) | **MISSING TLA+** — only tested in `recover_snapshot_plus_tail` |

### GAP-SEQ-001: Sequence validation not atomic
`events_for_run_from` (`journal/replay.rs:29-51`) validates sequence continuity incrementally per event inside the Fjall prefix iteration loop. The validation is:

```rust
let expected_seq = expected.unwrap_or_else(|| event.seq());
crate::codec::validate_replayed_event(run, expected_seq, &event)?;
expected = Some(crate::codec::next_seq(expected_seq)?);
```

There is no atomic post-condition that all returned events have contiguous seq from start to end. If the Fjall snapshot yields events out of order (Fjall prefix order should match byte-key order which IS seq order, but this is assumed not proven), the loop would catch it — but the assumption is not documented as a contract.

**Recommended TLA+ obligation**: Model `events_for_run` as `Seq<JournalEvent>` fetched by Fjall prefix scan. Prove `SeqOrderPreserved`: for all `i < j`, `journal[i].seq < journal[j].seq`.

### Verus/local coverage
- `codec/mod.rs::validate_replayed_event` — Verus-appropriate pure function; proven by construction (checked arithmetic + early return on mismatch)
- `codec/mod.rs::next_seq` — `checked_add` with `SequenceOverflow` error; overflow is impossible given bounded `EventSeq` range
- No Verus proof module exists for journal invariants

---

## 2. Key Encoding — Parse Don't Validate Contract

### Contract clauses

- **KEY-INV-001**: Every key encoder returns `Result<FixedArray, JournalError>` — encoding failures are unrecoverable programmer errors propagated as `KeyCapacity`
- **KEY-INV-002**: Each key variant uses a distinct type prefix byte; all digest-key variants (workflow_source, compiled_ir, blob) have distinct prefixes; all run-key variants (header, event, snapshot) have distinct prefixes
- **KEY-INV-003**: Big-endian byte order for all multi-byte numeric fields within keys
- **KEY-INV-004**: Key encoding is deterministic: same inputs always produce identical bytes

### Coverage assessment

All four invariants are **well tested** in `keys.rs:tests` with exhaustive cases covering prefix uniqueness, length, big-endian encoding, zero/max boundary values, and determinism. No `unsafe`, no `unwrap`, no `expect`.

### GAP-KEY-001: No `decode_key` — parse direction untested
The module provides encoding functions but **no corresponding decode/parse functions** to reconstruct `StorageKey` variants from encoded bytes. The "parse don't validate" contract requires that:
1. `encode_key(parse_key(encoded)) == encoded` (round-trip)
2. `parse_key(encode_key(variant)) == Some(variant)` (inverse)

These properties are not verified. If a caller ever deserializes keys from storage (e.g., during migration or debugging), there is no validated decode path.

**Recommended**: Add `decode_key(bytes: &[u8]) -> Result<StorageKey, JournalError>` with the same validation-before-parse discipline as the codec. Cover with proptest.

### GAP-KEY-002: Key capacity errors are opaque
`KeyCapacity` error carries no information about which field overflowed or what the byte budget is. This makes debugging key encoding failures difficult. Not a contract gap per se, but a diagnostic gap.

---

## 3. Codec Contracts — Magic+CRC Before Parse

### Contract clauses

- **CODEC-INV-001**: `decode_record_header` validates in order: magic → schema_version → known_kind → kind_family → header_len → payload_len → CRC32C → return header
- **CODEC-INV-002**: `decode_record_payload` calls `decode_record_header` first, then slices payload using validated header fields, then verifies BLAKE3 digest, then calls postcard
- **CODEC-INV-003**: Payload allocation only occurs after all header fields are validated and bounds-checked
- **CODEC-INV-004**: `postcard::from_bytes` is only called after CRC + digest validation passes

### Validation order verification

`codec/header.rs:26-58` — order confirmed:
```
31: get(..RECORD_HEADER_BYTES) → UnexpectedEof
35: decoded.magic != expected_magic → BadMagic
40: validate_schema_version → MigrationRequired | UnsupportedSchemaVersion
41: validate_known_kind → UnknownRecordKind
42: validate_kind_family → RecordKindFamilyMismatch
43: decoded.header_len != RECORD_HEADER_LEN → HeaderLengthMismatch
48: decoded.payload_len > max_payload_len → PayloadTooLarge
54: crc32c mismatch → HeaderChecksumMismatch
→ Ok(decoded)
```

`codec/payload.rs:56-81` — after header Ok:
```
62-68: usize conversions with overflow checks → UnexpectedEof
69-71: get(payload_start..payload_end) → UnexpectedEof
72: verify_digest_match → PayloadDigestMismatch
73-81: build RecordEnvelope, return payload slice
```

Postcard decode only called after all above pass — **contract sound**.

### GAP-CODEC-001: Postcard error erasure
`codec/mod.rs:42` maps postcard errors to generic `JournalError::PostcardDecodeFailed`. This loses the specific postcard error variant (serialize error, insufficient bytes, etc.). Not a safety gap since the result is Err, but a diagnostic gap.

### GAP-CODEC-002: Kani harness magic constant mismatch
`kani_record_magic.rs` uses `0x5650424Cu32` ("VPRL") as test magic, but the actual `MAGIC_WORKFLOW_SOURCE` is `0x5642_5352` ("VBSR"). The harness doesn't test against real crate constants. This is a test quality gap, not a contract gap, since the property (magic validation) is still being tested.

**Recommended**: Re-harness against actual `MAGIC_*` constants from `constants.rs`.

---

## 4. Recovery Contracts — Replay Divergence Handling

### Contract clauses

- **RECV-INV-001**: `replay_events` returns `ReplayDivergence` if a `StepStarted` event has `step < last_step` within the latest attempt
- **RECV-INV-002**: `replay_events` returns `NonIdempotentActionBlocked` if an `ActionScheduled`/`ActionCompleted`/`ActionFailed` event's `(action, step)` is already in `ActionReplayTracker`
- **RECV-INV-003**: `recover_snapshot_plus_tail` returns `ReplayDivergence` if any tail event has `seq <= snapshot_seq`
- **RECV-INV-004**: `recover_full_journal` returns `PolicyDigestMismatch` if no `RunAdmission` event exists and policy digest verification is required (GAP-3)
- **RECV-INV-005**: `ActionReplayTracker` tracks completed and failed actions separately; `is_resolved` returns true for either

### GAP-RECV-001: `ActionReplayTracker` does not track attempt
`is_resolved(action, step)` in `recovery/types.rs:351` returns true if `(action, step)` is in `completed` or `failed` sets — **no attempt dimension**. However, `replay_events` filters at the event level (only processes events from `max_attempt`), so resolved tracking is attempt-agnostic.

This is **correct by construction** but **not proven**. The filtering logic in `replay_events:43-49` skips events from older attempts before the tracker lookup, meaning old-attempt completions never reach `is_resolved`. The coupling between the filter and the tracker is implicit, not enforced by a type-level invariant.

**Recommended Verus proof**: Prove that for any `(action, step)` pair, at most one attempt's completion/failure is recorded in the tracker because events from non-max attempts are filtered before reaching the tracker methods.

### GAP-RECV-002: GAP-3 policy digest check not exercised
`recovery/replay/core.rs:143-147`:
```rust
if !has_run_admission && !expected_policy_digests.is_empty() {
    return Err(RecoveryError::PolicyDigestMismatch { step: StepIdx::ZERO });
}
```
This branch is marked GAP-3 and has no test coverage in `recovery/tests.rs`. The interaction between absent `RunAdmission` and non-empty policy digests is not exercised.

### GAP-RECV-003: No TLA+ model for snapshot+tail recovery invariant
`SNAP-INV-001` (tail events all after snapshot seq) is tested in `recover_snapshot_plus_tail` but has no TLA+ model. The existing `RecoveryReplay.tla` does not cover snapshot isolation.

**Recommended TLA+ obligation**: Model `SnapshotPlusTail` with variables `snapshot_seq`, `tail_events`. Prove `TailAfterSnapshot`: for all `e in tail_events`, `e.seq > snapshot_seq`.

### GAP-RECV-004: Fjall prefix iteration order assumed but not contracted
`journal/replay.rs:33` uses `snap.prefix(&self.events, run_prefix_key(run)?)`. Fjall's prefix iteration order is byte-key order (little-endian vs big-endian is irrelevant — it's the keyspace ordering). Since keys are `[0x11][run_id_be][seq_be]`, prefix iteration yields events in seq order. This is **relied upon but not formally documented** as a contract with Fjall.

---

## Summary of Contract Gaps

| ID | Area | Severity | Description |
|----|------|----------|-------------|
| GAP-SEQ-001 | Journal ordering | **Medium** | Sequence contiguity validated per-event, not as atomic post-condition; needs TLA+ `SeqOrderPreserved` proof |
| GAP-KEY-001 | Key encoding | **Medium** | No `decode_key` / round-trip proof; parse direction untested |
| GAP-CODEC-001 | Codec | Low | Postcard error variant erased to `PostcardDecodeFailed`; diagnostic gap |
| GAP-CODEC-002 | Codec | Low | Kani harnesses use non-constant test magic values instead of actual `MAGIC_*` constants |
| GAP-RECV-001 | Recovery | **Medium** | `ActionReplayTracker` attempt-agnostic design relies on implicit filtering coupling; needs Verus proof of tracker/invariant correctness |
| GAP-RECV-002 | Recovery | Medium | GAP-3 policy digest branch (`!has_run_admission && !expected_policy_digests.is_empty()`) has no test coverage |
| GAP-RECV-003 | Recovery | Medium | Snapshot+tail invariant (`TailAfterSnapshot`) has no TLA+ model |
| GAP-RECV-004 | Recovery | Low | Fjall prefix iteration order assumed but not contracted as part of storage contract |

---

## Contract Soundness Assessment

**Codec magic+CRC before parse** — ✅ SOUND. Validation order is correct: magic → schema → kind-family → header_len → payload_len → CRC32C → payload digest → postcard. Kani harnesses exist (VB-STORAGE-DECODE-001 through 006).

**Key encoding** — ⚠️ PARTIALLY SOUND. Encoding invariants (prefix uniqueness, big-endian, determinism) are well-tested. Missing decode path leaves parse direction unverified (GAP-KEY-001).

**Journal event ordering** — ⚠️ PARTIALLY SOUND. Per-event validation is correct; atomic sequence post-condition is missing (GAP-SEQ-001). Step ordering and attempt filtering are implemented but lack TLA+ formalization beyond the narrower `RecoveryReplay.tla`/`AttemptTracking.tla` scope.

**Recovery replay** — ⚠️ PARTIALLY SOUND. Divergence detection is implemented. `ActionReplayTracker` design is sound but informally coupled to the filter (GAP-RECV-001). GAP-3 policy branch untested (GAP-RECV-002). Snapshot+tail invariant needs TLA+ model (GAP-RECV-003).

---

## STATUS

**`CONTRACT_GAPS_FOUND: 8`**

- Medium: GAP-SEQ-001, GAP-KEY-001, GAP-RECV-001, GAP-RECV-002, GAP-RECV-003
- Low: GAP-CODEC-001, GAP-CODEC-002, GAP-RECV-004

No soundness-breaking gaps found. All four contract areas are structurally correct; gaps are completeness gaps (missing TLA+ models, missing tests, missing decode path) rather than soundness violations.
