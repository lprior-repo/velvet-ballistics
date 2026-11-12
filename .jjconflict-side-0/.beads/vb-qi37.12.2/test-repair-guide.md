# Test Repair Guide — vb-qi37.12.2 — State 8 Mutation Repair

STATUS: APPROVED — no further repair required for `RuntimeState::is_resumable`.

## Required State 11 Rerun
- Rerun State 11 mutation from the previously failed scoped gate.
- Include the shard lib tests so `is_resumable` unit coverage is executed, not only the resume propagation integration test.

Suggested rerun sequence:

```bash
# Original State 11 focused integration mutation gate.
TMPDIR="/home/lewis/src/vb-qi37-12-2/tmp" RUSTC_WRAPPER= \
  cargo mutants -p vb_runtime \
  --file crates/vb_runtime/src/shard/lifecycle/chunk_001.rs \
  --file crates/vb_runtime/src/shard/types.rs \
  --file crates/vb_runtime/src/error/conversions.rs \
  --all-features --timeout 120 --in-place \
  --output .beads/vb-qi37.12.2/mutants-out-status-rerun \
  --no-times -- --test vb_qi37_12_2_resume_error_propagation

# Required supplemental lib-test mutation gate for RuntimeState::is_resumable.
TMPDIR="/home/lewis/src/vb-qi37-12-2/tmp" RUSTC_WRAPPER= \
  cargo mutants -p vb_runtime \
  --file crates/vb_runtime/src/shard/types.rs \
  --all-features --timeout 120 --in-place \
  --output .beads/vb-qi37.12.2/mutants-out-is-resumable-rerun \
  --no-times -- --lib is_resumable
```

The original integration-only mutation gate does not execute the new shard unit tests; keep the supplemental lib-test mutation pass or run an equivalent no-filter package mutation gate.

## Do Not Change
- Do not alter `RuntimeState::is_resumable`; `crates/vb_runtime/src/shard/types.rs:331-333` is correct.
- Do not replace exact `assert_eq!` checks with `is_ok`, `is_err`, or smoke assertions.
- Do not allow-list the mutants; they are true behavioral mutants and are now killed.

owner_state: 11
rerun_from: 11
