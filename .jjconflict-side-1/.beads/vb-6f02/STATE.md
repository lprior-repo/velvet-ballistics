# STATE.md - vb-6f02

## Bead
- ID: vb-6f02
- Title: quality: Add contracts-as-data suite
- Source checkout: /home/lewis/src/velvet-ballistics
- Isolated workspace: /home/lewis/src/velvet-ballistics-vb-6f02-work

## State History
- State 1-6: COMPLETE (proof APPROVED)
- State 7: COMPLETE (test-plan.md)
- State 8: Tests written (proptest 17 pass, Kani harness)
- State 9: REJECTED (tests not bound to production, pipeline untested)
- State 8 REPAIR 1 (CRITICAL): COMPLETE — production binding test created
  - `contracts_production_binding.rs`: 31 tests, all pass
  - `contracts_as_data_props.rs`: 17 proptest, all pass
- State 11: REPAIR 2 (CRITICAL — miri gate + integration tests) COMPLETE
  - miri gate fixed (3 new ValidationError match arms + diagnostic codes)
  - `contracts_integration.rs`: 30/30 pass (was 8/30, 22 failures)
  - Root cause: `collect_cue_files()` path resolution bug — used `strip_prefix(base.parent())` instead of `strip_prefix(base)`, producing paths relative to parent dir while cue ran from contracts_dir
  - Fix: Changed to `strip_prefix(base)`, added CWD param to `run_cue_vet`, absolute path for file I/O
  - Also fixed `test_deeply_nested_discovery` dir creation bug and `test_json_deterministic_key_order` search pattern

## Repair Status
- Repair 1 (CRITICAL — production binding): DONE
- Repair 2 (CRITICAL — miri gate): DONE
- Repair 3 (CRITICAL — integration path / cue vet CWD): DONE
- Repair 4 (MAJOR — monotonicity gate): DONE (already working)
- Repair 5 (RECOMMENDED — JSON output): DONE (BTreeMap already deterministic)
- Repair 6 (RECOMMENDED — test fixes): DONE (nested dir, search pattern)
- Repair 7 (MINOR — unwrap cleanup in proptest): PENDING (cosmetic)

## Test Results (Post-Repair)
- `contracts_production_binding.rs`: 31/31 pass ✓
- `contracts_as_data_props.rs`: 17/17 pass ✓
- `contracts_integration.rs`: 30/30 pass ✓
- `contracts_as_data_kani.rs`: compiles ✓ (Kani tool not available)
- `cargo check -p xtask -p vb_validate -p vb_cli`: 0 new errors ✓
- `moon ci`: miri gate FIXED — all remaining failures pre-existing

## Next State: Repair 2 — integration path tests
- Exercise `discover_contracts()` pipeline end-to-end
- Test file walking → cue vet → field extraction → error collection → sorting → GateEvidence
