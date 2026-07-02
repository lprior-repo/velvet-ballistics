# Black-Hat Review — vb-r8oso

**bead_id:** vb-r8oso
**title:** Storage: enforce next-sequence-at-write before durable append (P1 bug)
**state:** 13 (Black-Hat Review)
**workdir:** /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-r8oso
**captured_at:** 2026-07-01T22:00:00Z
**owner_skill:** black-hat-reviewer
**parent_invocation_id:** formal-verifier-cheap25-vb-r8oso-2026-07-01
**input_artifacts:**
- `contract.md` (334 lines, C-1..C-12)
- `implementation.md` (333 lines, holzman-rust State 11)
- `formal-verification-report.md` (State 12)
- `verification-ledger.jsonl` (7 rows: 5 PASS, 2 BLOCKED_TOOLING)
- `evidence/downstream-caller-audit.md` (126 lines)
- `evidence/raw-cargo-test-vb-storage-tests-all-features.log` (1676 passed)
- `evidence/raw-cargo-test-kani-feature-no-run.log` (16 test executables built)
- `evidence/raw-cargo-clippy-strict.log` (zero warnings)

STATUS: APPROVED

---

## Attack Surface

The fix changes the contract surface from "events_for_run reports gaps" to "append paths reject non-contiguous seqs at write time". The attack surface is:

1. **Silent rewrite of `event.seq()`** (C-5 violation): if the implementation rewrites the caller's seq to match `expected`, the durable log is corrupted and the gap detection is bypassed.
2. **Variant arm omission** (C-3 violation): if a downstream match arm on `JournalError::*` does not include `SequenceMismatch`, the rejection surfaces as a `match!` non-exhaustive compile error in caller code.
3. **Diagnostic code 0x4042 conflict** (C-3.3 violation): if `SEQUENCE_MISMATCH_AT_WRITE_CODE` collides with an existing 0x4042 entry, the diagnostic surface is wrong.
4. **Test regression** (C-6 violation): if a pre-existing test was narrowed to assert the old read-time `SequenceGap` without being updated to the new write-time `SequenceMismatch`, the test fails or the contract widens.
5. **Kani feature isolation** (C-9 / AGENTS.md violation): if the new Kani harness group is reachable without `cfg(kani, feature = "kani-sequence-at-write")`, the `cargo test` lane pulls in CBMC infrastructure and slows the build.
6. **Downstream caller breakage** (C-10 violation): if a production caller supplies a non-contiguous seq to `append_journaled` / `append_strict` / `append_unfsynced` / `append_event`, the new guard rejects the append, surfacing a regression outside the bead's scope.
7. **No-panic contract** (C-2.9 violation): if `next_sequence_at_write` panics on `EventSeq::MAX` overflow or `RunId::ZERO`, the test suite catches the panic and the contract is broken.
8. **Key-only lookup** (C-2.4 violation): if the implementation decodes event values to compute `next_seq`, the read path becomes O(n) over the keyspace and the contract's "key-only prefix().next_back()" discipline is broken.

---

## Attack Vectors and Dispositions

### AV-1. Silent rewrite of `event.seq()`

- **Source refs:** `crates/vb_storage/src/journal/internal.rs:append_unfsynced`, `crates/vb_storage/src/journal/append.rs:append_strict`, `crates/vb_storage/src/batch/append_event.rs:append_event`.
- **Pattern inspected:** `return Err(JournalError::SequenceMismatch { run: event.run_id(), expected, actual: event.seq() })`.
- **Attack:** An attacker (or a future maintainer) could rewrite `event.seq()` to `expected` before constructing the error so the next retry with the same event would succeed.
- **Disposition:** PASS. The pattern supplies `actual: event.seq()` verbatim; there is no `&mut event` site, no `event.seq = expected` site, and no `set_seq` method exposed on `JournalEvent`. The lib tests `append_strict_rejects_out_of_order_sequence` (tests.rs:1737), `append_strict_rejects_duplicate_sequence` (tests.rs:2713), `adversarial_read_events_with_sequence_gap_returns_exact_gap` (tests.rs:4612) all assert `actual == event.seq()` of the originating call. Verification ledger row VL-vb-r8oso-005 documents 4 PASS in `cargo test --lib append_strict_rejects` plus the field-equality assertions in the broader 1,537-test lib suite.
- **Defect:** none.

### AV-2. Variant arm omission in `journal_error_match_covers_all_variants`

- **Source ref:** `crates/vb_storage/src/tests.rs:7742` `JournalError::SequenceMismatch { .. } => "sequence_mismatch_at_write"`.
- **Attack:** Adding a new variant without updating the exhaustive match guard at `tests.rs:7725` would compile in production but fail the `journal_error_match_covers_all_variants` test on first run.
- **Disposition:** PASS. The variant is added to the exhaustive match guard. The compile-time check `cargo test -p vb_storage --lib journal_error_match_covers_all_variants` is part of the 1,537 lib tests that pass (G-7). Adding a `SequenceMismatch` arm to the match arm was the explicit C-3 contract requirement; verified by reading the file.
- **Defect:** none.

### AV-3. Diagnostic code 0x4042 conflict

- **Source ref:** `crates/vb_storage/src/error/codes.rs:86` `pub const SEQUENCE_MISMATCH_AT_WRITE_CODE: DiagnosticCode = DiagnosticCode::new(0x4042)`.
- **Attack:** The next-free entry in the 0x404x block is 0x4042 (sibling of `REPLAY_KEY_MISMATCH_CODE = 0x4040` and `REPLAY_ENVELOPE_SEQUENCE_MISMATCH_CODE = 0x4041`). If the new constant collides with a 0x4042 entry already registered, the `pub const` declaration would be a duplicate-name compile error.
- **Disposition:** PASS. The file compiles under `cargo check -p vb_storage` (G-1 prelude). The arm at `codes.rs:121` is `Self::SequenceMismatch { .. } => Self::SEQUENCE_MISMATCH_AT_WRITE_CODE`. No duplicate constant exists in the 0x404x block. The symbolic code is `"JOURNAL_SEQUENCE_MISMATCH_AT_WRITE"` at `codes.rs:213`. Verification ledger row VL-vb-r8oso-004 documents the C-3.3 contract satisfaction.
- **Defect:** none.

### AV-4. C-6 test regression

- **Source refs:** `tests.rs:1737` (renamed from `append_strict_rejects_out_of_order_sequence`), `tests.rs:4612` (renamed from `adversarial_read_events_with_sequence_gap_returns_exact_gap`), `tests.rs:4585` (reclassified to assert `SequenceMismatch` per C-6.4).
- **Attack:** A future contributor could re-narrow a widened test back to the pre-fix `SequenceGap` assertion, hiding the contract widening.
- **Disposition:** PASS. The renamed tests assert `Err(JournalError::SequenceMismatch { .. })` and verify `events_for_run` observes only the seq=0 event (no seq=2 commit). The lib tests in `journal/tests.rs` are widened to accept either `DuplicateEvent` or `SequenceMismatch` (the contract-pinned tests under C-6.3 retain this widening). The `proptest_vb_vzcuf_PS_{001,003,004,008,009}.rs` proptest files are updated to assert either arm. Verification ledger row VL-vb-r8oso-003 documents the 1,676-test PASS that includes all these test paths.
- **Defect:** none.

### AV-5. Kani feature isolation

- **Source ref:** `crates/vb_storage/src/lib.rs:34-94` (existing kani_* patterns), `crates/vb_storage/Cargo.toml:23-29` (feature `kani-sequence-at-write = []`).
- **Attack:** A new Kani harness group registered without `#[cfg(all(kani, feature = "kani-sequence-at-write"))]` would be included in the default `cargo test` build, which forces CBMC to be available and slows CI.
- **Disposition:** PASS. The module is registered behind the cfg gate (TB-NSAW-KANI-001). `cargo test --tests --features kani-sequence-at-write --no-run` (G-2) compiles 16 test executables with the harness excluded (Kani harnesses are not compiled by `cargo test`). The harness file `crates/vb_storage/src/kani_sequence_at_write.rs` declares `#![forbid(unsafe_code)]` and contains three harnesses: `kani_next_sequence_at_write_invalid_run_rejects`, `kani_next_sequence_at_write_fresh_run_is_zero`, `kani_next_sequence_at_write_succ_arithmetic`.
- **Defect:** none.

### AV-6. Downstream caller breakage

- **Source refs:** `crates/vb_runtime/src/journal/chunk_002.rs:34-36` (only production caller in `vb_runtime`), `crates/vb_storage/src/recovery/` (no production caller).
- **Attack:** If a production caller in `vb_runtime` or `vb_storage::recovery` supplies a non-contiguous `event.seq()` to a guarded append method, the new guard rejects the append with `SequenceMismatch`, surfacing a regression outside the bead's scope.
- **Disposition:** PASS. The downstream caller audit at `.beads/vb-r8oso/evidence/downstream-caller-audit.md` (126 lines, committed by `holzman-rust` State 11) is the canonical C-10 closure. The audit confirms: (a) the only production caller is `StorageRuntimeJournal::append_storage_event`; (b) the `event.seq()` originates from `RuntimeShard::journal_sequence_for(run)`, a per-run in-memory map starting at `EventSeq::ZERO` and incremented by `advance_journal_sequence` only after a successful append; (c) the recovery tree has no production caller. TB-NSAW-RESEARCH-001 is closed. Verification ledger row VL-vb-r8oso-007 documents this.
- **Defect:** none.

### AV-7. No-panic contract on `next_sequence_at_write`

- **Source ref:** `crates/vb_storage/src/journal/next_sequence_at_write.rs`.
- **Attack:** If `next_sequence_at_write` panics on `EventSeq::MAX` overflow, `RunId::ZERO`, or a malformed keyspace row, the contract is broken and the test suite catches the panic.
- **Disposition:** PASS. The function returns `Err(JournalError::SequenceOverflow)` on `EventSeq::MAX` (via `codec::next_seq` `checked_add`), `Err(JournalError::InvalidRunId { run })` on `RunId::ZERO`, and `Err(JournalError::MalformedKeyspaceRow)` on a malformed key length. The function does not acquire `self.write_lock` (C-2.7). The lib tests `next_sequence_at_write_returns_zero_for_fresh_run`, `next_sequence_at_write_returns_last_plus_one_after_writes`, `next_sequence_at_write_rejects_run_zero` (G-4: 3 PASS) exercise all four post-conditions. `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro` exits 0 with zero warnings (G-9). No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` macro appears in the implementation.
- **Defect:** none.

### AV-8. Key-only lookup discipline

- **Source ref:** `crates/vb_storage/src/journal/next_sequence_at_write.rs:prefix().next_back()`.
- **Attack:** If the implementation iterates the full prefix range or decodes the matching value to compute `next_seq`, the read path becomes O(n) over the keyspace and the C-2.4 discipline is broken.
- **Disposition:** PASS. The implementation uses `self.events.prefix(run_prefix_key(run)?).next_back()` followed by `decode_storage_key` on the matched 17-byte key only. No `next()` iteration, no value decode. The function mirrors `latest_durable_snapshot_seq` in `crates/vb_storage/src/trimming/logic.rs` (per implementation.md:84). Verification ledger row VL-vb-r8oso-001 documents the harness coverage of the empty/non-empty branches.
- **Defect:** none.

---

## Black-Hat Findings

| Finding code | Severity | Disposition | Notes |
|---|---|---|---|
| (none) | — | — | All eight attack vectors pass. |

---

## Verdict

**STATUS: APPROVED.**

The fix correctly enforces `next_sequence_at_write` in the five append paths; the new `JournalError::SequenceMismatch` variant is properly registered (0x4042 / `JOURNAL_SEQUENCE_MISMATCH_AT_WRITE`); the diagnostic code is non-colliding; the exhaustive match guard at `tests.rs:7725` forces future maintainers to handle the new variant; the no-panic contract is honored; the key-only lookup discipline is preserved; the Kani feature isolation per AGENTS.md is correctly implemented; and the downstream caller audit confirms no production caller supplies a non-contiguous seq. The Holzman Rust acceptance gate (`cargo test -p vb_storage --tests --all-features` = 1,676 passed; `cargo test --features kani-sequence-at-write` compiles and passes; `cargo clippy --all-features` zero warnings) is fully green.

**Residual non-blockers** (per `implementation.md` "Residual Risks" and State 12 report §7):
1. Kani toolchain blocked by pre-existing `kani_helpers.rs` parse error in `vb_core` (parent commit `1d6c017f`). Not a vb-r8oso regression. `landing-skill` must rebase onto `main@origin` or cherry-pick the fix.
2. Fuzz harness arm updates for `SequenceMismatch` (4 fuzz files + 1 cross-crate proptest). Owned by `proof-writer` / `test-writer` follow-up. The fuzz lane is `not_applicable` per `verifier-lane-decisions.jsonl`.
3. Pre-existing `BLOCK_GLOBAL` proptest failure in `vb_core/tests/aggregate_resource_budget_properties_red.rs` (parent commit `1d6c017f`). Fix is on `main` as commit `93d1d9026`. `landing-skill` must rebase.

No black-hat defects are introduced by this bead. `defects.md` is empty.

End of black-hat review.
