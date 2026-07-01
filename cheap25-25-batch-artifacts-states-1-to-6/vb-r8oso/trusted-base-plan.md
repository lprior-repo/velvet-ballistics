# Trusted Base Plan — vb-r8oso

This plan enumerates every trust marker that any proof obligation in
`proof-obligations.planned.jsonl` raises. Each marker is a typed
property the proof assumes; failure of any marker invalidates the
proof that depends on it. The `proof-writer` (State 5) maintains the
ledger; the `formal-verifier` (State 12) verifies the ledger against
the run-time evidence.

| Marker | Type | Description | Required by | Evidence path |
|---|---|---|---|---|
| **TB-NSAW-001** | Type-level contract | The new method signature `pub fn next_sequence_at_write(&self, run: RunId) -> Result<EventSeq, JournalError>` is the canonical signature adopted by `public_api::next_sequence_at_write` (C-2.1, C-2.8). The variant `JournalError::SequenceMismatch { run: RunId, expected: EventSeq, actual: EventSeq }` is the canonical variant (C-3.1). Both are checked at compile time by `cargo check -p vb_storage`. | POB-001, POB-002, POB-003, POB-004, POB-005, POB-006, POB-007 | `contract.md` C-2.1 / C-3.1; `domain-model.md` §1; `type-contracts.md` §2.1 |
| **TB-NSAW-002** | Runtime seam | `codec::next_seq(seq)` (at `crates/vb_storage/src/codec/mod.rs:153`) maps `EventSeq::MAX` to `Err(JournalError::SequenceOverflow)` via `checked_add`. The Kani harness for `next_sequence_at_write` depends on this seam, not on a hand-rolled `+1`. | POB-001, POB-003 | `crates/vb_storage/src/codec/mod.rs:153` |
| **TB-NSAW-003** | Build surface | `crates/vb_storage/src/lib.rs:1` is `#![forbid(unsafe_code)]`; the new method is pure Rust with no FFI, no raw pointer arithmetic, no provenance-sensitive operations. Miri is therefore `not_applicable`. | (lane-decision evidence: miri not_applicable) | `crates/vb_storage/src/lib.rs:1` |
| **TB-NSAW-004** | Single-process boundary | `FjallJournal` is single-process; the `write_lock` serialises all five append paths. The codebase-map §2 certifies this and excludes cross-process multi-writer. Loom is therefore `not_applicable` (research-gated). | (lane-decision evidence: loom not_applicable) | `codebase-map.md` §2 |
| **TB-NSAW-RESEARCH-001** | RESEARCH_REQUIRED | **Status: open until holzman-rust closes the audit.** The downstream caller audit (contract C-10 / `domain-model.md` ODQ-1) MUST be performed and reported before any of POB-002, POB-003, POB-005, POB-006 close. The audit MUST grep `append_journaled\|append_strict\|append_unfsynced\|append_event` across `crates/vb_runtime` and `crates/vb_storage::recovery` and report every caller. If a caller supplies an `event.seq()` not derived from a fresh per-run counter, the contract widens per `domain-model.md` ODQ-1 and this plan returns to State 3. | POB-002, POB-003, POB-005, POB-006 | (artifact path TBD: `.beads/vb-r8oso/audit-downstream-callers.md`) |
| **TB-NSAW-KANI-001** | Kani feature isolation | The new Kani harness group is compiled only when `cfg(all(kani, feature = "kani-sequence-at-write"))` holds. Default `cargo test` does NOT pull in the harness. The Cargo feature is declared `kani-sequence-at-write = []` and the module is registered behind the cfg-gate. | POB-001, POB-002, POB-006 (compile check) | `crates/vb_storage/Cargo.toml:23-29`; `crates/vb_storage/src/lib.rs:34-94` (existing kani_* patterns) |
| **TB-NSAW-CODE-001** | Symbolic code registration | `SymbolicCode::JOURNAL_SEQUENCE_MISMATCH_AT_WRITE` is registered in `SymbolicCode::CODE_REGISTRY`, or the INTERNAL_INVARIANT fallback is acceptable for v1 per C-3.4. The proptest exhaustiveness harness (POB-004) accepts either outcome. | POB-004 | `crates/vb_storage/src/error/codes.rs` (existing 0x404x block) |
| **TB-NSAW-FUZZ-001** | Fuzz arm updates | The four existing fuzz arm lists (`fuzz/fuzz_targets/journal_decode.rs:126`, `fuzz/fuzz_targets/decode_record.rs:119`, `fuzz/src/journal_target/errors.rs:46`, `fuzz/tests/proptest_journal_error_exhaustiveness.rs:106`) receive a `JournalError::SequenceMismatch { .. }` match arm under `holzman-rust` per `delivery-scope.jsonl:19-22`. The fuzz lane is `not_applicable` for this bead (no new fuzz harness), but the arm updates are part of the holzman-rust delivery. | (delivery-scope item 19-22) | `fuzz/fuzz_targets/journal_decode.rs:126`; `fuzz/fuzz_targets/decode_record.rs:119`; `fuzz/src/journal_target/errors.rs:46`; `fuzz/tests/proptest_journal_error_exhaustiveness.rs:106` |

## Trust Marker Failure Handling

If any trust marker fails at run time, the corresponding proof
obligation MUST be re-planned. Specifically:

- **TB-NSAW-001** failure (signature change) → the plan returns to
  State 3; contract C-2.1 and C-3.1 must be re-emitted by
  `rust-contract`.
- **TB-NSAW-002** failure (`codec::next_seq` change) → the plan
  returns to State 5; the Kani harness for POB-001 must be re-written
  to use the new helper.
- **TB-NSAW-003** failure (`unsafe_code` introduced) → Miri becomes
  a required lane; POB-001 and POB-002 must be re-planned with Miri
  obligations and the lane-decision `not_applicable` for Miri is
  revoked.
- **TB-NSAW-004** failure (cross-process multi-writer enabled) → Loom
  becomes a required lane; POB-002, POB-005, POB-006 must be re-planned
  with Loom obligations and a sync-indirection harness.
- **TB-NSAW-RESEARCH-001** failure (audit finds a non-conforming
  caller) → the contract widens; the plan returns to State 3; the
  `next_sequence_at_write` invariant is narrowed to admit the
  exception class.
- **TB-NSAW-KANI-001** failure (Kani module reachable without
  feature) → AGENTS.md kani-harness-isolation rule violated; the
  feature gate must be tightened; POB-006 compile-check fails.
- **TB-NSAW-CODE-001** failure (CODE_REGISTRY rejects the new
  symbolic code AND the INTERNAL_INVARIANT fallback is not
  acceptable) → POB-004 fails; the symbolic code must be registered
  in `SymbolicCode::CODE_REGISTRY`.
- **TB-NSAW-FUZZ-001** failure (a fuzz arm does not receive the
  match arm update) → the fuzz exhaustiveness proptest at
  `fuzz/tests/proptest_journal_error_exhaustiveness.rs:106` will
  panic on a `SequenceMismatch` value; the holzman-rust delivery is
  incomplete.

## Trust Marker Coverage

Every proof obligation in `proof-obligations.planned.jsonl` has
non-empty `trusted_base_refs` where assumptions exist; see the
`trusted_base_refs` field on each POB row. Specifically:

- POB-001 (kani, next_sequence_at_write): TB-NSAW-001, TB-NSAW-002, TB-NSAW-003
- POB-002 (kani, append_unfsynced guard): TB-NSAW-001, TB-NSAW-003, TB-NSAW-RESEARCH-001
- POB-003 (proptest, random valid/invalid appends): TB-NSAW-001, TB-NSAW-002, TB-NSAW-RESEARCH-001
- POB-004 (proptest, error taxonomy exhaustiveness): TB-NSAW-001
- POB-005 (proptest, no-silent-rewrite): TB-NSAW-001
- POB-006 (proptest, batch atomicity + Kani isolation): TB-NSAW-001, TB-NSAW-004, TB-NSAW-RESEARCH-001
- POB-007 (proptest, downstream caller audit): TB-NSAW-001, TB-NSAW-004, TB-NSAW-RESEARCH-001

Coverage is bidirectional: every `assumptions` entry on every POB
has a corresponding `trusted_base_refs` entry, and every
`trusted_base_refs` entry maps to a row in this plan. The
`proof-writer` (State 5) is responsible for producing the
`trusted-base-ledger.jsonl` rows for each of these markers.
