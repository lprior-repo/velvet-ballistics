# Proof Writer Report — vb-rpch verus-flux-rust-r5

bead: `vb-rpch`  
state: 5 Kani/proptest/fuzz harness repair  
date: 2026-05-24

## Scope executed

Touched only Kani/proptest/fuzz proof artifacts and `#[cfg(kani)]` harness support needed to unblock Kani compilation. No Flux work performed.

## Artifacts changed

- `crates/vb_storage/src/kani_admission.rs` — repaired global Kani compile blockers from non-`Arbitrary` `RuntimePolicy`/`FjallJournal` construction.
- `crates/vb_storage/src/kani_recovery_hydrate.rs` — repaired/added RPCH Kani harnesses for `VFR-R2-KANI-001..004`; bounded event generation and result-drop handling for later harnesses.
- `crates/vb_storage/src/recovery/types.rs` — `#[cfg(kani)]` deterministic replay-tracker backing store to avoid `HashSet`/OS-random symbolic blow-up; not production reachable.
- `crates/vb_storage/tests/recovery_property_tests.rs` — added planned target and seven named properties for `VFR-R2-PROPTEST-001..007`.
- `fuzz/Cargo.toml` and `fuzz/fuzz_targets/vb_rpch_*_fuzz.rs` — added planned RPCH fuzz target names for `VFR-R2-FUZZ-001..004`.

## Blockers closed

- Closed prior `BLOCKED_GLOBAL_COMPILE` for RPCH Kani compile on the checked harnesses: `VFR-R2-KANI-001..004` now compile and verify under Kani 0.67.0 using harness-specific commands without disabling unwinding checks.
- Closed prior `BLOCKED_MISSING_ARTIFACT` for proptest: target `crates/vb_storage/tests/recovery_property_tests.rs` now exists and all seven planned property names execute.
- Closed prior `BLOCKED_MISSING_ARTIFACT` for fuzz target naming: all four planned fuzz target files and `Cargo.toml` bins now exist.

## Remaining blockers / limitations

- `VFR-R2-KANI-005..007`: not claimed closed. Existing hydration/replay harnesses remain expensive and only smoke/repair groundwork was done; exact harness verification should rerun with a larger resource budget or further model reduction.
- `VFR-R2-FUZZ-001..004`: target artifacts exist, but cargo-fuzz execution is `BLOCKED_TOOLCHAIN_TARGET`: configured fuzz build uses `x86_64-unknown-linux-musl`, which is not installed and incompatible with sanitizer static libc in this environment.
- Planned commands contain invalid `--no-unwind` for installed cargo-kani 0.67.0. Evidence uses harness-specific `cargo kani -p vb_storage --harness ...` without disabling unwinding checks.

## Proof reviewer rerun disposition

Proof-reviewer may rerun for `VFR-R2-KANI-001..004` and `VFR-R2-PROPTEST-001..007`. Do not approve fuzz execution or Kani `005..007` as closed from this r5 evidence.
