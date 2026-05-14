# Formal Verification Summary: vb-gvmt

## PASS Evidence

- TLA+: `tlc -config .beads/vb-gvmt/specs/GeneratedParity.cfg .beads/vb-gvmt/specs/GeneratedParity.tla` passed with `No error has been found`, 17 states generated, 13 distinct states. Claim scope is valid lifecycle/journal ordering/trace parity abstraction only; invalid-resume no-mutation and concrete journal no-drop are not claimed from TLA+ in this revision.
- Verus: `/home/lewis/.local/bin/verus --crate-type=lib .beads/vb-gvmt/proofs/generated_semantics_verus.rs` passed with `6 verified, 0 errors`.
- Kani: five `vb_codegen::kani_generated_runtime` harnesses passed with `5 successfully verified harnesses, 0 failures, 5 total`.
- Executable parity: `rtk cargo test -p vb_codegen post_011 -- --nocapture` passed 4 POST-011 semantic comparison tests; broader `post_` suite passed 33 tests.
- Canonical CI: `moon ci` passed with 19 completed tasks and nextest 8276/8276 tests passed.

## Non-PASS / Deferred

- Mutation: scoped `cargo mutants` run produced 35/35 unviable mutants. This is recorded as `FAIL_UNVIABLE` and does not satisfy mutation adequacy.
- `compare_generated_to_ir`: remains a static source-pattern/count guard. Semantic evidence comes from POST-011 executable tests, not from this function.
