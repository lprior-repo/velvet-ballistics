# Runtime shared fixture root declarations

Bead: vb-eirmk.13.13.3.4.16.3.1. Parent: vb-eirmk.13.13.3.4.16.3. Source-only fixture repair; no production validation/API/schema changes.

Exclusive source fence: crates/vb_runtime/src/prepared_inputs/fixtures.rs; crates/vb_runtime/src/recovery/input_binding/fixtures.rs; crates/vb_runtime/src/run_checkpoint/symbol_prefix_fixture_tests.rs; crates/vb_runtime/src/runtime/bound_symbol_duplicate_retry_fixture_tests.rs. Lorentz retains artifact decode/admission production and all other runtime files.

The actual SlotFlow seed marks optional inputs without either default as absent; every unconditional read must have a required declaration or a genuine preceding write. Do not add a synthetic write/default, lower the validator, mock acceptance or bypass checked construction.

Prepared-input composition is polymorphic: slot0 can Symbol or Object, slot1 can Symbol or Object, slot2 is List. Declare required Any slot0 for the Finish read and optional Any slot1/List slot2 for additional supplied roots; these pure composition tests do not assert complete admission of every partial root set.

Recovery carrier fixture is intentionally used for Object and scalar I64 controls, with a generic Finish read, so its single declared root must be required Any rather than optional Object. Prefix/action fixture has a polymorphic action input and must declare required Any slot0. Duplicate-retry expression reads an Object Has receiver and must retain required Object slot0.

Preserve all original supplied roots/values/IDs/taints, duplicate symbol tails and order, arena snapshots, source identity/prefix policies, node/op/PC layout, full journal/counter/authority oracles, case names and literal expected errors. Keep versioned digest construction over the corrected real WorkflowParts rather than fabricating an identity. Do not modify/delete the parent's generated proptest regression artifact.

Import only these four current parent fixture postimages, double-hash guarding reads and preserving own preimages in ignored evidence/JJ ancestry; never copy a whole mutable tree. Source/fmt/function-bound receipts precede a frozen handoff. Parent runs actual runtime native and independent CODE/TEST; no acceptance from source parsing or model RESULT.
