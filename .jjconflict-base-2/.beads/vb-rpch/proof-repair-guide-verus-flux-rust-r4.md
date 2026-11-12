# Proof Repair Guide — vb-rpch verus-flux-rust-r4

bead: vb-rpch
state: 6 proof review repair guide
reviewer_invocation_id: p6-proof-review-r4-verus007
date: 2026-05-24

## What is repaired

- `VFR-R2-VERUS-007` no longer requires the stale-attempt no-state-effect conclusion.
- `VFR-R2-VERUS-001..007` are approved at standalone Verus ghost-model scope, subject to the active production source-correspondence trust boundary.

## Repairs still required for full proof-gate closure

1. Flux: install/enable `cargo flux` or provide an approved waiver, then rerun `VFR-R2-FLUX-001..007`.
2. Kani: repair `crates/vb_storage/src/kani_admission.rs` / harness construction so `vb_storage` compiles under Kani, then rerun `VFR-R2-KANI-001..007`.
3. Proptest: create or rename the planned `recovery_property_tests` target, then rerun `VFR-R2-PROPTEST-001..007`.
4. Fuzz: create or rename the planned `vb_rpch_seed_dimensions_fuzz` target, then rerun `VFR-R2-FUZZ-001..004`.
5. Rust attachment: replace the invalid `rtk cargo check -p vb_storage --features verus` command or add a justified feature gate, then rerun `VFR-R2-RUST-ATTACH-001..007` from the owning State 4/11 path.
6. Provenance: append truthful invocation rows for prior proof planning/writing/reviewing and the current review, or keep provenance as an explicit final blocker.

Do not reopen `VFR-R2-VERUS-007` unless the production replay filtering/source-correspondence contract changes.
