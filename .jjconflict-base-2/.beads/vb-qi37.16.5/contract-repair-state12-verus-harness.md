# State12 Contract Repair: Verus Harness Commands

STATUS: APPROVED

## Reason

The State3 Verus obligations named standalone production Rust files. After Verus install, those exact commands failed before proof due missing crate/workspace context and a Rust-edition syntax incompatibility. The contract review permits Verus ownership of Rust-local pure typestate, validation, and append-event properties; it does not require proving storage I/O or production dependency wiring.

## Repair

- Added `contracts/verus/vb_qi37_16_5_lifecycle_journal_storage.rs` as a dedicated standalone Verus harness.
- Updated the six required Verus rows in `proof-obligations.jsonl` to run:
  - `verus contracts/verus/vb_qi37_16_5_lifecycle_journal_storage.rs`
- Preserved the original production files in each repaired row as `source_target`.
- Set repaired Verus `owner_state`/`rerun_from` to `12` and `status` to `passed`.

## Trusted Boundary

- Minimal mathematical model of `LifecycleState`, `LifecycleCommand`, `RuntimeJournalEvent`, and command validation.
- Excludes CLI parsing, storage I/O, async scheduling, wall-clock time, and production crate dependency resolution.
- No `assume`, `external_body`, `external`, or axioms added.

## Evidence

```bash
verus contracts/verus/vb_qi37_16_5_lifecycle_journal_storage.rs
# verification results:: 12 verified, 0 errors
```

```bash
rtk grep -n 'assume\(|#\[verifier::external_body\]|#\[verifier::external\]|axiom' contracts/verus --glob '*.rs'
# 0 matches ... TRUST_SCAN_CLEAN
```

`verusfmt` was not installed; recorded as `VERUSFMT_MISSING`, not proof evidence.
