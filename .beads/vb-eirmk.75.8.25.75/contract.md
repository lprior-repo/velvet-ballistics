# Canonical primitive diagnostic contract

Public CompileError::code and diagnostic_code must return the registered INVALID_TOGETHER code for UnsupportedStepPrimitive with primitive together and StepFieldShape with field together. They must return INVALID_REDUCE for the corresponding reduce names. Existing classifications for parallel, branches, aggregate, summarize, collect/gather and generic unknown names remain unchanged. Error classification does not enable any YAML execution syntax.

This repair addresses the public library error API. A canonical compiler-pipeline producer of the exact together/reduce fallback cases has not been demonstrated; early canonical YAML parse failures and unreachable unsupported reduce-body arms are not evidence of such a producer.

Scope: crates/vb_compile/src/mod_compile_errors/codes.rs, automatically discovered public integration tests in crates/vb_compile/tests/canonical_primitive_diagnostics.rs, and this contract. No new variants, API signatures, schemas, persisted formats, dependencies, gates, budget logic, idempotency logic, features or trust changes.

Acceptance: run the unchanged public integration suite before the repair and observe all four canonical cases fail with actual fallback codes; preserve that source and raw log. Add canonical labels to the four existing mapper arms. Re-run the identical suite, then the full vb_compile native suite and pinned canonical format/source checks scoped where applicable. Preserve exact commands, exits, logs and source hashes. Independent CODE and TEST review precede landing; this workspace does not self-approve or push.
