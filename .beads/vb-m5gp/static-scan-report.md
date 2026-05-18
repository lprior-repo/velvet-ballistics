# Static Scan Report

STATUS: PASS

- Source clippy exact obligation: `cargo +nightly clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings` — PASS.
- Strict source scan: `cargo +nightly clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` — PASS.
- Dependency-edge scan after repair: `errors_to_validation=0 matches`; `validation_to_lowering_or_core=0 matches`; `include_bodies=0 matches`.
- Public split module exposure scan: `pub mod mod_compile_*` absent; private `mod mod_compile_core/errors/lowering/validation` declarations present in `lib.rs`.
- Stale scaffolding review: no blind wiring of stale `compile/`, `lower/`, or `validation/` scaffolding found in active split path.
