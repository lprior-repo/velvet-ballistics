# vb-eirmk.13.1.2 contract

Bounded test repair in `crates/workspace_tests/tests/runtime_version_barrier_tests.rs`; production behavior and gates remain governed by the master contract.

The original malformed nested Collect fixture is preserved in `malformed_nested_collect_missing_steps_field`, requiring exactly one `MissingStepField { step: 0, field: "collect.steps" }`. The original unsupported-primitive test name now exercises a structurally valid Together inside Collect and requires exactly one `UnsupportedStepPrimitive { step: 0, primitive: "parallel" }`.

The positive nested Collect fixture requires exactly eight node variants in order: CollectStart, CollectStart, SetConst, CollectNext, CollectFinish, CollectNext, CollectFinish, Finish. Variant discriminants are compared directly. CollectNext slots at steps 3 and 5 must be present and distinct. Actual compiled loop regions must be inner (start 1, body 2, control 3, done 4) and outer (start 0, body 1, control 5, done 6); step 2 has no control region.

Use checked access and typed values; no casts, unchecked indexing, panic helpers, skipped cases, or weakened assertions. Each helper is limited to 25 logical lines; the actual repository test category determines the file limit.

Acceptance requires the pinned warnings-denied registered nextest suite, CODE review and independent TEST review of the frozen source. Focused success does not establish full workspace or backend acceptance.
