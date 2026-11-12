# State 11: Machine Gate Report (Post-Repair)

## Gate Execution Results

### 1. moon ci --force
**STATUS: FAIL (all failures pre-existing)**
- Command: `moon ci --force`
- Result: 5 completed, 5 failed, 11 skipped
- Failed tasks (all pre-existing):
  - `velvet-ballistics:fuzz-smoke` - pre-existing compilation error in fuzz crate
  - `velvet-ballistics:fmt` - pre-existing diff marker
  - `velvet-ballistics:lint-src` - pre-existing compilation errors
  - `velvet-ballistics:check` - pre-existing xtask compilation errors
  - `velvet-ballistics:workspace-assertions` - pre-existing assertion failure
- miri gate: **FIXED** — no longer a new failure
- Passed: beads-server-mode, agent-cli-contract, nightly-feature-gate, fmt (lint), source-length, miri (full)

### 2. cargo check --workspace
**STATUS: FAIL (pre-existing only)**
- Command: `cargo check --workspace`
- Result: 8 errors, 11 warnings (5 crates)
- All errors pre-existing in `xtask/src/shell.rs` (E0425/E0433)
- No new compilation errors from this bead

### 3. cargo test (workspace_tests)
**STATUS: PASS (all bead-related tests)**
- Command: `cargo test -p velvet-ballistics-workspace-tests`
- Results by test file:
  - `contracts_production_binding.rs`: **31 passed, 0 failed** ✓
  - `contracts_as_data_props.rs`: **17 passed, 0 failed** ✓
  - `contracts_integration.rs`: **30 passed, 0 failed** ✓ (was 8/30, 22 failures)
  - `contracts_as_data_kani.rs`: compiles cleanly (Kani harnesses)
- Fixes applied:
  1. **`collect_cue_files` path resolution** — changed `strip_prefix(base.parent())` → `strip_prefix(base)` so relative paths are correct for cue execution
  2. **`run_cue_vet` CWD support** — added `cwd: Option<&Path>` parameter; cue runs with contracts_dir as working directory so relative paths resolve
  3. **`validate_single_file` file I/O** — passes absolute path for fs::read_to_string, relative path + CWD for cue vet
  4. **`test_deeply_nested_discovery`** — fixed test dir creation (was creating nested dirs but writing files relative to tempdir root)
  5. **`test_json_deterministic_key_order`** — fixed search pattern from `"aaa_bad"` → `INVALID_KIND: aaa_bad` (BTreeMap key format)

### 4. cargo clippy -p xtask
**STATUS: FAIL (pre-existing only)**
- Pre-existing: 5 unused functions in `xtask/src/shell.rs`
- No new clippy errors from contracts.rs changes

### 5. Kani (formal verification)
**STATUS: NOT_AVAILABLE**
- Harnesses compile cleanly but Kani tool not available for execution

### 6. Verus (formal verification)
**STATUS: FAIL (pre-existing spec compilation issues)**
- Type inference errors in contracts_as_data_spec.rs (not introduced by this bead)

### 7. TLC (formal verification)
**STATUS: NOT_AVAILABLE**
- TLC tool not installed

## Overall Classification (Post-Repair)

| Gate | Result | Classification | Cause |
|------|--------|---------------|-------|
| moon ci | FAIL | DEFERRED_GLOBAL | all failures pre-existing |
| cargo check | FAIL | DEFERRED_GLOBAL | pre-existing xtask errors |
| cargo test | PASS | — | all 78 bead tests pass |
| cargo clippy | FAIL | DEFERRED_GLOBAL | pre-existing xtask errors |
| Kani | NOT_AVAILABLE | — | tool not available |
| Verus | FAIL | DEFERRED_GLOBAL | pre-existing spec issues |
| TLC | NOT_AVAILABLE | — | tool not installed |

## Repair Summary

### Repair 1: miri gate (BLOCK_REGRESSION → fixed)
- Added 3 match arms in `vb_validate/src/diag_render.rs` for `MissingSchemaVersion`, `CueVetFailed`, `VersionMonotonicityBreach`
- Added 3 match arms in `vb_validate/src/diagnostic.rs`
- Added 3 match arms in `vb_validate/src/diag_convert.rs`
- Added 3 diagnostic codes (0x0601-0x0603) in `diag_codes.rs`
- Added 3 match arms in `vb_cli/src/app_impl.rs` `explain_validation_error`

### Repair 2: integration tests (BLOCK_LOCAL → fixed)
- **Root cause**: `collect_cue_files()` used `strip_prefix(base.parent())` producing paths relative to parent dir, but cue ran from contracts_dir — mismatch caused cue vet to fail on files in temp directories
- **Fix**: Changed to `strip_prefix(base)` so paths are relative to contracts_dir (the cue CWD)
- **Also fixed**: `validate_single_file` now uses absolute path for file I/O + relative path + CWD for cue vet
- **Also fixed**: `test_deeply_nested_discovery` directory creation bug
- **Also fixed**: `test_json_deterministic_key_order` search pattern mismatch
