STATUS: APPROVED

Findings:
- `crates/vb-compiler/src/lib.rs:116-127` now runs AST parsing plus `references::validate_workflow_ast` on the public `compile` boundary after strict YAML, duplicate-key, strict-profile, lowering, and schema checks. The prior public bypass is closed.
- `crates/vb-compiler/src/lib.rs:131-142` keeps `parse_ast` on the same ordering path, so lowering/schema diagnostics still win before reference diagnostics.
- `crates/vb-compiler/src/references.rs:21-46` now builds declaration tables without pretending duplicate table inserts are reference errors. Duplicate ownership stays upstream, as required.
- `crates/vb-compiler/src/references.rs:113-199` rejects unknown roots, illegal runtime/bare nondeterministic references, and step references through typed `CompileError` variants. No runtime crate or YAML leakage found in the reviewed files.
- `crates/vb-compiler/src/references/tests.rs:162-212` now proves `compile` rejects invalid references in retained `examples`, which was the missing contract test.

Gate evidence:
- Ran `rtk cargo test -p vb-compiler references::tests -- --nocapture`: 7 passed.
- Forbidden construct scan over reviewed compiler files found no `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, or `unsafe`.

Verdict:
APPROVED. The targeted repair closes the compiler bypass and adds the tests that should have existed the first time.
