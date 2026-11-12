# Ordinary db79 source reconciliation

Candidate is 509859fc3db273e8f452c3773cc738ffd29b875b; incoming main is db791e8f7d55517b969a41885bfb400c4be498c7; verified merge base is ccc958d784849f46b335ba416e2d9a3c28c6ca0a. Work only in unique colocated Git/JJ root /home/lewis/src/isoloated/velvet-ballistics-db79-port-luna, with matching roots and a new contracted merge change.

Resolve only five ordinary conflict roots and their actual upstream destinations listed below. The contract is the only additional tracked author path; all other automatic merge contents must remain unchanged.

- crates/vb_compile/src/mod_compile_lowering/canonical_reduce.rs
- crates/vb_compile/src/mod_compile_lowering/canonical_reduce/body.rs
- crates/vb_compile/src/mod_compile_lowering/part_01.rs
- crates/vb_compile/src/mod_compile_lowering/part_01/width.rs
- crates/vb_compile/src/mod_compile_lowering/part_01/layout.rs
- crates/velvet_ballistics/tests/cli_integration.rs
- crates/velvet_ballistics/tests/cli_integration/part_a.rs
- crates/velvet_ballistics/tests/cli_integration/part_b.rs
- crates/velvet_ballistics/tests/cli_integration/part_c.rs
- crates/velvet_ballistics/tests/cli_integration/part_d.rs
- xtask/src/proof.rs
- xtask/src/proof/commands.rs
- xtask/src/proof/types.rs

Keep upstream split structure. Reduce root/body retain the candidate's canonical_reduce_initial::accumulator_type and existing parent field_error reuse, removing upstream duplicate helpers; no semantic change to nodes/digest/slots/typed errors. Part01 width/layout children retain upstream visibility/reexports and the candidate's existing bounded body_width_walk module and five-argument finalize_compiled_workflow call; BodyWidthWalker and finalizer implementation files stay untouched.

Move candidate CLI function bodies and all original assertions/case identities into the corresponding upstream lexical include parts; retain the input_bin_case child registration, typed input_slot_parts result and exact cli_tempdir helper. Restore neither an old test monolith nor an old compiler monolith.

Retain upstream xtask proof/types and command split, with candidate Kani filter module, package-qualified command derivation, obligation selection, registry/evidence functions and all five existing tests. Only ordinary xtask command source is in scope; no proof/model/gate behavior changes or solver execution.

Protected conflicts scripts/test-integrity-justifications.tsv, verification/verus/extern_budget_monotonic.rs and verification/verus/extern_collect_ir_structure.rs must remain unresolved with their automatic content hashes unchanged. No manual gate/config/exception/registry/model edits. All56 incoming paths, including step_state_machine.rs, are inherited automatically and inventoried; unchanged math bytes are not newly verified proofs.

Before author edits, save the automatic merge source pin, eight conflict paths, protected conflict hashes and full changed-path content baseline. After porting, require exactly the three protected conflicts remain; preserve every path outside the author fence and contract, all candidate-only paths and every non-owned incoming path. Record function/test source mappings, original case counts, CLI temp helper hash and compile-only error-order limits. No assertions, cases, values, signatures or messages may be weakened. Module visibility/path qualification adjustments must be explicit.

Use pinned nightly-2026-04-28 edition/style2024, no foreign formatting, no new unsafe/panic/unchecked constructs or waivers. No broad native/Moon checks on the partial conflict tree; at most five minutes of affected-package compile is permitted only after all Cargo source conflicts are resolved, using own target, jobs two, wrappers off and TMPDIR=/run/user/1000/vbs. Freeze ordinary proposal/source/hash manifest plus empty successor; source still has protected conflicts and is not a landing or full acceptance claim. No main/parent writes, history rewrite, Git commit/push, agents or messages to other chats.
