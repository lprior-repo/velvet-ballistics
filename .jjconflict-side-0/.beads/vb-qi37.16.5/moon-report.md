bead_id: vb-qi37.16.5
phase: state-8
status: PASS_AFTER_REPAIR

# State 8 Machine Gate Report

Initial State 8 run failed at formatting (`rtk cargo fmt -- --check`) after State 6 replay repair. The failure was routed to `holzman-rust` and repaired in `state-8-format-repair.md`.

## Verified gates after repair

```text
rtk cargo fmt                                  PASS
rtk cargo fmt -- --check                       PASS
rtk cargo test --package velvet_ballistics --test lifecycle_integration -- --test-threads=1
  cargo test: 43 passed (1 suite, 0.61s)
moon run :quick                                PASS
moon run :test
  9894 tests run: 9894 passed, 0 skipped
```

## Notes

The State 8 repair also fixed downstream compile/check failures surfaced by `moon run :test`: unused variables in `lifecycle_integration.rs` and missing UI match arms for new journal lifecycle events.

## State 15 landing preflight repair — 2026-05-12

Workspace-only rebase and preflight repair completed in `/home/lewis/src/Velvet-ballistics-vb-qi37-16-5-go`.

```text
jj git fetch                                                        PASS (Nothing changed)
jj rebase -s @ -d main                                             PASS, conflicts resolved locally
rtk cargo fmt --all                                                PASS
rtk cargo test --package velvet_ballistics --test lifecycle_integration -- --test-threads=1
  PASS: 43 passed (1 suite, 1.82s)
rtk cargo test --package vb_storage --doc inject_seq_gap
  PASS: 1 passed (1 suite, 0.00s)
moon ci                                                            PASS: 19 completed (1 cached), 0 failed
```

Repairs applied:

- Rebased onto `main` `c9939431 landing: merge landable vb-jkrk wave3 qi37.16.3`.
- Resolved rebase conflicts in `Cargo.lock`, `crates/vb_core/src/errors.rs`, `crates/velvet_ballistics/src/lib.rs`, `fuzz/fuzz_targets/decode_record.rs`, and `xtask/src/main.rs`.
- Fixed `FjallJournal::inject_seq_gap` doctest scope and made journal corruption test hooks return typed errors instead of ignoring fallible writes.
- Removed post-rebase local lint blockers in lifecycle helpers (`as` conversion, `expect`, ignored `Result`).
- Kept upstream `EnvelopeHeader: Default`; removed duplicate local impl and retained safe `u64::from` payload length conversion.
