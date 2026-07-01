# Boundary Map — vb-1wora

**Bead:** `vb-1wora` — Codec: reject trailing bytes after declared record payload (P1 bug)
**Skill:** `rust-contract` (State 3)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`

This file locates the change at the boundary surfaces of the v1 storage record codec, names the pure-core / imperative-shell / async-shell / storage boundaries the change crosses, and identifies parser boundaries (where external input is parsed once and trusted thereafter).

---

## 1. Boundary layers (architectural decomposition)

The `vb_storage` crate is structured around a **functional-core / imperative-shell** boundary. The codec sits squarely in the **functional-core** layer because it is a pure parser over `&[u8]` with no I/O, no time, no randomness, and no allocation beyond the `usize` subtraction in the new check.

```
┌─────────────────────────────────────────────────────────────────────┐
│ EXTERNAL (untrusted)                                                │
│   - Fjall keyspace values (bytes from disk)                         │
│   - Wire records received over the network / IPC                    │
│   - Test fixtures / fuzz inputs                                      │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              v   <-- PARSER BOUNDARY (one-way gate)
┌─────────────────────────────────────────────────────────────────────┐
│ FUNCTIONAL CORE (pure parsers, no I/O, no time, no randomness)      │
│                                                                     │
│   decode_record_header  ──>  RecordHeader                           │
│   decode_record_payload ──>  (RecordEnvelope, &[u8])                │
│   decode_envelope_only  ──>  (RecordEnvelope, &[u8])                │
│   decode_record<T>      ──>  (RecordEnvelope, T)                    │
│   decode_journal_event  ──>  (RecordEnvelope, JournalEvent)         │
│                                                                     │
│   *** NEW TRAILING-BYTES CHECK *** lives in                          │
│   decode_record_payload and decode_envelope_only.                    │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              v   <-- typed Rust values (trusted)
┌─────────────────────────────────────────────────────────────────────┐
│ IMPERATIVE SHELL (storage, network, time, side effects)             │
│                                                                     │
│   Fjall read/write                                                   │
│   Trimming logic (has_terminal_event at trimming/logic.rs:251)      │
│   Replay / recovery loops                                            │
│   Doctor / filtering tools                                           │
└─────────────────────────────────────────────────────────────────────┘
```

## 2. Boundary table — pre-fix vs post-fix

Each row is one surface where the change either creates, removes, or preserves a boundary. The columns:

- **Surface**: the path or symbol.
- **Layer**: which architectural layer.
- **Trust**: what flows in / out.
- **Pre-fix state**: what the boundary looked like before.
- **Post-fix state**: what it looks like after.
- **Effect of the change**: created / hardened / unchanged.

| Surface | Layer | Trust | Pre-fix | Post-fix | Effect |
|---|---|---|---|---|---|
| `decode_record_payload` (`crates/vb_storage/src/codec/payload.rs:56-82`) | Functional core | Input: untrusted `&[u8]`; output: typed `RecordEnvelope` + bounded payload. | Silently accepts `bytes.len() > payload_end`. | Fails closed with `TrailingBytes { trailing }`. | **Hardened.** |
| `decode_envelope_only` (`crates/vb_storage/src/codec/envelope.rs:48-83`) | Functional core | Input: untrusted `&[u8]`; output: typed `RecordEnvelope` + bounded payload. | Silently accepts `bytes.len() > payload_end`. | Fails closed with `TrailingBytes { trailing }`. | **Hardened.** |
| `decode_record<T>` (`crates/vb_storage/src/codec/mod.rs:82-95`) | Functional core | Input: untrusted `&[u8]`; output: typed `T`. | Inherited silent acceptance via delegation. | Inherits fail-closed via delegation. | **Hardened (transitively).** |
| `decode_journal_event` (`crates/vb_storage/src/codec/mod.rs:126-151`) | Functional core | Input: untrusted `&[u8]`; output: typed `JournalEvent`. | Inherited silent acceptance. | Inherits fail-closed. | **Hardened (transitively).** |
| `has_terminal_event` (`crates/vb_storage/src/trimming/logic.rs:251`) | Imperative shell | Calls `decode_journal_event` per Fjall keyspace item. | Loop continues past malformed rows that decoded silently. | Loop continues past malformed rows that now yield `Err(TrailingBytes)`; the new variant flows through `TrimError::Journal` already. | **Unchanged at the call site** (fail-closed improvement, no signature change). |
| `JournalError::diagnostic_code` (`crates/vb_storage/src/error/codes.rs:99-176`) | Functional core | Pattern-match on variant. | No arm for `TrailingBytes`. | New arm returns `TRAILING_BYTES_CODE = 0x4042`. | **Created.** |
| `JournalError::symbolic_code` (`crates/vb_storage/src/error/codes.rs:180-268`) | Functional core | String -> `SymbolicCode`. | No arm for `TrailingBytes`. | New arm returns `"JOURNAL_TRAILING_BYTES"`. | **Created.** |
| `CODE_REGISTRY` (`crates/vb_core/src/diagnostic.rs` slice to line 1583) | Cross-crate registry | Symbolic name -> numeric code. | `JOURNAL_TRAILING_BYTES` not registered. | (Recommended) Add `("JOURNAL_TRAILING_BYTES", DiagnosticCode::new(0x4042))`. | **Created (recommended).** |
| `SpecJournalError` enum (`verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:335-413`) | Verus mirror | Mirror of production `JournalError` reachable variants. | No `TrailingBytes` variant. | New variant `TrailingBytes { trailing: u32 }`. | **Created.** |
| Verus bridge `assume_specification[ production::decode_record ]` (`verification/verus/vb-vzcuf-PS-003.rs:387-451`) | Verus spec | `ensures` clause enumerating reachable Err arms. | No arm for `TrailingBytes`. | New arm specifying `bytes.len() > expected_payload_end`. | **Created.** |
| `extern_vb_vzcuf_PS_003.rs:83-87` (`verification/verus/`) | Verus extern shim | Re-exports `SpecJournalError`. | Re-exports 15 variants. | Re-exports 16 variants automatically (no code change). | **Unchanged (re-export covers new variant).** |

## 3. Parser boundary discipline

The codec is a **parser**: it takes untrusted `&[u8]` and produces typed Rust values (`RecordEnvelope`, `JournalEvent`, etc.). Once parsed, downstream code must trust the typed values and must **not** re-parse or re-validate the bytes. The new check enforces this discipline by ensuring that the parser cannot produce typed values for malformed inputs.

### 3.1 Boundary inputs

| Input | Source | Validation |
|---|---|---|
| `bytes: &[u8]` | Fjall keyspace value, network buffer, test fixture. | All validation happens inside the parser. The caller MUST NOT pre-validate the slice length; if it did, the validation would be duplicated and could drift. |
| `expected_magic: u32` | Caller-provided. | Caller responsibility. The codec trusts the magic and uses it to dispatch. |
| `max_payload_len: u32` | Caller-provided. | Caller responsibility. Used to bound `PayloadTooLarge`. |

### 3.2 Boundary outputs

| Output | Trust level | Validation |
|---|---|---|
| `RecordEnvelope` | Trusted: typed, validated. | None needed downstream. |
| `payload: &[u8]` (sub-slice of input) | Trusted: bounds-checked to `[payload_start..payload_end]`, length is `header.payload_len as usize`, no trailing bytes (post-fix). | None needed downstream. |
| `T: DeserializeOwned` (postcard-decoded) | Trusted: payload was valid bytes; postcard decoded without error. | Caller-side semantic checks (e.g. `JournalEvent::is_valid()`) still apply. |

### 3.3 The new check is boundary-strict

The trailing-bytes check ensures that no output escapes the parser with a length-ambiguous relationship to the input. Pre-fix, `payload: &[u8]` was returned as `bytes.get(payload_start..payload_end)` regardless of how long `bytes` was — a length-ambiguous contract that downstream code could silently misuse. Post-fix, `payload` is exactly `bytes.len() - RECORD_HEADER_BYTES` (when the input is well-formed), making the contract precise.

## 4. Pure-core / impure-shell split

The codec functions are 100% pure:

- No `fs::*`, no `fjall::*`, no `std::time::*`, no `rand::*`.
- No `unsafe`.
- No interior mutability.
- No thread spawning.
- No `tokio` / `async` / `await`.
- The only side-effect-adjacent operation is `Vec::with_capacity` and `Vec::extend_from_slice` in `encode_record_payload` — pure allocation, no I/O.

The new trailing-bytes check is pure arithmetic (`bytes.len() - payload_end`), no allocation.

## 5. Async-shell boundary

The codec does not run inside the async shell. Fjall reads happen in the imperative shell (`crates/vb_storage/src/lib.rs` etc.), and the resulting `Vec<u8>` is passed by `&[u8]` into the codec. The new check inherits the same async-shell relationship: callers in `trimming/logic.rs:251` etc. await Fjall synchronously per item and call the codec inline.

No new async-shell surface is introduced.

## 6. Storage boundary

| Storage op | Path | Interaction with the fix |
|---|---|---|
| `Fjall::get` | `crates/vb_storage/src/lib.rs` (storage layer) | Returns `Vec<u8>`; caller passes `&[u8]` to codec. Pre-fix, Fjall reads could yield bytes with arbitrary trailing junk (e.g. row corruption). Post-fix, those bytes yield `Err(TrailingBytes)` instead of silently decoding. |
| `Fjall::insert` | `crates/vb_storage/src/lib.rs` | Encoder never produces trailing bytes; no change needed. |
| Snapshot iteration | `crates/vb_storage/src/trimming/logic.rs:251` | Loops over `snap.prefix(...)`. Each item is decoded via `decode_journal_event`. Post-fix, a corrupted item yields `Err` and the loop continues via `?`. |

## 7. Network boundary

No network boundary is touched by this bead. The codec surface is library-internal and is reached by callers within the same process.

## 8. FFI boundary

No FFI. The change is Rust-internal.

## 9. Unsafe / provenance boundary

No `unsafe` is added. No raw pointers. No `MaybeUninit`. The new check is a pure `usize` subtraction and compare.

## 10. Time / clock boundary

The codec does not consult time. No change.

## 11. Randomness boundary

The codec does not use randomness. No change.

## 12. Cross-crate boundaries

| Boundary | Direction | Surface |
|---|---|---|
| `vb_storage` → `vb_core` | outward (re-exports) | `DiagnosticCode`, `SymbolicCode`, `RunId`, `WorkflowDigest`, `WorkflowError`. No change to the boundary; the new `TRAILING_BYTES_CODE` constant is `DiagnosticCode::new(0x4042)`, a value-type construction. |
| `vb_storage` ↔ `fjall` | outward (read/write) | Unchanged. The fix is decoder-side only. |
| `vb_storage` ↔ `postcard` | outward (encode/decode) | Unchanged. The check is *before* `postcard::from_bytes`. |
| `vb_storage` → `verification/verus/` | outward (production-binding) | Verus mirror gains one variant; bridge gains one arm. Drift gate re-checks parity. |

## 13. Boundary artifacts

| Artifact | Path | Role |
|---|---|---|
| Mirror (drift-tracked) | `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:335-413` | Mirrors reachable `JournalError` variants in the `SpecJournalError` enum. Must add `TrailingBytes { trailing: u32 }`. |
| Extern shim | `verification/verus/extern_vb_vzcuf_PS_003.rs:83-87` | Re-exports `SpecJournalError`. No code change; re-export covers the new variant automatically. |
| Bridge | `verification/verus/vb-vzcuf-PS-003.rs:387-451` | `assume_specification[ production::decode_record ]` enumerates reachable Err arms. Must add the `TrailingBytes` arm. |
| Drift gate | `scripts/check-production-inner-drift.sh` | Re-runs after edits; fails on shape mismatch. |
| Production-binding gate | `scripts/check-verus-production-binding.sh` | Re-runs after edits; fails on missing `TrailingBytes` arm in bridge. |
| Production source | `crates/vb_storage/src/codec/payload.rs:56-82`, `crates/vb_storage/src/codec/envelope.rs:48-83` | The two sites that gain the new check. |
| Tests | `crates/vb_storage/src/codec/tests.rs:1498-1524`, `crates/vb_storage/src/codec/envelope.rs:153-170` (sibling), `crates/vb_storage/src/error_tests.rs`, `crates/vb_storage/src/error_code_tests.rs` | Regression locks. |

## 14. Boundary discipline checklist

| Item | Status |
|---|---|
| All untrusted input enters via a parser. | YES (unchanged). |
| Parser output is fully typed. | YES (unchanged). |
| Parser does not perform I/O. | YES (unchanged). |
| Parser does not block on time, channel, or lock. | YES (unchanged). |
| Parser does not spawn tasks. | YES (unchanged). |
| Parser does not use `unsafe`. | YES (unchanged). |
| Parser is idempotent. | YES (unchanged). |
| Parser fail-closes on shape defects. | **NEWLY YES** (pre-fix `bytes.len() > payload_end` slipped through). |
| Parser's failure modes are enumerated in the error taxonomy. | YES (after adding the `TrailingBytes` arm). |
| Parser's failure modes are bound to the production surface via a Verus mirror. | YES (after adding the `SpecJournalError::TrailingBytes` arm). |
| Cross-crate boundaries are minimal. | YES (only `DiagnosticCode::new(0x4042)` touches `vb_core`). |

---

## Summary

The change hardens one functional-core boundary (the codec decoder) and creates one new bridge arm in the Verus mirror. The impure-shell and async-shell boundaries are unchanged; the codec continues to be a pure parser over `&[u8]`. The fix enforces parser boundary discipline by ensuring that no typed output escapes the parser with a length-ambiguous relationship to its input.