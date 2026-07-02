# Contract — vb-5bqmr

**Bead**: vb-5bqmr — SlotExtra: reject unknown VBSE versions instead of legacy downgrade (P1 bug)
**State 3 — rust-contract output**. This is the canonical contract; downstream agents (proof-planner, proof-writer, test-planner, holzman-rust) MUST satisfy every behavior-affecting clause.

## 1. Context

### 1.1 Problem statement

`decode_slot_written_extra` (currently `crates/vb_storage/src/slot_extra.rs:60-69`) classifies any byte slice that does not EXACTLY equal `b"VBSE\x01"` as `DecodedSlotWrittenExtra::LegacyFrameExtra(bytes)`. This is the legacy-downgrade anti-pattern: a writer emitting `b"VBSE\x02..."` would be silently treated as legacy, with the durable taint metadata thrown away. The fix MUST replace this implicit classification with an explicit `Err(SlotWrittenExtraError::VersionMismatch { found })` for the magic-but-unknown-version branch, while preserving the legacy fallback for bytes that do NOT begin with the VBSE magic.

### 1.2 Call-graph blast radius

| Site | Symbol | Behavior |
|---|---|---|
| `crates/vb_storage/src/slot_extra.rs:60-69` | `decode_slot_written_extra` | PRIMARY EDIT — tighten discriminator |
| `crates/vb_storage/src/slot_extra.rs:7` | `SLOT_WRITTEN_EXTRA_PREFIX` | HOIST to `MAGIC` + `VERSION` |
| `crates/vb_storage/src/slot_extra.rs:12-19` | `SlotWrittenExtraError` | ADD variant `VersionMismatch { found: u8 }` |
| `crates/vb_storage/src/recovery/replay/summary/hydrate.rs:220-235` | `decoded_slot_taint` | ADD version-mismatch arm with `tracing::warn!` |
| `crates/vb_runtime/src/primitives/collect.rs:248-275` | `hydrate_slot_written_extra` | ADD version-mismatch arm with `tracing::warn!` |
| `crates/vb_core/src/errors.rs` | `CollectExtraHydrationFailureKind` | ADD arm `VersionMismatch` |

### 1.3 Decisions ratified by this contract

1. **Surface (a)** in `codebase-map.md §5 Q1` — add `VersionMismatch` ONLY to `SlotWrittenExtraError`. Do NOT widen `RecoveryError`.
2. **Hoist** the 5-byte prefix into two constants: `SLOT_WRITTEN_EXTRA_MAGIC: &[u8; 4] = b"VBSE"` and `SLOT_WRITTEN_EXTRA_VERSION: u8 = 0x01`. RETAIN `SLOT_WRITTEN_EXTRA_PREFIX` as a composition of the two.
3. **Surface (i)** for the collect side — add `CollectExtraHydrationFailureKind::VersionMismatch`. Do NOT widen `EngineError` enum root.

These are binding unless redacted in a later contract revision.

## 2. Behavior-affecting clauses

Each clause is a single contract obligation. The implementation agent MUST satisfy every clause; the proof agent MUST prove every clause.

### 2.1 Codec — discriminator (binding)

**C-DEC-001** For all `bytes: &[u8]`,

> If `bytes.len() >= 5` and the first 4 bytes equal `SLOT_WRITTEN_EXTRA_MAGIC` and the 5th byte equals `SLOT_WRITTEN_EXTRA_VERSION`, then `decode_slot_written_extra(bytes)` returns the result of `postcard::from_bytes::<SlotWrittenExtraEnvelope>(&bytes[5..])` mapped into either `Ok(DecodedSlotWrittenExtra::Envelope(_))` or `Err(DecodeFailed)`.

**C-DEC-002** For all `bytes: &[u8]`,

> If `bytes.len() >= 5` and the first 4 bytes equal `SLOT_WRITTEN_EXTRA_MAGIC` and the 5th byte does NOT equal `SLOT_WRITTEN_EXTRA_VERSION`, then `decode_slot_written_extra(bytes)` returns `Err(SlotWrittenExtraError::VersionMismatch { found: bytes[4] })`.

**C-DEC-003** For all `bytes: &[u8]`,

> If `bytes.len() < 5` OR the first 4 bytes do NOT equal `SLOT_WRITTEN_EXTRA_MAGIC`, then `decode_slot_written_extra(bytes)` returns `Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(bytes))`.

**C-DEC-004** The three arms (decode / version-mismatch / legacy) are MUTUALLY EXCLUSIVE and EXHAUSTIVE. For any `bytes`, exactly one of {C-DEC-001, C-DEC-002, C-DEC-003} applies.

### 2.2 Codec — constant invariants (binding)

**C-CON-001** `SLOT_WRITTEN_EXTRA_PREFIX.as_slice() == SLOT_WRITTEN_EXTRA_MAGIC.iter().chain(std::iter::once(&SLOT_WRITTEN_EXTRA_VERSION)).copied().collect::<Vec<u8>>()`. Or, equivalently, `SLOT_WRITTEN_EXTRA_PREFIX == b"VBSE\x01"`.

**C-CON-002** `SLOT_WRITTEN_EXTRA_PREFIX` is RETAINED with its historical byte sequence. Removing or renaming it requires an additive `pub use` re-export in `crates/vb_storage/src/lib.rs:208-211`.

**C-CON-003** `SLOT_WRITTEN_EXTRA_MAGIC` and `SLOT_WRITTEN_EXTRA_VERSION` are public constants (re-exported from `vb_storage::slot_extra` and visible to `vb_runtime`).

**C-CON-004** `SLOT_WRITTEN_EXTRA_PREFIX_LEN == SLOT_WRITTEN_EXTRA_MAGIC.len() + 1 == 5`.

### 2.3 Codec — error invariant (binding)

**C-ERR-001** `SlotWrittenExtraError::VersionMismatch { found }` is `Copy`-able and carry-able through `Result` chains without allocation.

**C-ERR-002** `SlotWrittenExtraError::VersionMismatch { found: 0x01 }` is UNREACHABLE from the decoder (because the discrimination selects the v1 branch instead).

**C-ERR-003** Every `bytes: &[u8]` produces at most one of {`Ok(Envelope(_))`, `Ok(LegacyFrameExtra(_))`, `Err(DecodeFailed)`, `Err(VersionMismatch { found })`}.

### 2.4 Recovery translation (binding)

**C-REC-001** At `crates/vb_storage/src/recovery/replay/summary/hydrate.rs:220-235`, `decoded_slot_taint` matches every `SlotWrittenExtraError` variant EXPLICITLY. No catch-all arm on the storage-layer error.

**C-REC-002** When the codec result is `Err(VersionMismatch { found })`, the translation is `Err(RecoveryError::CorruptSlotTaint { slot })` AND emits a `tracing::warn!(slot, found, "slot extra: VBSE magic present but unknown version")`.

**C-REC-003** When the codec result is `Err(DecodeFailed)` (or `Err(EncodeFailed)` or `Err(AllocationFailed)` defensively), the translation is `Err(RecoveryError::CorruptSlotTaint { slot })` without additional logging at this level.

**C-REC-004** `RecoveryError` is NOT widened in this bead. The compile-time exhaustiveness test at `recovery_unit_tests.rs:1149-1172` MUST remain green without modification of that test's match arms.

### 2.5 Collect translation (binding)

**C-RUN-001** At `crates/vb_runtime/src/primitives/collect.rs:248-275`, `hydrate_slot_written_extra` matches every `SlotWrittenExtraError` variant EXPLICITLY. No catch-all arm on the storage-layer error.

**C-RUN-002** When the codec result is `Err(VersionMismatch { found })`, the translation is `Err(EngineError::CollectExtraHydrationFailed { kind: CollectExtraHydrationFailureKind::VersionMismatch, run_id: run, collector_slot: slot, event_seq: Some(core_event_seq(seq)) })` AND emits `tracing::warn!(slot, seq, found, "slot extra: VBSE magic present but unknown version")`.

**C-RUN-003** When the codec result is `Err(DecodeFailed)` (defensively: `Err(EncodeFailed)` / `Err(AllocationFailed)`), the translation is `Err(EngineError::CollectExtraHydrationFailed { kind: CollectExtraHydrationFailureKind::DecodeFailed, run_id: run, collector_slot: slot, event_seq: Some(core_event_seq(seq)) })`.

**C-RUN-004** `CollectExtraHydrationFailureKind` is `#[non_exhaustive]` (or made so in the same edit), and gains exactly one new arm: `VersionMismatch`. No other arms are touched.

### 2.6 Encoder (binding)

**C-ENC-001** `encode_slot_written_extra(taint, frame_extra)` is unchanged. Its output is the byte sequence `SLOT_WRITTEN_EXTRA_PREFIX.iter().chain(postcard_bytes_of_envelope).copied().collect::<Vec<u8>>()`. A no-op edit MUST keep the byte sequence identical.

**C-ENC-002** `encode_slot_written_extra` must continue to round-trip with `decode_slot_written_extra(encode_result).map(|d| d.as_envelope_ref()) == Ok(Envelope(taint, frame_extra))` for all `taint: Taint` and `frame_extra: Option<Vec<u8>>`.

### 2.7 Negative invariants (binding)

**C-NEG-001** `decode_slot_written_extra(b"\x01\x02\x03\x04")` returns `Ok(LegacyFrameExtra(b"\x01\x02\x03\x04"))`. ZERO allocation. No `unsafe`.

**C-NEG-002** `decode_slot_written_extra(b"VBSE")` returns `Ok(LegacyFrameExtra(b"VBSE"))`. The 4-byte magic-only input cannot claim a version byte, so the legacy arm is correct.

**C-NEG-003** `decode_slot_written_extra(b"VBSE\x01\xff\xff\xff")` returns `Err(DecodeFailed)`.

**C-NEG-004** `decode_slot_written_extra(b"VBSE\x02\xff\xff\xff")` returns `Err(VersionMismatch { found: 0x02 })`.

**C-NEG-005** `decode_slot_written_extra(b"VBSE\xFF\xff\xff\xff")` returns `Err(VersionMismatch { found: 0xFF })`.

**C-NEG-006** `decode_slot_written_extra` does not allocate on the legacy arm.

### 2.8 Open forbidden states (binding)

**C-FOR-001** No site in `crates/vb_storage/src/slot_extra.rs` may match on `Err(_)` (catch-all) — every call site must enumerate `SlotWrittenExtraError`.

**C-FOR-002** No site in `hydrate.rs` or `collect.rs` may match on `Err(slot_extra::_)` (catch-all) — every call site must enumerate `SlotWrittenExtraError`.

**C-FOR-003** No future bead may re-introduce the "magic-with-wrong-version classified as legacy" behavior in this codebase without first redacting this contract.

## 3. Engineered envelope (non-behavior-affecting reference)

These are NOT proof obligations. They document the migration path for downstream crates.

### 3.1 Re-export surface

```rust
// crates/vb_storage/src/lib.rs:208-211 (REPLACE)

pub use slot_extra::{
    DecodedSlotWrittenExtra,
    SLOT_WRITTEN_EXTRA_MAGIC,
    SLOT_WRITTEN_EXTRA_PREFIX,
    SLOT_WRITTEN_EXTRA_VERSION,
    SlotWrittenExtraEnvelope,
    SlotWrittenExtraError,
    decode_slot_written_extra,
    encode_slot_written_extra,
};
```

### 3.2 Module doc

```rust
//! Versioned slot-write extra envelope.
//!
//! Producers emit a versioned envelope prefixed with [`SLOT_WRITTEN_EXTRA_PREFIX`]
//! = `[`[`SLOT_WRITTEN_EXTRA_MAGIC`]`, &[`[`SLOT_WRITTEN_EXTRA_VERSION`]`]]`. Future
//! versions MUST emit a different version byte and adjust the decoder to
//! recognise them. The decoder rejects any version byte other than the current
//! one with `Err(SlotWrittenExtraError::VersionMismatch { found })` instead of
//! silently downgrading to the legacy-frame-extra arm.
```

### 3.3 Test module

A new `#[cfg(test)] mod tests` at the bottom of `slot_extra.rs` covers C-NEG-001..006 (negative invariants) and C-DEC-001..003 (discriminator). Test fixtures:

| Input | Expected output |
|---|---|
| `b"\x01\x02\x03\x04"` | `Ok(LegacyFrameExtra(b"\x01\x02\x03\x04"))` |
| `b"VBSE"` (4 bytes) | `Ok(LegacyFrameExtra(b"VBSE"))` |
| `b"VBSE\x01\xff\xff\xff"` | `Err(DecodeFailed)` |
| `b"VBSE\x02\xff\xff\xff"` | `Err(VersionMismatch { found: 0x02 })` |
| `b"VBSE\xFF\xff\xff\xff"` | `Err(VersionMismatch { found: 0xFF })` |
| `b"VBSE\x00\xff\xff\xff"` | `Err(VersionMismatch { found: 0x00 })` |
| `&encode_slot_written_extra(Taint::DerivedFromSecret, Some(vec![1,2,3]))?` (round-trip) | `Ok(Envelope(Taint::DerivedFromSecret, Some(vec![1,2,3])))` |
| `SLOT_WRITTEN_EXTRA_PREFIX` (identity) | `SLOT_WRITTEN_EXTRA_PREFIX.as_slice() == b"VBSE\x01"` |

## 4. Out-of-scope (explicit non-goals)

- No fuzz harness for `decode_slot_written_extra` (RED QUEEN §M3, `vb-1rqz7.15` separate).
- No `PayloadTooLarge` cap on decoded `Vec` allocation (wave3 `agent-01-holzman-rust-B.md:15`; `vb-1rqz7.15.15` separate).
- No encoder change (v1 only).
- No version-counter bump.
- No `RecoveryError` widening.
- No `EngineError` enum-root widening.

## 5. Inputs and dependencies (for downstream agents)

### 5.1 Upstream (this contract provides)

- Domain ubiquity (§1 of `domain-model.md`).
- Type surface (§3 of `type-contracts.md`).
- Hazard list (§1 of `hazard-analysis.md`).
- Proof seeds (`proof-seeds.jsonl`).
- Traceability map (`traceability-matrix.jsonl`).

### 5.2 Downstream (consumers of this contract)

| Agent | Reads | Writes |
|---|---|---|
| `proof-planner` | `contract.md`, `proof-seeds.jsonl`, `hazard-analysis.md` | planned proof obligations, lane decisions (NOT written by rust-contract) |
| `proof-writer` | approved planned obligations | Verus / Kani / Flux / proptest harnesses |
| `test-planner` | `contract.md`, `hazard-analysis.md` | `test-plan.md`, behavior tests |
| `holzman-rust` | `contract.md`, `type-contracts.md`, `error-taxonomy.md` | production Rust source changes |
| `black-hat-reviewer` | all 9 artifacts | review verdicts |

## 6. Acceptance gates

| Gate | Status (rust-contract) |
|---|---|
| All 9 artifacts written | PENDING `proof-seeds.jsonl` and `traceability-matrix.jsonl` |
| Each artifact carries bead ID + scope and no migration of out-of-scope concerns | YES |
| `cargo build --all-targets -p vb_storage` is NOT run by rust-contract (the source-lint gate is downstream) | n/a |
| Every behavior-affecting clause (C-DEC-001..004, C-ERR-001..003, C-REC-001..004, C-RUN-001..004, C-NEG-001..006, C-FOR-001..003) maps to at least one proof seed | PENDING `proof-seeds.jsonl` |

## 7. Closure conditions (what acceptance looks like at the right level of detail)

- 9 artifacts present under `.beads/vb-5bqmr/`.
- Every behavior-affecting clause references at least one proof seed and one test fixture.
- No clause contradicts `domain-model.md` or `error-taxonomy.md`.
- No out-of-scope item appears in this contract's clauses.
