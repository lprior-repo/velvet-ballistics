# Formal Verification Report — vb-qi37.6 State 11 integration repair

STATUS: APPROVED

## Executed obligation evidence

- Moon proof lane: `TMPDIR=/home/lewis/src/tmp_build/vb-qi37.6-integration-moon CXXFLAGS=-pipe CFLAGS=-pipe RUSTC_WRAPPER= moon run :verify-proof --force` -> PASS, `All proof checks passed`.
- Capability TLA+ configs: `CapabilityLifecycleAll.cfg`, `CapabilityLifecycleGateMismatch.cfg`, `CapabilityLifecycleNoContract.cfg`, `CapabilityLifecycleExcessGrant.cfg`, `CapabilityLifecycleExactProfile.cfg`, `CapabilityLifecycleLegacyBypass.cfg` -> each TLC run reported `Model checking completed. No error has been found.`
- Verus: `TMPDIR=/home/lewis/src/tmp_build/vb-qi37.6-integration-moon RUSTC_WRAPPER= verus verification/verus/capability_artifact_model.rs` -> PASS, `8 verified, 0 errors`.
- Kani: `cargo kani -p vb_core --harness capability_name_grants_harness` and `cargo kani -p vb_runtime --harness check_capability_grants_exact_match` -> PASS.
- Fuzz registration/execution: `capability_name_schema` and `capability_contract_schema` targets present in `fuzz/Cargo.toml`; both `cargo fuzz run ... --target x86_64-unknown-linux-gnu -- -runs=1000` commands completed.
- Focused tests: runtime submit, admission capability, UI required capabilities, and UI admission integration filters exited successfully.

## Waivers

- None for vb-qi37.6 required capability obligations.

## Decision

State 11 approved for current-main integration repair.
