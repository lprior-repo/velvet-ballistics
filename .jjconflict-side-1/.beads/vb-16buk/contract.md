**Contract: Add missing `input_slots` initializers to `WorkflowParts` in cross-crate adversarial tests**

**Scope:** `crates/velvet_ballistics/tests/cross_crate_adversarial.rs` only. No production code or assertion changes.

**Fix:** Add `input_slots: Box::from([])` to every `WorkflowParts` struct initializer in the test file.

**Rationale:** `WorkflowParts` requires an `input_slots` field. Test fixtures were missing this initializer, causing compilation errors. All test workflows are self-contained with no declared external input ports, so an empty `input_slots` array is correct.

**Coverage:**
- `minimal_parts` helper: covers `direct`, `core`, `engine`, and `invalidIR` test variants.
- `concrete3SetConst` workflows: produce their own slots internally; no CLI binary input payload is involved.

**Design decision:** Do not infer input ports from `frame.write_slot` calls. Those are runtime-internal input seeding operations, not CLI schema declarations. Empty `input_slots` is added only when no external ports are declared.

**Acceptance:** Execute the focused cross-crate target and inspect actual results. Compilation/pass is not yet demonstrated. No behavioral or assertion changes.