# Trusted Base Plan — vb-1wora

This plan enumerates the surfaces that the proof obligations (POB-vb-1wora-001..007) trust without further proof. Each trusted surface has a concrete justification (production-binding gate, drift gate, compile-time invariant, or external-crate guarantee) so the reviewer can audit whether the trust is justified.

The seven obligations' "trusted_base_refs" cite this plan by ID (`TBP-VB-1WORA-001` etc.).

---

## 1. Trusted Surfaces

### TBP-VB-1WORA-001 — Production source `decode_record_payload` is well-typed and total (trusted — type system + existing tests)

- **Surface:** `crates/vb_storage/src/codec/payload.rs:56-82` `pub(crate) fn decode_record_payload(bytes: &[u8], expected_magic: u32, max_payload_len: u32) -> Result<(RecordEnvelope, &[u8]), JournalError>`.
- **Justification:**
  - The function signature is `pub(crate)` and its inputs are well-typed primitives (`&[u8]`, `u32`, `u32`); no raw pointers, no `MaybeUninit`.
  - The crate is `#![forbid(unsafe_code)]` (workspace-level invariant).
  - Existing pre-fix tests at `crates/vb_storage/src/codec/tests.rs` (~50+ tests) already exercise every code path on well-formed and intentionally malformed inputs; none of those tests are modified by this bead.
  - The existing Kani H1-H5 harnesses at `crates/vb_storage/src/kani_postcard_envelope_wire.rs` already prove panic-freedom on similar header/payload shapes (proven via prior beads and the existing CI gate).

### TBP-VB-1WORA-002 — Production source `decode_envelope_only` is well-typed and total (trusted — type system + mirror symmetry)

- **Surface:** `crates/vb_storage/src/codec/envelope.rs:48-83` `pub(crate) fn decode_envelope_only(bytes: &[u8]) -> Result<(RecordEnvelope, &[u8]), JournalError>`.
- **Justification:**
  - Same as TBP-VB-1WORA-001: well-typed `pub(crate)` fn, no `unsafe`, no raw pointers.
  - The function is `#[allow(dead_code, reason = "inspection-only entry point retained for doctor/filtering workflows")]`, but it is `pub(crate)` and called by tests at `crates/vb_storage/src/codec/envelope.rs:153-170` (pre-fix `decode_envelope_only_rejects_truncated_payload`).
  - The mirror invariant INV-CODEC-TB-004 is locked by adding the same trailing-bytes check; structural symmetry with `decode_record_payload` is enforced by the contract.

### TBP-VB-1WORA-003 — `JournalError` enum is well-formed (trusted — type system)

- **Surface:** `crates/vb_storage/src/error/mod.rs:21-188` `pub enum JournalError { ... }`.
- **Justification:**
  - The enum is derived `#[derive(Debug, thiserror::Error)]`; each variant is either a unit, a tuple, or a struct.
  - The new `TrailingBytes { trailing: usize }` variant follows the same template as the existing `MalformedKeyspaceRow { actual_len: usize, expected_len: usize }` at lines 97-105 (already cited as the precedent in `contracts/error-taxonomy.md §1.4`).
  - Rust's enum semantics guarantee that no two variants share the same discriminant; pattern matches are exhaustive by construction.

### TBP-VB-1WORA-004 — `DiagnosticCode::new(0x4042)` is a valid distinct constant (trusted — type system + numeric uniqueness)

- **Surface:** `pub const TRAILING_BYTES_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);` in `crates/vb_storage/src/error/codes.rs:~50`.
- **Justification:**
  - `DiagnosticCode` is a `pub struct DiagnosticCode(pub u16)` (or similar wrapper) defined in `crates/vb_core/src/diagnostic.rs`; the `.0` field is a `u16`, and `0x4042` is well within `u16::MAX`.
  - Numeric uniqueness is locked by `codebase-map.md` (highest used constant is `0x4041`; `0x4042` is the next free slot in the `0x40xx` journal range). Re-verified by the test `trailing_bytes_error_has_correct_code` (POB-vb-1wora-002).
  - Cross-crate collision check: `crates/vb_core/src/diagnostic.rs::CODE_REGISTRY` registry stops at `0x4032` per `codebase-map.md`; `0x4040`/`0x4041` are journal codes defined in `vb_storage` but not yet registered symbolically. `0x4042` is verified free.

### TBP-VB-1WORA-005 — Kani symbolic execution bounds are sufficient (trusted — Kani toolchain)

- **Surface:** `cargo kani -p vb_storage --harness kani_harness_rejects_trailing_bytes --output-format=json`.
- **Justification:**
  - Kani is a bounded model checker; it explores all reachable states up to `#[kani::unwind(N)]` for loops/recursion.
  - The H6 harness inherits `#[kani::unwind(4)]` from H5 (`crates/vb_storage/src/kani_postcard_envelope_wire.rs:271-337`) which already exhaustively proves digest-before-postcard ordering on the same header/payload shape. The trailing-bytes path adds only an `if` + `Err` return — no new loop, no new recursion, no new heap allocation. Unwind bound 4 is sufficient.
  - `kani::any()` over a 60-byte header + arbitrary `payload_len ∈ [0, MAX_JOURNAL_EVENT_PAYLOAD_BYTES]` + 1..=8 trailing bytes is bounded by the constant pool of valid header shapes (finite) and the `payload_len` symbolic bound (one symbolic `u32`).
  - Workspace-level guarantee: Kani is installed in CI (per AGENTS.md Verifier Tooling Runbook) and is used by every prior `vb_storage` proof artifact.

### TBP-VB-1WORA-006 — `production_inner/vb_vzcuf_PS_003_production.rs` mirror reflects production (trusted — drift gate)

- **Surface:** `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs` (mirror of `crates/vb_storage/src/codec/mod.rs:1-100`, `codec/payload.rs`, `codec/header.rs`, `codec/kind_parity.rs`).
- **Justification:**
  - The mirror is the established pattern for the PS-003 spec (existing bridge for `decode_record` already enumerates 14 reachable Err variants via `assume_specification[ production::decode_record ]` at `verification/verus/vb-vzcuf-PS-003.rs:387-451`).
  - `scripts/check-production-inner-drift.sh` is the drift-detection gate; it runs in CI and fails on any divergence between the production source and the mirror.
  - The mirror header at `verification/verus/production_inner/vb_vzcuf_PS_003_production.rs:1-26` explicitly documents the regeneration policy and the binding ledger (lines 63-95).
  - Adding `SpecJournalError::TrailingBytes { trailing: u32 }` to the mirror's enum is mechanical (the variant shape is identical to the production `TrailingBytes { trailing: usize }` modulo the `usize → u32` cast, which is bounded by `bytes.len() <= u32::MAX as usize`).

### TBP-VB-1WORA-007 — Verus toolchain is installed and verified (trusted — Verus toolchain)

- **Surface:** `bash scripts/verify-verus.sh`, `bash scripts/check-verus-production-binding.sh`.
- **Justification:**
  - Verus toolchain (0.2026.05.05 / Rust 1.95.0) is installed in CI per AGENTS.md Verifier Tooling Runbook.
  - `scripts/verify-verus.sh` is the registry-driven obligations runner (established pattern, used by every prior `vb_storage` Verus spec).
  - `scripts/check-verus-production-binding.sh` is the production-binding gate; it walks every spec with `proof fn`, extracts the `#[path = "..."]` target, and rejects any spec without a `#[path = ".../crates/..."` or `#[path = ".../production_inner/..."]` binding. The PS-003 spec is already compliant via the existing `extern_vb_vzcuf_PS_003.rs` extern shim (`#[path = "production_inner/vb_vzcuf_PS_003_production.rs"] mod production_inner;`).

### TBP-VB-1WORA-008 (auxiliary) — proptest toolchain is installed (trusted — proptest toolchain)

- **Surface:** `cargo test -p vb_storage --features proptest --lib ...`.
- **Justification:**
  - `proptest` is already in `vb_storage`'s `[dev-dependencies]` (used by existing proptest properties; this bead does not add a new dependency).
  - Workspace-level guarantee: proptest is the standard property-test crate for Rust in this workspace.

### TBP-VB-1WORA-009 (auxiliary) — cargo-fuzz toolchain is installed (trusted — nightly + cargo-fuzz)

- **Surface:** `cargo +nightly fuzz run -p vb_storage_fuzz fuzz_target_trailing_bytes -- -max_total_time=60`.
- **Justification:**
  - `cargo-fuzz` (libFuzzer wrapper) is already used by existing fuzz targets at `fuzz/fuzz_targets/`.
  - Nightly toolchain is installed in CI per AGENTS.md (the `rust-toolchain.toml` pins the version; cargo-fuzz requires nightly).
  - **Fallback:** if `cargo +nightly fuzz` is unavailable in a given CI run, POB-vb-1wora-007's status degrades to `blocked_tooling`; POB-vb-1wora-002 (directed cargo-test) and POB-vb-1wora-004 (Kani H6 over arbitrary trailing bytes 1..=8) still cover the invariant.

---

## 2. Model Reductions and Assumptions

### 2.1 No Concurrency

The codec is single-threaded synchronous: `decode_record_payload` and `decode_envelope_only` are pure parsers over `&[u8]` with no shared state. There are no `Arc`, `Mutex`, atomic, or `Send`/`Sync` boundary crossings in the post-fix diff. The crate-level `#![forbid(unsafe_code)]` and the absence of `std::sync` imports in `codec/*` modules are the trust anchors.

**Implication:** Loom is not required; POB-vb-1wora-007 records this in `verifier-lane-decisions.jsonl:VLD-vb-1wora-007-loom`.

### 2.2 No unsafe / No UB

The post-fix trailing-bytes check is a pure `usize` compare + subtraction:

```rust
if bytes.len() > payload_end {
    return Err(JournalError::TrailingBytes { trailing: bytes.len() - payload_end });
}
```

There is no raw pointer, no `MaybeUninit`, no aliasing. The `TrailingBytes { trailing: usize }` variant contains only a `usize` field. Miri would find no UB paths; POB-vb-1wora-008 records this in `verifier-lane-decisions.jsonl:VLD-vb-1wora-008-miri`.

### 2.3 No Refinement Types

The `trailing > 0` invariant is enforced structurally at the producer site (`bytes.len() > payload_end` mathematically implies `bytes.len() - payload_end > 0`). No refinement type is introduced. The Verus mirror handles the refinement claim at the bridge level (POB-vb-1wora-006); Flux RS would add no coverage.

**Implication:** Flux RS is not required; POB-vb-1wora-009 records this in `verifier-lane-decisions.jsonl:VLD-vb-1wora-009-flux`.

### 2.4 No Temporal / State-Machine Behavior

The decode pipeline is single-pass synchronous. The new variant is a one-shot failure arm in a pure function; no observable state across calls. TLA+ was explicitly removed from the proof-planner skill (proof-planner doctrine: "TLA+ removed. The temporal-workflow shape uses loom + proptest."); POB-vb-1wora-010 records this in `verifier-lane-decisions.jsonl:VLD-vb-1wora-010-tla-plus`.

### 2.5 Kani Bounded Exploration

The H6 harness exhaustively explores:

- `valid_magic: u32` — `kani::any()` over all `u32` values.
- `payload_len: u32` — `kani::any()` over all `u32` values, bounded by `MAX_JOURNAL_EVENT_PAYLOAD_BYTES` via `kani::assume`.
- Header fields (`schema_version`, `record_kind`, `sequence`, `payload_digest`, CRC) — constructed deterministically (no `kani::any()`) to ensure header validity.
- Trailing bytes — 1..=8 concrete bytes (not symbolic) so the harness focuses on the trailing arithmetic.

The bounded exploration is justified because:

1. The decoder has no loop on the trailing-bytes path (the `if bytes.len() > payload_end` early-returns).
2. The trailing-bytes check is the only new code path; the digest/postcard steps are downstream and not reachable when `bytes.len() > payload_end`.

---

## 3. Reduction Justifications

| Reduction | Justification |
|---|---|
| The H6 Kani harness exhausts `1..=8` trailing bytes (concrete) instead of symbolic | The trailing-byte count is the only field that matters for the invariant; symbolic bytes would only delay the proof. Concrete 1..=8 covers the most common attacker shapes (1-byte, 4-byte, 8-byte appends) without exhausting Kani's unwinding budget. |
| The proptest property uses 1024 cases per property | Standard proptest budget; sufficient to expose any non-trivial regression. Shrinking is enabled, so any counterexample is shrunk to minimal failing case. |
| The cargo-fuzz budget is 60 seconds | AGENTS.md fuzz discipline: "Every speed claim requires real baseline/result benchmark evidence; fuzz budgets are bounded to avoid melting CI." 60 seconds is sufficient to reach the TrailingBytes arm given the encode-record + append-N-oracle shape. |
| The Verus bridge arm uses `expected_payload_end` as a top-level bridge parameter | Per `contracts/type-contracts.md §4.3`: the planner may extend `SpecRecordEnvelope` to carry `payload_len`, OR hoist `expected_payload_end` as a top-level bridge parameter. The latter is the minimum-fuss template (no spec surface change beyond the new arm). |

---

## 4. Non-Behavior Waivers

**No waivers requested.** All 7 planned obligations are behavior-affecting (`behavior_affecting: true`) and must be proven through their assigned verifier lanes. No tooling gaps, legacy constraints, dependency issues, or acceptable risk trade-offs merit a waiver at planning time. The `waiver-candidates.jsonl` file documents the four non-applicable lanes (loom, miri, flux, tla-plus) with concrete evidence; these are NOT waivers in the E_BEHAVIOR_WAIVER sense — they are cross-cutting lane non-applicability statements.

---

## 5. Trust Summary

| Trusted Base ID | Surface | Justification | Cited by |
|---|---|---|---|
| TBP-VB-1WORA-001 | `decode_record_payload` (production) | Well-typed pure fn; crate `#![forbid(unsafe_code)]`; existing tests + Kani H1-H5 | POB-001, POB-002, POB-003, POB-004, POB-005, POB-006, POB-007 |
| TBP-VB-1WORA-002 | `decode_envelope_only` (production) | Well-typed `pub(crate)` fn; mirror symmetry with `decode_record_payload` | POB-002, POB-005 |
| TBP-VB-1WORA-003 | `JournalError` enum | `thiserror`-derived; well-formed enum with primitive fields | POB-002, POB-006 |
| TBP-VB-1WORA-004 | `DiagnosticCode::new(0x4042)` | u16 wrapper; numeric uniqueness verified | POB-001, POB-002, POB-006 |
| TBP-VB-1WORA-005 | Kani toolchain + unwind bound | Established CI toolchain; H5 proven at unwind(4) | POB-004 |
| TBP-VB-1WORA-006 | `production_inner` mirror | Drift gate `scripts/check-production-inner-drift.sh` enforces parity | POB-006 |
| TBP-VB-1WORA-007 | Verus toolchain + binding gate | Established CI toolchain; production-binding gate enforces bridge parity | POB-006 |
| TBP-VB-1WORA-008 | proptest toolchain | In dev-dependencies; standard property-test crate | POB-003, POB-005 |
| TBP-VB-1WORA-009 | cargo-fuzz toolchain (nightly) | Established CI toolchain; fallback to `blocked_tooling` if unavailable | POB-007 |