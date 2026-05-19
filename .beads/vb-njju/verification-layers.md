# Verification Layers

## Boundary

- Verus-owned kernel: none required for State 3; State 4 may add only if non-trivial pure classifiers are introduced.
- TLA+ temporal model: waived; no temporal behavior.
- Theorem projection: waived; no theorem kernel.
- Runtime shell: workspace acceptance tests and quality gates.
- External systems excluded from formal proof: Moon, cargo-mutants internals, cargo-fuzz internals, filesystem, fuzz corpus management.

## Layer assignment

- PRE-001, PRE-002, POST-005, INV-001 -> `cargo test` acceptance catalog validation.
- PRE-003, POST-001, INV-003 -> `cargo test` + `cargo-mutants` scoped to vb-njju acceptance test and mutation-plan validation.
- PRE-004, POST-002, INV-002 -> `cargo test` + `moon run :fuzz-smoke` + `cargo fuzz build --target x86_64-unknown-linux-gnu`; State 4 must add/verify run or seed evidence for required targets.
- PRE-005, POST-003, INV-004 -> `cargo test` and proptest target for generated-vs-IR taint parity.
- PRE-006, POST-004, INV-005 -> boundary inventory tests + fuzz-smoke + release gate failure test.
- INV-006 -> traceability matrix + JSONL proof obligations validation.

## Required executable commands

- `cargo test --package velvet-ballastics-workspace-tests --test vb_njju_mutation_fuzz_property_closure`
- `cargo test --package velvet-ballastics-workspace-tests --test vb_hxm0_acceptance_catalog`
- `cargo test --package velvet-ballastics-workspace-tests --test vb_c3k9_current_api_mutation_plan`
- `cargo test --package vb_codegen --lib proptests::fixed_six_step_emitted_rust_and_ir_match_finished_signal_and_slots`
- `cargo test --package vb_storage --lib proptests::ppi_001_deterministic_replay_invariant`
- `cargo test --package velvet-ballastics-workspace-tests --test vb_y1zq_boundary_inventory_contract`
- `moon run :mutants-smoke`
- `cargo mutants --package velvet-ballastics-workspace-tests --test vb_njju_mutation_fuzz_property_closure`
- `moon run :fuzz-smoke`
- `cargo fuzz build --target x86_64-unknown-linux-gnu`

## Verus scope

- Current scope: waived.
- Trigger for State 4: if State 4 implements non-trivial pure functions that classify mutation/fuzz/property/release evidence, add Verus target names before formal verification. Until then, cargo tests/proptest/mutation own the evidence.

## TLA+ scope

- Current scope: waived in `tla-spec.md`.
- No module/model/config path is claimed.

## Theorem scope

- Current scope: waived in `lean-contract.md`.
- No Lean module/theorem is claimed.

## Release gate requirements

- All four bead acceptance criteria are release-critical.
- Missing or weak local evidence must block release or produce an explicit blocker/follow-up record accepted by independent review.
- Build-only fuzz smoke is weak evidence for this bead.
- Unrelated mutation smoke is weak evidence for admission branch closure.

## Waivers

- TLA-WAIVE-001: no temporal model; compensated by BDD/property/mutation/release gate tests.
- LEAN-WAIVE-001: no theorem kernel; compensated by executable evidence.
- VERUS-WAIVE-001: no new pure production classifier specified at State 3; if State 4 adds one, revisit.
