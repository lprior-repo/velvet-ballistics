bead_id: vb-tw3b
phase: 2

# Codebase map

Scope inspected:
- `crates/vb_expr/src/eval.rs`: bounded stack bytecode evaluator for `ExprProgram`.
- `crates/vb_codegen/src/tests.rs`: existing generated-vs-IR parity tests.
- `crates/vb_codegen/src/lib.rs` and `src/codegen/mod.rs`: generated expression validation/emission surfaces.

No code change required for closure; current tests already exercise binary expression operators, generated/interpreter finished output, taint, journal, append/merge helper parity, and generated typed error variants.
