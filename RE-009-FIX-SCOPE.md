# RE-009 Cascading Fix Checklist — bead vb-loa3o

## Background
Bead vb-loa3o added `JournalEvent::WaitResolvedEvent` + `RecordKind::WaitResolved = 31`. The implementer updated the immediate codec tests (`replay_integrity.rs`, `kill_kind_admission.rs`) but missed 11 other files that hardcode the OLD record-kind range (`1|2|3|10..=29|30|40|50`) — they treat kind 31 as unknown. With the new code admitting kind 31, those tests/proofs/bdd scenarios fail.

## Old range (must change in 11 files)
```
matches!(kind, 1 | 2 | 3 | 10..=29 | 30 | 40 | 50)        // known
matches!(kind, 10..=29)                                    // journal
```

## New range (kind 31 is now journal-admitted)
```
matches!(kind, 1 | 2 | 3 | 10..=29 | 30 | 31 | 40 | 50)   // known (add 31)
matches!(kind, 10..=29 | 31)                               // journal (add 31)
```

## Files to update (in this worktree: /home/lewis/src/velvet-ballistics/.worktrees/vb-loa3o)

### Production source — already updated, but verify symbol references
1. `crates/vb_storage/src/records.rs:161` — `WaitResolved = 31` ✓ already done
2. `crates/vb_storage/src/events.rs` — WaitResolvedEvent variant + 5 match arms ✓ already done
3. `crates/vb_storage/src/codec/validation.rs:24,46` — replace magic `31` with `RecordKind::WaitResolved.id()` (currently hardcoded `10..=29 | 31`). Production code.
4. `crates/vb_storage/src/codec/flux_validation.rs:12-15,27,29-31` — flux model `model_is_known_record_kind` + journal-magic allowlist — must add 31 to both.

### Kani harnesses (proofs will refute with current code)
5. `crates/vb_storage/src/kani_record_kind.rs:179-187` (`check_unknown_kind_rejected`) — currently asserts kind=31 must Err. CHANGE: change test to use kind=32 (still unknown) and add a new positive test for kind=31.
6. `crates/vb_storage/src/kani_record_kind.rs:202-230` (`check_journal_family_exhaustive`) — assert range `10..=29` only. CHANGE: update range to `10..=29 | 31`.
7. `crates/vb_storage/src/kani_storage_invariants.rs:240` — `matches!(kind_id, 1 | 2 | 3 | 10..=29 | 30 | 40 | 50)`. ADD `31`.
8. `crates/vb_storage/src/kani_typed_partitioned_ids.rs:32` — same pattern. ADD `31`.
9. `crates/vb_storage/src/kani_vb_u8gi_storage_decode_order.rs:23,31,92,104,106,144,173` — both `1|2|3|10..=29|30|40|50` and `10..=29` need 31 added.
10. `crates/vb_storage/src/kani_vb_u8gi_storage_numeric_fields.rs:44` — same pattern. ADD `31`.
11. `crates/vb_runtime/src/verification/kani/kani_cancel_kill_lattice.rs:116,124,152,156` — `10..=29` range. ADD `31`.

### Proptests
12. `crates/vb_storage/src/proptest_storage.rs:138-166` (`record_kind_id_roundtrip`) — match arm missing 31. ADD `31 => RecordKind::WaitResolved`.
13. `crates/workspace_tests/tests/cancel_kill_lattice_props.rs:21` — comment block referring to old range. Update comment.

### BDD scenarios
14. `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:55` (`is_unknown_kind`) — match missing 31. ADD `31` is no longer unknown. ALSO line 466-496 (`is_known_record_kind_returns_false_for_invalid_kinds`) asserts kinds [31,32..=39] must Err — REMOVE 31 from that array (it's now valid).
15. `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:467` — comment says `{1,2,3,10-29,30,40,50}`. Update.
16. `crates/workspace_tests/tests/restate_typed_partitioned_id_tests.rs:19` (`unknown_kind`) — match missing 31. ADD. Proptest at lines 58-67 also relies on this — verify it still works after the change.
17. `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs:458,492` — comments describe old set. Update comments to include 31 in known set.

### Other tests (no change needed but verify)
- `crates/workspace_tests/tests/proptest_error_types_nonzero_codes.rs:292` — uses 0xFF, fine.
- `crates/workspace_tests/tests/restate_decode_error_taxonomy_tests.rs:64,97` — uses kinds 6, 8, 9999, fine.
- `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs:235-253` — uses kind 9999, fine.

## Test gates (must pass)
```bash
cargo build -p vb_storage -p vb_runtime -p vb_core
cargo test -p vb_storage --lib
cargo test -p vb_runtime --lib
cargo test -p vb_core --lib
cargo test -p workspace_tests --test vb_eepg_bdd_tests
cargo test -p workspace_tests --test restate_typed_partitioned_id_tests
cargo test -p workspace_tests --test cancel_kill_lattice_props
cargo test -p workspace_tests --test restate_storage_blackhat_fixture_corpus
cargo test -p workspace_tests --test proptest_error_types_nonzero_codes
cargo test -p workspace_tests --test restate_doctor_storage_scan_decode_tests
cargo test -p workspace_tests --test restate_decode_error_taxonomy_tests
bash scripts/forbidden-scan.sh
bash scripts/check-panic-surface.sh
bash scripts/guard-zero-tests.sh -- cargo test -p vb_storage --lib
bash scripts/guard-zero-tests.sh -- cargo test -p vb_runtime --lib
```

DO NOT run `cargo kani` (out of scope; orchestrator runs that). The point of the fix is to make the Kani harnesses consistent with the new range so they WOULD pass if run.

## Also: add the roundtrip test the reviewer flagged
- `crates/vb_storage/src/codec/tests.rs:1009` area — add `kind_31_wait_resolved_record_roundtrip` mirroring the existing RetryScheduled roundtrip. Just encode + decode + assert equal. Test that kind 31 wire format works end-to-end.

## Also: add suspension non-inflation invariant test
- `crates/vb_storage/src/recovery/tests.rs` near line 990 — add `wait_resolved_event_does_not_inflate_suspension_count` asserting `summarize_events(&[WaitResolvedEvent]).suspensions == 0`.

## Style reminders
- No `unsafe`, no `unwrap`, no `expect`, no `panic`, no `todo!`, no `dbg!`.
- Use the typed `RecordKind::WaitResolved.id()` instead of the magic number `31` wherever the production source allows.
- Match exhaustiveness: when adding 31 to `matches!` allowlists, do not weaken type safety.
- Do NOT run `cargo kani`, `cargo flux`, `cargo mutants`, or `moon ci`.

## Workflow
1. cd into /home/lewis/src/velvet-ballistics/.worktrees/vb-loa3o (worktree already exists, branch already committed with the partial fix at commit 6da405631)
2. Read the checklist files in order
3. Apply each fix
4. Run the test gates above
5. git add -A; git commit -m "bead vb-loa3o: extend record-kind range to admit 31 in 11 surrounding files"
6. Return: commit SHA, diff stat, gate evidence (which tests pass), 3-sentence summary.