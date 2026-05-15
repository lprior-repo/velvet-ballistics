# Machine Gate Report: vb-qi37.12.4

STATUS: PASS

## Commands

```text
command: scripts/check-ignored-fallible-results.sh
exit: 0
summary: FixturePass for DISCARD-001..006, path-bound exception accepted, overbroad/malformed exceptions rejected, production scan NoViolationFound.
```

```text
command: rtk cargo fmt --all --check
exit: 0
summary: formatting clean.
```

```text
command: rtk cargo test -p vb_runtime
exit: 0
summary: 1460 passed.
```

```text
command: rtk cargo test -p vb_ipc
exit: 0
summary: 407 passed.
```

```text
command: rtk cargo test -p vb_storage
exit: 0
summary: 983 passed.
```

```text
command: rtk cargo test -p velvet_ballastics -- --test-threads=1
exit: 0
summary: 471 passed.
```

```text
command: moon run :verify-standard
exit: 0
summary: direct gate, clippy, focused vb_compile unit tests, and standard Kani harnesses passed.
```

## Non-Blocking Observation

```text
command: rtk cargo test --manifest-path crates/vb_ui/Cargo.toml
exit: 101
summary: excluded vb_ui compile fails on pre-existing missing JournalEvent attempt fields; classified DEFERRED_GLOBAL for this bead because the direct gate and touched non-excluded package tests pass.
```
