# vb-8eocj.2 Source Type Identity Tests — v18

Status: two numeric fixture syntax corrections applied, tests NOT RUN
Workspace: /home/lewis/src/isoloated/vb-f1-contract-local3090-v1
Git root: /home/lewis/src/isoloated/vb-f1-contract-local3090-v1
JJ root: /home/lewis/src/isoloated/vb-f1-contract-local3090-v1

## Files

Test file: crates/workspace_tests/tests/type_identity.rs
Bead doc: .beads/vb-8eocj.2/source-type-tests-v18.md

## Changes from v17

V17 was pushed but contained a REAL RUST SYNTAX ERROR in two numeric fixture strings.
The inner result value was written as a quoted string `"result"` inside the Rust
string literal, producing unescaped inner quotes that break Rust compilation.

This v18 applies the exact two-line source-string fix:

- Line 36: `finish: { result: "result" }` → `finish: { result: result }` (yaml_numeric_producer)
- Line 42: `finish: { result: "result" }` → `finish: { result: result }` (yaml_numeric_qualified)

Both changes convert the YAML finish result from a quoted string to a bare YAML
value, matching the style of the other fixture strings in the file. The fix
removes the unescaped inner quotes that caused the syntax error.

## What did NOT change

- All seven test semantics preserved (5 negative compile-error, 2 positive runtime).
- Distinct producer step ID (`producer`) vs output slot (`flag`) pattern unchanged.
- All 20 original tests in the suite unchanged.
- No production code edits.
- No Cargo, moon, test, or verification gates invoked.
- Source typed references (`$producer`, `$step.producer`, `$steps.producer`) unchanged.

## Verification

Read back of crates/workspace_tests/tests/type_identity.rs confirms:
- No unescaped inner quote remains in any fixture string.
- Lines 36 and 42 now contain `result: result\n` (bare YAML value).
- All other fixture strings use escaped inner quotes (`\"`) as before.
