# Red Phase Report: vb-nsnc

## Files changed

- `crates/vb_validate/src/lib.rs`
- `crates/vb_validate/src/diagnostic.rs`
- `crates/vb_validate/Cargo.toml`
- `crates/vb_validate/tests/capability_contract_schema.rs`
- `crates/vb_validate/tests/capability_schema_kani.rs`
- `crates/vb_validate/benches/capability_schema.rs`
- `fuzz/Cargo.toml`
- `fuzz/src/lib.rs`
- `fuzz/src/bin/capability_name_schema.rs`
- `fuzz/src/bin/capability_contract_schema.rs`
- `.beads/vb-nsnc/red-phase.md`

## Intended failing test commands

- `cargo nextest run -p vb_validate --test capability_contract_schema`
- `PROPTEST_CASES=1000 cargo nextest run -p vb_validate --test capability_contract_schema proptest`
- `cargo test -p vb_validate --bench capability_schema --no-run`
- `cargo test -p vb_validate --test capability_schema_kani --no-run`
- `cargo test -p velvet-ballistics-fuzz --features fuzz --bin capability_name_schema --no-run`
- `cargo test -p velvet-ballistics-fuzz --features fuzz --bin capability_contract_schema --no-run`

## Why failures are expected before implementation

- Gate 12 currently checks only missing and orphan action contracts, so invalid capability names, too-long names, action mismatches, and duplicate requirements currently return `Ok(())` instead of the new exact `ValidationError` variants.
- The shared validation path delegates to the same live `gates.rs` gate, so the public `shared::validate_with_contracts` empty-name test fails until the schema validator is wired into that path.
- Diagnostic conversion has only a red-phase scaffold for the new capability variants, so `E050D..E0511` assertions fail until real diagnostic mappings and messages are implemented.
- Existing missing/orphan regression behavior is not duplicated in this red suite because those paths already pass before the new capability-schema implementation; this suite keeps only red tests for unimplemented behavior.
