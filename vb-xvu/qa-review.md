STATUS: APPROVED

# Black Hat QA Review — `vb-xvu` AST repair

## Findings first

The previous blockers are fixed. This is now acceptable.

- **Parser-span source marks: PASS.** `crates/vb-compiler/src/ast/marks.rs:35-41` builds `AstMarks` by driving `saphyr_parser::Parser` and converting event spans through `SourceMark::from_parser_span`. The old substring-scan garbage is gone.
- **No public bypass parser: PASS.** `crates/vb-compiler/src/ast.rs:11` exports `parse_workflow_ast` as `pub(crate)`, and `crates/vb-compiler/src/ast/parse.rs:10` is also `pub(crate)`. External callers only get `YamlCompiler::parse_ast` at `crates/vb-compiler/src/lib.rs:125-134`, which runs strict validation plus compile parity first.
- **Returnable public AST surface: PASS.** `StepKindAst` now only advertises `Save`, `Choose`, and `Finish` in `crates/vb-compiler/src/ast/types.rs:90-112`. The dead `Unsupported`/placeholder primitive surface was removed.
- **`vars` and `examples`: PASS.** `WorkflowAst` retains both at `crates/vb-compiler/src/ast/types.rs:13-24`, populated in `crates/vb-compiler/src/ast/parse.rs:16-21`.
- **`ipc`, unknown trigger fields, unknown step fields: PASS.** Public `parse_ast` calls `compile_validated_document` at `crates/vb-compiler/src/lib.rs:132` before AST construction, so current compiler diagnostics remain authoritative.
- **Runtime YAML leakage: PASS.** YAML dependencies only appear in `crates/vb-compiler/Cargo.toml`; runtime crate manifests did not show YAML deps.
- **Mechanical size: PASS.** New AST files are split; measured AST implementation functions did not exceed 25 lines.
- **Forbidden Rust constructs: PASS.** Non-test AST implementation grep found no `unwrap()`, `expect()`, `panic!`, `unsafe`, `todo!`, `unimplemented!`, or `dbg!`.
- **Tests: PASS.** `crates/vb-compiler/src/ast/tests.rs:44-121` covers `vars`/`examples`, `ipc` rejection, unknown trigger field rejection, unknown step field rejection, and exact source mark index/line/column checks.

## Remaining caveats

- `parse_workflow_ast` is still callable inside `vb-compiler`, so future code can misuse it internally. That is tolerable for this bead because it is not public, but do not let some lazy follow-up wire it around `YamlCompiler::parse_ast`.
- `parse_ast` still performs a cold compile-parity pass before AST return. That is not hot-path behavior and is acceptable for this phase because it preserves current deterministic compiler behavior.

## Verification run

- `rtk cargo fmt --all -- --check` — PASS.
- `rtk cargo test -p vb-compiler` — PASS: 71 tests.
- `rtk cargo test --workspace --all-targets` — PASS: 158 tests.
- `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings` — PASS.

## Verdict

APPROVED. The repair now satisfies the bead scope without leaking YAML into runtime crates or advertising fake AST capabilities.
