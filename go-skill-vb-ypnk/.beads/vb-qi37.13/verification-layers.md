bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 3
updated_at: 2026-05-14T22:16:30Z
attempt: 1-of-7

# Verification Layers

## Layer Assignment

- INV-001/POST-001 -> Verus + unit/integration tests.
- POST-002/ERR-009 -> unit/integration tests + static grep evidence that no public `= 9` remains.
- INV-003/POST-005 -> fuzz/proptest/Kani-or-approved-waiver for postcard decode/encode route.
- INV-004 -> CLI parser/diagnostic tests.
- INV-005 -> Verus/schema tests.

## Verus Scope

- Target: `verification/verus/diagnostic_envelope_verus.rs`.
- Required repair: replace `0..=9` proof surface with `0..=8` public exit-code proof surface.
- Shell exclusions: process I/O, filesystem writes, command execution.

## Executable Scope

- Unit: `crates/velvet_ballastics/src/exit_code.rs` tests must reject out-of-range values.
- Integration: CLI structured-output tests must observe only codes `0..=8`.
- Fuzz/proptest/Kani: postcard route must execute under repository tooling or be explicitly waived by proof review.

## Waivers

- TLA-WAIVE-001 and LEAN-WAIVE-001 as recorded in sibling artifacts.
- No waiver is currently approved for the integrated postcard proof route.
