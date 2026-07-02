# Bead vb-5bqmr — Codebase Map (State 2 / explore)

- **bead_id**: vb-5bqmr
- **title**: SlotExtra: reject unknown VBSE versions instead of legacy downgrade (P1 bug)
- **isolated_workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr`
- **captured_at**: 2026-07-01T15:27Z
- **scout**: explore (artifact-writing only; no production edits)
- **scope_mode**: focused — single-call graph blast radius, no broad re-scan

## 1. Problem Frame (literated from bead text + grep evidence)

The SlotWrite extra envelope is encoded as
`SLOT_WRITTEN_EXTRA_PREFIX = b"VBSE\x01"` followed by a postcard-serialized
`SlotWrittenExtraEnvelope { taint, frame_extra }`. The 5-byte prefix embeds
a 1-byte version byte (`\x01`).

`decode_slot_written_extra` (`crates/vb_storage/src/slot_extra.rs:60-69`)
currently treats **any byte slice that does not start with exactly
`b"VBSE\x01"`** as a `DecodedSlotWrittenExtra::LegacyFrameExtra(bytes)`.
That is the legacy-downgrade anti-pattern: a future writer that emits a
recognised-but-newer VBSE magic (e.g. `b"VBSE\x02"`, `b"VBSE\x03"`, …) is
silently re-classified as legacy frame extra and processed through the
`LegacyFrameExtra` lattice instead of being rejected.

This bead narrows the legacy fallback to bytes that **do not start with the
`"VBSE"` magic at all**, and forces an explicit `VersionMismatch` error on
the "magic-but-unknown-version" branch.

## 2. Touched / Suspected Files

All paths are relative to `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr/`.

### 2.1 Production source — primary edit surface

| Path | Why it is in scope |
|---|---|
| `crates/vb_storage/src/slot_extra.rs` | Defines the prefix constant, `SlotWrittenExtraError`, `DecodedSlotWrittenExtra`, `encode_slot_written_extra`, `decode_slot_written_extra`. The decode function (lines 60–69) is **exactly** the legacy-downgrade site. |

Key symbols (line numbers from disk):
- `pub const SLOT_WRITTEN_EXTRA_PREFIX: &[u8; 5] = b"VBSE\x01"` — slot_extra.rs:7
- `pub enum SlotWrittenExtraError { EncodeFailed, AllocationFailed, DecodeFailed }` — slot_extra.rs:10-19 (no `VersionMismatch` variant — must be added; enum is `#[non_exhaustive]` already)
- `pub struct SlotWrittenExtraEnvelope { pub taint: Taint, pub frame_extra: Option<Vec<u8>> }` — slot_extra.rs:22-28
- `pub enum DecodedSlotWrittenExtra<'a> { Envelope(SlotWrittenExtraEnvelope), LegacyFrameExtra(&'a [u8]) }` — slot_extra.rs:31-37
- `pub fn encode_slot_written_extra(taint, frame_extra) -> Result<Vec<u8>, SlotWrittenExtraError>` — slot_extra.rs:40-57 (NO change required; encodes only the v1 envelope and the prefix already differentiates)
- `pub fn decode_slot_written_extra(bytes: &[u8]) -> Result<DecodedSlotWrittenExtra<'_>, SlotWrittenExtraError>` — slot_extra.rs:60-69 (CHANGE site: add an explicit `bytes.starts_with(b"VBSE")` test that returns `SlotWrittenExtraError::VersionMismatch { found: u8 }` for the magic-but-unknown-version branch; tighten the legacy branch to "magic not present at all")

### 2.2 Production source — required propagation sites

| Path | Call-site identifier | Required handling change |
|---|---|---|
| `crates/vb_storage/src/recovery/replay/summary/hydrate.rs` | `decoded_slot_taint` at line 220-235; `recovered_slot_taint` at line 209-218; `record_slot_write` at line 275-298 | Today every `Err(_)` from `decode_slot_written_extra` collapses to `RecoveryError::CorruptSlotTaint { slot }` (line 233). Must distinguish `VersionMismatch` (proposed: `RecoveryError::SlotExtraVersionMismatch { slot, found }`) from generic decode failure. |
| `crates/vb_runtime/src/primitives/collect.rs` | `hydrate_slot_written_extra` at line 248-275 | Today every `Err(_)` from `vb_storage::decode_slot_written_extra` collapses to `EngineError::CollectExtraHydrationFailed { kind: DecodeFailed, .. }`. Must distinguish the version-mismatch path; either a new `CollectExtraHydrationFailureKind::VersionMismatch` or reuse `DecodeFailed` per contract decision. |

### 2.3 Public-surface and crate re-export (verify, do NOT change unless forced)

| Path | Surface |
|---|---|
| `crates/vb_storage/src/lib.rs:187` | `pub mod slot_extra;` |
| `crates/vb_storage/src/lib.rs:208-211` | `pub use slot_extra::{ DecodedSlotWrittenExtra, SLOT_WRITTEN_EXTRA_PREFIX, SlotWrittenExtraEnvelope, SlotWrittenExtraError, decode_slot_written_extra, encode_slot_written_extra }` (re-exports — confirm the new `SlotWrittenExtraError::VersionMismatch` is reachable; no re-export change needed beyond that). |

### 2.4 Recovery error surface (only if we introduce a new `RecoveryError` variant)

| Path | Why |
|---|---|
| `crates/vb_storage/src/recovery/types.rs:36-140` | `RecoveryError` enum (currently 13 variants + `Journal`). Adding a `SlotExtraVersionMismatch { slot, found }` variant requires updating: (1) the enum definition with doc, (2) `Display` impl (line 142-207), (3) `Error::source` match (line 209-229), and (4) `PartialEq` impl (line 246-336). |
| `crates/vb_storage/src/recovery/recovery_unit_tests.rs:1149-1172` | Compile-time exhaustiveness check covering **every** existing variant; adding a variant breaks this build and forces the test to be updated in lockstep. |
| `crates/vb_storage/src/recovery/recovery_unit_tests.rs:1162` | The `_exhaustive_match` arm already includes `RecoveryError::CorruptSlotTaint { .. } => "corrupt_slot_taint"` and serves as the registry of variant-name strings; a new variant must be added here too (used by snapshot / golden tests). |

### 2.5 Existing tests (sit on the call graph)

| Path | Test / helper |
|---|---|
| `crates/vb_runtime/tests/recovery_bdd_tests.rs:3158-3211` (`typed_rejection_hydrate_from_events_slot_taint_fails_closed`) | Uses `extra: Some(vec![0x01, 0x02, 0x03, 0x04])` — bytes that do **not** start with `"VBSE"`. Should remain `LegacyFrameExtra` and propagate as `UnsupportedFrameSeed` with reason `"slot_taint"`. **MUST NOT regress.** |
| `crates/vb_storage/src/recovery/tests.rs:2332-2336` | `corrupt_slot_taint_envelope()` helper builds `[VBSE\x01, 255, 255, 255]`, exercising the `DecodeFailed` arm of `decode_slot_written_extra` (current behavior at slot_extra.rs:66). After the fix, the version byte is `\x01` and this remains a `DecodeFailed` — no behavior change for v1 corruption. |
| `crates/vb_storage/src/recovery/tests.rs:2507-2536` (`hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata`) | Asserts `Err(RecoveryError::CorruptSlotTaint { slot })` for `corrupt_slot_taint_envelope`. Should remain green if we add a distinct `SlotExtraVersionMismatch` variant for unknown-version bytes (v1 corruption still maps to `CorruptSlotTaint`). |
| `crates/vb_storage/src/recovery/tests.rs:2538-2570` (`hydrate_run_frame_from_events_accepts_legacy_frame_extra_without_taint_sidecar`) | Uses `extra: Some(vec![1, 2, 3, 4])` — bytes that do not start with `VBSE`. Must continue to classify as `LegacyFrameExtra`. |
| `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:57-61, 263-297` | Mirror of the corrupt-envelope test in the cross-crate workspace_tests area; same compatibility profile (corrupt bytes, not unknown-version bytes). |

### 2.6 Out-of-scope but referenced (no edit)

| Path | Reason |
|---|---|
| `crates/vb_runtime/src/journal/chunk_002.rs:326-333` (`encoded_slot_taint_extra`) | Producer side; only ever encodes v1 with `SLOT_WRITTEN_EXTRA_PREFIX`. No change needed. |
| `crates/vb_runtime/src/journal/tests/chunk_002.rs:140` | Asserts the writer side; no behavior change. |
| `crates/vb_storage/src/error/warnings.rs:14` (`SchemaVersionMismatch`) | Closest sibling variant in the warning surface; useful as a naming convention but not a target. |
| `fuzz/RED_QUEEN_MASTER_PLAN.md:144,378`, `fuzz/FUTURE.md:36`, `fuzz/research/red-queen-strategy.md:230,399` | Document that `decode_slot_written_extra` is a P0 fuzz target. The current fuzz target list has **no harness** for this function (`fuzz/research/red-queen-strategy.md:399`). Bead scope does NOT include adding the fuzz harness. |
| `to-fix/wave3/agent-01-holzman-rust-B.md:15` | Floats a separate concern (decoded-`Vec` allocation cap, `PayloadTooLarge` error). That is a **different** open issue (vb-1rqz7.15); bead vb-5bqmr does **not** address it. Out of scope. |
| `to-fix/wave3/agent-04-truth-serum.md:13` | Walks the existing PATCHED status of the taint-decode plumbing. Confirms only one site (`recovery/replay/summary/hydrate.rs:225`) carries the decode result into the recovery-error lattice. |

## 3. Call-Graph Blast Radius

```
encode_slot_written_extra       (producer — UNCHANGED)
       │
       ▼  // written by journal/chunk_002.rs:330
StorageRuntimeJournal.append_sequenced  (writes SlotWrittenEvent.extra)
       │
       │  durable storage
       ▼
hydrate_run_frame_from_events  (summary hydrator)
  └── record_slot_write        (replay/summary/hydrate.rs:275)
        └── recovered_slot_taint (line 209)
              └── decoded_slot_taint (line 220)  ◀── THIS BEAD
                    └── crate::slot_extra::decode_slot_written_extra  ◀── primary edit

hydrate_run_frame (snapshot + tail)
  └── apply_tail_events (recover/hydrate_support.rs:264, NOT touched by this bead)
        └── frame.read_taint OR decode_slot_written_extra (untouched in this bead)

CollectStates::hydrate_journal_events  (runtime/primitives/collect.rs:228)
  └── CollectStates::hydrate_journal_event (line 234)
        └── CollectStates::hydrate_slot_written_extra (line 248)  ◀── THIS BEAD
              └── vb_storage::decode_slot_written_extra  ◀── primary edit
```

## 4. Existing Verifier Coverage Inventory (pre-bead)

| Lane | Coverage of slot_extra in current codebase | Notes |
|---|---|---|
| **Unit (cargo test)** | NONE dedicated to `decode_slot_written_extra`. Helpers at recovery/tests.rs:2332 and workspace_tests:57 exercise the corrupt-payload arm indirectly. | No existing `slot_extra_tests.rs`. New tests required. |
| **Behavior (BDD)** | `typed_rejection_hydrate_from_events_slot_taint_fails_closed` (recovery_bdd_tests.rs:3172) covers the legacy-frame path; same module also covers corrupt-taint at line 2508 indirectly. | New BDD scenario required for unknown-version path. |
| **proptest / Kani / Loom / fuzz** | Zero. `fuzz/research/red-queen-strategy.md:399` explicitly flags the function as MISSING fuzz coverage. | Out-of-scope for this bead (separate concern from the version-mismatch bug). |
| **Verus** | No `verification/verus/` artifact mirrors `slot_extra.rs`. The closest is `recovery_hydration_contracts.rs` but it proves generic hydration only (per `to-fix/wave2/agent-09-verus.md:36`). | Optional; if added must bind to slot_extra.rs via `#[path = ".../crates/vb_storage/src/slot_extra.rs"]`. |
| **moon ci** | `moon ci` is the canonical gate per `AGENTS.md`. The bead's CI gate must satisfy source lint (`#![forbid(unsafe_code)]` already present at slot_extra.rs:1). | Mandatory before landing. |

## 5. Open Questions for Downstream Agents

1. **Where does `VersionMismatch` live?** Three candidate surfaces:
   - (a) Add variant to `SlotWrittenExtraError` only (lexically tight; matches existing `#[non_exhaustive]` pattern). Callers must pattern-match on `Err(SlotWrittenExtraError::VersionMismatch { found })` vs `Err(SlotWrittenExtraError::DecodeFailed)`.
   - (b) Add a public `RecoveryError::SlotExtraVersionMismatch { slot, found }` variant in addition to (a). Maps cleanly into the recovery-error lattice; risks breaking the compile-time exhaustiveness check at recovery_unit_tests.rs:1149 and the PartialEq/Display/Error impls at recovery/types.rs:142-336.
   - (c) Reuse `RecoveryError::CorruptSlotTaint { slot }` with a refinement field (`cause: SlotTaintCause`). Wider blast radius through tests.
   **Recommendation (for downstream contract agent)**: surface (a) is minimal and fits the bead text "reject unknown VBSE versions with a VersionMismatch error variant" without expanding the recovery-error enum. Surface (b) is preferred only if downstream code (typed gate, hydration, diagnostics) needs to branch on the cause.
2. **Magic boundary**: confirm the version byte is the 5th byte of the prefix (`b"VBSE\x01"`). Should the magic detection be `bytes.starts_with(b"VBSE")` (4-byte magic + version byte parse) or `bytes.starts_with(b"VBSE\x01\x02\x03")` etc.? The current 5-byte constant conflates magic + version; the fix should hoist a separate `pub const SLOT_WRITTEN_EXTRA_MAGIC: &[u8; 4] = b"VBSE"` and a `pub const SLOT_WRITTEN_EXTRA_VERSION: u8 = 0x01` so the unknown-version check can read `[MAGIC.len()]` byte directly. **Decision**: contract agent should ratify.
3. **Collect-side error mapping**: `CollectExtraHydrationFailureKind` (referenced in vb_core/src/errors.rs) does not currently have a `VersionMismatch` arm. If the contract agent picks surface (a) above, do we (i) add `Kind::VersionMismatch`, (ii) reuse `Kind::DecodeFailed`, or (iii) introduce a separate `EngineError` variant? Confirm with rust-contract before proof planning.
4. **Legacy path preservation**: any `extra` payload that does **not** start with `b"VBSE"` continues to be returned as `Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(bytes))`. Confirm this is what the test author intends for `recovery_bdd_tests.rs:3172` (`vec![0x01, 0x02, 0x03, 0x04]`). The current test still expects `LegacyFrameExtra` because the bytes have no `VBSE` header.
5. **Producer vs consumer asymmetry**: `encode_slot_written_extra` (slot_extra.rs:40-57) ALWAYS emits v1; the fix is decoder-side only. No schema version bump is needed; `encode_slot_written_extra` may emit a comment noting that future versions require a new encoder.
6. **Diagnostics / error codes**: `RecoveryError` variants have NO stable `diagnostic_code` method on `RecoveryError` itself (see `crates/vb_storage/src/recovery/recovery_unit_tests.rs:1149-1168` — names only, no codes). Adding `VersionMismatch` does not require a diagnostic code change in this bead. Future bead may add codes.

## 6. Recommended Owners (downstream agents)

| Owner | Lane | Inputs from this map |
|---|---|---|
| rust-contract | Domain/type contract | §1 + §2.1 + §5 Q1, Q2, Q3 + §2.4 |
| proof-planner | Verifier lane planning | §2.4, §4 (verifier inventory), §5 Q2 (magic boundary) |
| proof-writer | Verus/Kani/Flux harnesses | §3 call graph + §4 (no current proof coverage) |
| test-planner | Behavior + unit + proptest scenarios | §2.5 existing tests + §4 BDD coverage + §5 Q4 |
| holzman-rust | Implementation | §2.1 + §2.2 + §5 Q1 |
| black-hat-reviewer | Gate | All sections; cross-check against `to-fix/wave3/agent-01-holzman-rust-B.md:15` to confirm vb-1rqz7.15 stays separate. |

## 7. Risk Tags (handed to delivery-scope.jsonl)

- `parser/codec` — discriminator must be **exact**: 4-byte magic + unknown version-byte ⇒ reject; no magic ⇒ legacy fallback.
- `public_api` — `SlotWrittenExtraError` is `#[non_exhaustive]`; adding a variant is API-additive. If a `RecoveryError` variant is added it is also additive (the enum is `#[non_exhaustive]`).
- `user_visible_behavior` — visible at the recovery gate boundary; a corrupt-now-unknown-version payload changes from "legacy frame extra processed" to "fail-closed with `VersionMismatch`". No silent regressions for v1 callers.
- `migration` — only relevant for upgraded writers emitting VBSE\x02+. The fix makes that path explicit.
- `temporal` — N/A (no async, no concurrency).
- `unsafe_UB` — N/A (`#![forbid(unsafe_code)]` enforced at the file head).
- `persistence` — derived from durable journals (SlotWrittenEvent.extra). Decoder is the only point this bead edits; encoder is unchanged.
- `auth_security` — fail-closed by design; lattice preserved (vb-i21a2 / SR-013).
- `concurrency` — N/A.
- `performance` — branch prediction only; no allocation impact.
- `dependency` — N/A (no crate dep changes; postcard stays).
