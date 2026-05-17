# Mutation Report — vb-qi37.12.2 State 11 Rerun

STATUS: PASS

Tool: `/home/lewis/.cargo/bin/cargo-mutants`, version `cargo-mutants 27.0.0`.

## Classifying scoped resume/is_resumable command

`env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo mutants -p vb_runtime --file crates/vb_runtime/src/shard/types.rs -F 'RuntimeState::is_resumable|ResumeError::record_source|ResumeError::source_runtime_error' --all-features --timeout 120 --in-place --output .beads/vb-qi37.12.2/mutants-out-resume-filtered --no-times`

Result: PASS.

```text
Found 6 mutants to test
ok       Unmutated baseline
6 mutants tested: 5 caught, 1 unviable
```

## Exact is_resumable repair confirmation

`env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo mutants -p vb_runtime --file crates/vb_runtime/src/shard/types.rs -F RuntimeState::is_resumable --all-features --timeout 120 --in-place --output .beads/vb-qi37.12.2/mutants-out-is-resumable --no-times -- --lib is_resumable`

Result: PASS.

```text
Found 2 mutants to test
ok       Unmutated baseline
2 mutants tested: 2 caught
```

owner_state: 11
rerun_from: 11
MUTATION-001: PASS; no missed scoped mutants.
