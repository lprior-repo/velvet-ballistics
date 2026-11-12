# Fixture V18: Safe array access in declared_keyspaces test

**File:** `crates/vb_storage/src/tests.rs`
**Test:** `declared_keyspaces_returns_eleven_entries`

## Change

Two new array-index assertions added in prior work used unchecked indexing (`[9]`, `[10]`), violating the user's no-new-unsafe rule. Replaced with safe `.get()` calls:

```diff
- assert_eq!(keyspaces[9], "run_inputs");
+ assert_eq!(keyspaces.get(9), Some(&"run_inputs"));

- assert_eq!(keyspaces[10], "emergency_deny");
+ assert_eq!(keyspaces.get(10), Some(&"emergency_deny"));
```

Existing assertions 0..8 remain unchanged. No broader style cleanup performed. No production/API/config/gates changes.

## Gate result

```
python3 /tmp/vb-run-gate.py 120 .beads/vb-ny068/evidence/FIXTURE_V18_FMT \
    rustup run nightly-2026-04-28 cargo fmt --all --check

exit_code: 0 (PASS)
wall_time: 2.02s
```