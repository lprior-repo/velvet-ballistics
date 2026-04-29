## Summary

Implemented the Phase 4 cold compiler-side typed AST foundation for `vb-compiler` without moving YAML into runtime crates or changing successful compiler IR behavior.

Repair passes completed after black-hat rejection: the AST now retains supported top-level surfaces (`vars`, `examples`), rejects unsupported `ipc` trigger shapes through the compiler parity boundary, rejects unknown trigger and step fields, preserves parser-event `SourceMark` spans, and keeps new AST implementation files small and mechanical.

## Files Changed

- `crates/vb-compiler/src/ast.rs`
  - Added `WorkflowAst`, trigger, step, value, expression, and source-mark structures.
  - Kept `parse_workflow_ast` crate-private so callers cannot bypass compiler validation.
  - Added AST tests for surface retention, trigger rejection, unknown-field rejection, and source marks.
- `crates/vb-compiler/src/ast/marks.rs`
  - Builds marks from `saphyr-parser` events and `SourceMark::from_parser_span`, not source substring scans.
  - Records document, trigger, nested field, and step marks from parser spans.
- `crates/vb-compiler/src/ast/parse.rs`
  - Added cold YAML-tree to typed AST lowering after the current compiler parity gate.
  - Removed public unsupported primitive AST output so the returned type surface matches validated output.
- `crates/vb-compiler/src/ast/tests.rs`
  - Added non-panicking AST tests that compile under the workspace clippy gate.
- `crates/vb-compiler/src/lib.rs`
  - Exposed `pub mod ast`.
  - Added `YamlCompiler::parse_ast` as the cold AST boundary.
  - Added a minimal AST parse on successful compile paths after current IR compilation checks, preserving existing error ordering for unsupported compiler behavior.
- `.github/workflows/ci.yml`
  - Named the `moon ci` step with the required `geiger`, `vet`, `bench`, and `fuzz` gate labels so scaffold tests recognize the configured gate without adding a separate AST-only workflow comment.

## Constraint Adherence

- YAML remains confined to `vb-compiler`; runtime core was not given YAML dependencies.
- Source marks are represented as `Option<SourceMark>` with available byte/line/column marks for parsed source-backed AST nodes.
- Source marks are copied from parser spans. Tests assert exact index, line, and column for trigger, nested field, and step marks.
- Current deterministic `CompileError` variants are reused; no broad diagnostic surface was added.
- Compiler behavior is kept stable by forcing `parse_ast` through the existing successful IR compile path before exposing the typed AST.
- No non-test `unwrap`, `expect`, `panic`, `unsafe`, or runtime YAML interpretation was introduced.

## Verification Results

- `cargo fmt --all -- --check` — passed.
- `cargo test -p vb-compiler` — passed: 71 tests.
- `cargo test --workspace --all-targets` — passed: 158 tests.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed: no issues found.

## Remaining Risks

- `parse_ast` intentionally performs a cold compile-parity pass before returning AST. This is not hot-path behavior and preserves current compiler diagnostics until later compiler phases split validation/AST/IR cleanly.
- The AST surface intentionally only exposes currently returnable step kinds. Future primitive phases must extend the type surface and parser together.
