# Hazard Analysis — vb-1wora

**Bead:** `vb-1wora` — Codec: reject trailing bytes after declared record payload (P1 bug)
**Skill:** `rust-contract` (State 3)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-1wora`

This file enumerates the hazards the fix introduces, removes, leaves in place, or fails to address. Hazards are classified by category (temporal, Rust-core invariant, bounded state, refinement, concurrency, unsafe/provenance, hostile input, performance, release/API), and each is tagged with severity, the existing mitigation, and the residual risk after the fix.

---

## 1. Hazard register

Each row is a single hazard. Tags use the format `[cat:severity]` where `cat` ∈ {`TEMP`, `CORE`, `BOUND`, `REFINE`, `CONCUR`, `UNSAFE`, `HOSTILE`, `PERF`, `API`} and `severity` ∈ {`LOW`, `MED`, `HIGH`, `CRIT`}.

### 1.1 Removed by the fix

| ID | Hazard | Tags | Pre-fix description | Post-fix outcome |
|---|---|---|---|---|
| `HAZ-CODEC-TB-001` | Silent acceptance of trailing bytes (P1 bug) | `[CORE:HIGH] [HOSTILE:HIGH] [REFINE:HIGH]` | `decode_record_payload` (and its mirror `decode_envelope_only`) used `bytes.get(payload_start..payload_end)` and discarded the tail. Inputs with extra trailing bytes decoded successfully and the tail was silently dropped. An attacker who could append bytes to a stored record could either (a) confuse the decoder about record boundaries or (b) hide payload mutations in the discarded tail. | The new check returns `Err(TrailingBytes { trailing })`. Decoder fails closed. The tail is no longer silently dropped; the count is reported. |
| `HAZ-CODEC-TB-002` | Decoder behavior divergent from docstring | `[CORE:MED] [API:MED]` | `decode_envelope_only`'s docstring claimed to perform "envelope + payload" validation. Pre-fix it did not detect trailing bytes, contradicting the claim. | The mirror check brings the implementation in line with the docstring. |
| `HAZ-CODEC-TB-003` | Fjall keyspace corruption masked as success | `[HOSTILE:MED] [BOUND:MED]` | `has_terminal_event` (`crates/vb_storage/src/trimming/logic.rs:251`) iterated Fjall values and silently decoded them, dropping any tail. Corrupted rows looked like valid events. | Post-fix, corrupted rows yield `Err(TrailingBytes)` and the loop continues with the diagnostic surfaced. |

### 1.2 Introduced by the fix

| ID | Hazard | Tags | Description | Mitigation |
|---|---|---|---|---|
| `HAZ-CODEC-TB-004` | Verus mirror drift (GOD RULE 2) | `[REFINE:HIGH]` | Adding a new `JournalError` variant without updating the Verus mirror and bridge breaks the production-binding gate (`scripts/check-verus-production-binding.sh`). The drift gate (`scripts/check-production-inner-drift.sh`) would also fail. | Mirror and bridge updates are part of this bead (see `type-contracts.md` §4, `error-taxonomy.md` §8, `boundary-map.md` §13). Gates are re-run before landing. |
| `HAZ-CODEC-TB-005` | Diagnostic-code collision (numeric) | `[API:LOW]` | Choosing `0x4042` must not collide with another constant defined in `vb_storage` or `vb_core`. | Verified free in `codebase-map.md` (codes up to `0x4041` used; `0x4042` is the next slot). Re-verified before landing. |
| `HAZ-CODEC-TB-006` | Test inversion forgotten | `[CORE:MED]` | The existing `decode_ignores_trailing_bytes_beyond_payload` test at `crates/vb_storage/src/codec/tests.rs:1498-1524` asserts the *buggy* behavior. Post-fix it must be renamed and re-asserted. | The rename + inversion is in the delivery scope (`delivery-scope.jsonl` entry 5). The test-writer owns it. |
| `HAZ-CODEC-TB-007` | Round-trip test regression | `[CORE:MED]` | A naive fix could break round-trip tests that encode then decode a fresh record. | The encoder (`encode_record_payload` at `crates/vb_storage/src/codec/payload.rs:34-54`) always produces `bytes.len() == RECORD_HEADER_BYTES + payload_len`, so the new check never fires on round-trip. Tests pass unchanged. |
| `HAZ-CODEC-TB-008` | Symbolic-name not registered in `CODE_REGISTRY` | `[API:LOW]` | If `JOURNAL_TRAILING_BYTES` is added as a string in `symbolic_code()` but not registered in `CODE_REGISTRY`, callers see `SymbolicCode::INTERNAL_INVARIANT` instead of the registered name. | Recommended: register `JOURNAL_TRAILING_BYTES` in `CODE_REGISTRY`. The fallback is non-blocking but degrades observability. |
| `HAZ-CODEC-TB-009` | Decode ordering regression (cheap-before-expensive violated) | `[PERF:LOW] [CORE:MED]` | If the trailing-bytes check is accidentally placed *after* `verify_digest_match`, the digest op runs on every malformed input. | The contract pins the order: step 3 (trailing) MUST be before step 4 (digest). Lint-able via call-site review. The Verus mirror's enumeration comment records the order. |

### 1.3 Unchanged by the fix (residual hazards in the codec surface)

| ID | Hazard | Tags | Description | Why not addressed here |
|---|---|---|---|---|
| `HAZ-CODEC-EXIST-001` | Long-payload BLAKE3 cost | `[PERF:LOW]` | BLAKE3 over `max_payload_len` bytes is the dominant cost. Pre-fix and post-fix, this is unchanged. | Out of scope; the new check actually *reduces* BLAKE3 calls on malformed inputs. |
| `HAZ-CODEC-EXIST-002` | `header.payload_len` field trust | `[CORE:LOW]` | `header.payload_len` is taken from the (CRC32C-validated) header. An attacker who can forge headers can claim any payload length. | Out of scope; addressed by the existing `HeaderChecksumMismatch` arm and the BLAKE3 digest over the payload. The new check sits between these. |
| `HAZ-CODEC-EXIST-003` | Replay of legitimate-looking records | `[HOSTILE:MED]` | A replayed record with a valid header, valid digest, and valid payload will decode correctly. The trailing-bytes check does not detect replays. | Out of scope; replay protection is a different bead. |
| `HAZ-CODEC-EXIST-004` | `UnexpectedEof` vs `TrailingBytes` confusion | `[API:LOW]` | Operators may confuse truncation (`UnexpectedEof`) with extension (`TrailingBytes`). | The Display messages are distinct (`"unexpected end of record"` vs `"trailing bytes after declared payload: N"`); the diagnostic codes are distinct (`0x4014` vs `0x4042`). |
| `HAZ-CODEC-EXIST-005` | Loom-irrelevant (no concurrency) | `[CONCUR:N/A]` | The codec is single-threaded; no concurrency hazards. | N/A; the Loom lane is not required. |
| `HAZ-CODEC-EXIST-006` | No `unsafe` introduced | `[UNSAFE:N/A]` | The new check is pure arithmetic; no `unsafe`, no raw pointers, no `MaybeUninit`. | N/A; the unsafe lane is not required. |

### 1.4 Hazards introduced by *not* doing the fix (still on the table for re-evaluation)

| ID | Hazard | Tags | Description | Status |
|---|---|---|---|---|
| `HAZ-CODEC-NO-FIX-001` | Decoder silently accepts corrupted records | `[CORE:CRIT] [HOSTILE:HIGH]` | As above; the original P1 bug. | **Fixed by this bead.** |
| `HAZ-CODEC-NO-FIX-002` | Doctor scans report false-positive valid events | `[API:MED]` | Doctor surfaces "valid record" for any byte sequence with a valid prefix and trailing junk. | **Fixed by this bead (via `decode_envelope_only` mirror).** |
| `HAZ-CODEC-NO-FIX-003` | No symbolic observability for shape defects | `[API:LOW]` | Pre-fix there is no symbolic code for the silent-acceptance case because there is no error to symbolize. | **Indirectly fixed: the new variant has a registered symbolic code (recommended).** |

## 2. Hazard-by-category roll-up

### 2.1 Temporal (TEMP)

No new temporal hazards. The decode pipeline is single-pass and synchronous. The new check is O(1) and does not introduce loops, retries, or timeouts.

### 2.2 Rust-core invariant (CORE)

| Invariant | Status |
|---|---|
| `decode_record_payload` returns `Err(TrailingBytes { trailing })` iff `bytes.len() > payload_end`. | **NEW**; locked by `INV-CODEC-TB-001`. |
| `decode_record_payload` returns `Ok` only if `bytes.len() == payload_end`. | **NEW**; locked by `INV-CODEC-TB-002`. |
| The check runs before `verify_digest_match` and before `postcard::from_bytes`. | **NEW**; locked by `INV-CODEC-TB-003` and the bridge comment. |

### 2.3 Bounded state (BOUND)

The new check does not affect any bounded-state model. The existing `payload_len_u32` guard at `crates/vb_storage/src/codec/payload.rs:20-32` continues to bound `payload_len` against `max_payload_len`. The new variant's `trailing: usize` is bounded by `bytes.len()`, which is bounded by Fjall row size limits (kernel page size, etc.).

### 2.4 Refinement (REFINE)

| Refinement claim | Status |
|---|---|
| Production `JournalError::TrailingBytes { trailing: usize }` is mirrored by Verus `SpecJournalError::TrailingBytes { trailing: u32 }`. | **NEW**; locked by `INV-CODEC-TB-007` and the drift gate. |
| The bridge `ensures` clause enumerates the new arm with the correct precondition. | **NEW**; locked by the production-binding gate. |

### 2.5 Concurrency (CONCUR)

No concurrency hazards. The codec is single-threaded and `&[u8]`-based. Loom is not required.

### 2.6 Unsafe / provenance (UNSAFE)

No unsafe code added. The new check is a pure `usize` subtraction and compare. Kani's unsafe lane is not required for this bead (the existing `kani_postcard_envelope_wire.rs` H5 harness is `unsafe`-free).

### 2.7 Hostile input (HOSTILE)

The fix is the hostile-input mitigation. Pre-fix, an attacker could append arbitrary bytes to a stored record and the decoder would accept it. Post-fix, the decoder rejects the input.

| Hostile-input claim | Status |
|---|---|
| Inputs with `bytes.len() > payload_end` yield `Err(TrailingBytes)`. | **NEW**; the core fix. |
| Inputs with `bytes.len() < payload_end` continue to yield `UnexpectedEof`. | **Unchanged**. |
| Inputs with `bytes.len() == payload_end` decode normally (including round-trip). | **Unchanged**. |
| Fuzz targets exercise the trailing-bytes path. | **Recommended**: add a "append 0..=8 junk bytes" loop to `fuzz/fuzz_targets/fuzz_storage_codec_payload_corruption.rs`. Not in scope unless proof-planner calls for it. |

### 2.8 Performance (PERF)

| Performance claim | Status |
|---|---|
| The new check is O(1) (one compare, one subtraction). | **NEW**; negligible. |
| The check reduces BLAKE3 calls on malformed inputs. | **NEW**; improvement, not regression. |
| Round-trip cost unchanged. | **Unchanged**; the encoder never produces trailing bytes. |

### 2.9 Release / API (API)

| API claim | Status |
|---|---|
| `JournalError::TrailingBytes { trailing: usize }` is a public-API addition. | **NEW**; backward-compatible (additive variant, no breaking change). |
| `TRAILING_BYTES_CODE = 0x4042` is a public constant. | **NEW**; backward-compatible. |
| Existing tests that round-trip continue to pass. | **Unchanged**. |
| Test `decode_ignores_trailing_bytes_beyond_payload` is renamed and re-asserted. | **NEW**; breaking change at the test-function name level, no production-code breakage. |
| `decode_record_payload` signature unchanged. | **Unchanged**. |
| `decode_envelope_only` signature unchanged. | **Unchanged**. |

## 3. Severity roll-up

| Severity | Count (post-fix) | Notes |
|---|---|---|
| CRITICAL | 0 | The pre-fix P1 bug is gone. |
| HIGH | 0 (post-fix) | The Verus mirror drift hazard (HAZ-CODEC-TB-004) is mitigated by the in-bead mirror update; residual risk is reviewer-driven, not code-driven. |
| MED | 4 | Test inversion (HAZ-CODEC-TB-006), cheap-before-expensive ordering (HAZ-CODEC-TB-009), round-trip regression (HAZ-CODEC-TB-007, mitigated by encoder property), pre-existing replay (HAZ-CODEC-EXIST-003). |
| LOW | 6 | Diagnostic-code collision (mitigated), symbolic registration (optional), pre-existing long-payload cost, header-field trust, operator confusion, etc. |

## 4. Proof lane requirements (recommended)

Hazards imply verifier-lane requirements. These are *recommendations only*; the proof-planner owns final decisions.

| Hazard | Verifier lane |
|---|---|
| `HAZ-CODEC-TB-001` | Kani (bounded panic-freedom over the trailing-bytes path), Verus (production binding + bridge `ensures`), proptest (round-trip + property-based: random byte appends always yield `TrailingBytes` or `Ok`). |
| `HAZ-CODEC-TB-002` | Verus (bridge `ensures` arm for `Ok((env, payload))` requires `bytes.len() == payload_end`). |
| `HAZ-CODEC-TB-003` | proptest (property: random Fjall row keys + appended junk bytes => `Err(TrailingBytes)` from `decode_journal_event`). |
| `HAZ-CODEC-TB-004` | Drift gate `scripts/check-production-inner-drift.sh` and production-binding gate `scripts/check-verus-production-binding.sh`. |
| `HAZ-CODEC-TB-006` | Manual review + test-runner (the inverted test must pass). |
| `HAZ-CODEC-TB-009` | Structural review + Kani harness asserting call ordering (optional). |
| `HAZ-CODEC-NO-FIX-001` | Verus + Kani + proptest (the original bug). |

## 5. Open hazard decisions (carry to planner)

1. **Kani H6 harness for trailing-bytes path.** Recommended. Optional unless proof-planner explicitly adds it. The existing H5 (`kani_harness_digest_before_postcard`) covers the digest-before-postcard ordering; H6 would cover the trailing-before-digest ordering.
2. **Fuzz target update.** Recommended. Optional. The targeted "append 0..=8 junk bytes" loop is additive value.
3. **`JOURNAL_TRAILING_BYTES` registration in `CODE_REGISTRY`.** Recommended. Non-blocking.

---

## Summary

The fix removes one CRITICAL-severity hazard (silent acceptance of trailing bytes) and introduces three low/medium-severity hazards (mirror drift, test inversion, ordering) that are all mitigated within the same bead. The new variant slots cleanly into the shape-defect bucket and inherits the existing infrastructure (Verus mirror, bridge, drift gate, diagnostic-code wiring). No concurrency, unsafe, or temporal hazards are introduced.