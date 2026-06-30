# vb-qi37.6 State 8 Test Writer Report

STATUS: GREEN_FOR_STATE_8_SETUP

## Skill citations

- `/home/lewis/.claude/skills/test-writer/SKILL.md` lines 21-27 require behavior tests through observable APIs with exact state assertions.
- `/home/lewis/.claude/skills/test-writer/SKILL.md` lines 223-250 require fuzz targets for parsers/deserializers and corpus/setup ownership.
- `/home/lewis/.claude/skills/test-writer/SKILL.md` lines 252-276 require Kani harness setup for critical invariants.
- `/home/lewis/.agents/skills/test-writer/SKILL.md` contains the same rules and wins on conflict.
- `/home/lewis/.agents/skills/test-writer/references/rust-test-ecosystem.md` lines 142-185 document cargo-fuzz setup and lines 187-234 document Kani harness setup/execution.

## Inputs honored

- `.beads/vb-qi37.6/test-plan.md` State 8 setup checks, especially lines 630-639.
- `.beads/vb-qi37.6/contract-verification-review.md` approved State 8/11 routing.
- `.beads/vb-qi37.6/proof-review.md` approved deferred Kani/fuzz blockers: prior `KANI_SETUP_MISSING` and `FUZZ_BINS_MISSING` were not PASS.
- 24-row `.beads/vb-qi37.6/proof-obligations.jsonl` and `.beads/vb-qi37.6/traceability-matrix.jsonl`.

## State 8 setup changes

- Added `crates/vb_core/src/kani.rs` as the Kani module setup marker required by `INV-001-KANI-EXACT-SETUP` and `INV-002-KANI-CARDINALITY-SETUP`.
- Registered `capability_name_schema` and `capability_contract_schema` in `fuzz/Cargo.toml` for `PRE-003-FUZZ-SCHEMA` under `autobins = false`.
- Added `crates/workspace_tests/tests/vb_qi37_6_state8_setup.rs` to assert setup predicates exactly report `KANI_SETUP_PRESENT` and `FUZZ_BINS_PRESENT`.
- Added/strengthened behavior tests for exact capability matching, missing/excess/non-exact grant denial, and UI `required_capabilities` serde roundtrip. Existing in-flight implementation edits in this workspace are not claimed as State 8 setup work.

## Red/green evidence

### Prior red evidence from approved reviews

- `proof-review.md` lines 15 and 27: Kani setup previously reported `KANI_SETUP_MISSING`.
- `proof-review.md` lines 16 and 28: fuzz setup previously reported `FUZZ_BINS_MISSING`.

### Green State 8 setup evidence

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= sh -c 'if test -f crates/vb_core/src/kani.rs || test -f crates/vb_core/src/kani/mod.rs; then printf "KANI_SETUP_PRESENT\n"; else printf "KANI_SETUP_MISSING\n"; fi; if test -f fuzz/Cargo.toml && rg -q "name = \"capability_name_schema\"" fuzz/Cargo.toml && rg -q "name = \"capability_contract_schema\"" fuzz/Cargo.toml; then printf "FUZZ_BINS_PRESENT\n"; else printf "FUZZ_BINS_MISSING\n"; fi'
```

Output:

```text
KANI_SETUP_PRESENT
FUZZ_BINS_PRESENT
```

Command:

```bash
TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_6_state8_setup
```

Result: `2 passed; 0 failed`.

### Focused behavior/setup compile evidence

- `cargo test -p vb_core capability_set_grants_exact_name_and_action --lib` -> `1 passed; 0 failed`; existing unrelated `vb_core::budget::tests` unused-import warnings remain.
- `cargo test -p vb_runtime admit_artifact_run_rejects_missing_grants_without_allocation --lib` -> `1 passed; 0 failed`.
- `cargo test -p vb_runtime admit_artifact_run_rejects_non_exact_grant_without_allocation --lib` -> `1 passed; 0 failed`.
- `cargo test -p vb_ui_model action_description_view_required_capabilities_roundtrip --lib` -> `1 passed; 0 failed`.
- `cargo test -p velvet-ballistics-fuzz --features fuzz --bin capability_name_schema --no-run` -> compiled; existing unrelated `fuzz/src/lib.rs:1433` unused-comparison warning remains.
- `cargo test -p velvet-ballistics-fuzz --features fuzz --bin capability_contract_schema --no-run` -> compiled; same existing fuzz warning remains.

## Not claimed in State 8

- No `cargo kani` PASS is claimed. State 8 only proves module wiring/setup.
- No `cargo fuzz run` PASS is claimed. State 8 only proves bin registration and compile reachability.
- Full mutation, coverage, Miri, Moon, and release gauntlet remain State 11/release evidence.

## Route to State 9 / State 11

- State 9 may consume this setup evidence without treating it as formal execution PASS.
- State 11 must run:
  - `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo kani -p vb_core --harness capability_name_grants_harness`
  - `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo kani -p vb_runtime --harness check_capability_grants_exact_match`
  - `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo fuzz run capability_name_schema -- -runs=1000`
  - `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo fuzz run capability_contract_schema -- -runs=1000`
