# Truth Serum Report - vb-2b4g

## Status

STATUS: APPROVED_WITH_RESIDUAL_RISKS

The audit was executed from the active workspace context after restoring the jj workspace to a child of the `vb-2b4g` implementation commit. The evidence supports scoped `vb_codegen` runtime parity, generated-source static contract confidence, compile/trybuild/fmt confidence, full local `vb_codegen` test confidence, and a strict production clippy panic-surface gate for the touched crate.

This report does not claim formal proof, theorem proof, mutation confidence, performance confidence, or global `moon ci` pass confidence.

## Execution Evidence

### Workspace Recovery

Initial `jj status` reported a stale working copy:

```text
Error: The working copy is stale (not updated since operation 9d64056c320f).
Hint: Run `jj workspace update-stale` to update it.
See https://docs.jj-vcs.dev/latest/working-copy/#stale-working-copy for more information.
```

The first test wrappers also used zsh reserved variable `status`; the cargo tests themselves passed, but the wrapper failed with `zsh:1: read-only variable: status`. These attempts were not counted as final proof. The commands were rerun with `rc` below.

```text
$ jj workspace update-stale
Concurrent modification detected, resolving automatically.
Working copy  (@) now at: pvputynw a23103ff (empty) (no description set)
Parent commit (@-)      : xxsyqsus 97be914f main | test: kill mutation survivors in canonical/validate functions
Added 29 files, modified 17 files, removed 50 files
Updated working copy to fresh commit a23103ff2a4d
```

```text
$ jj log -r 'pqomuxro | pqomuxro-'
○  pqomuxro priorlewis43@gmail.com 2026-05-17 15:31:02 ed267f78
│  go-skill vb-2b4g parity implementation
○  xxoyykps priorlewis43@gmail.com 2026-05-17 11:00:07 ab1117de
│  go-skill vb-qi37.10 final IR coverage
~
```

```text
$ jj new pqomuxro -m "go-skill vb-2b4g evidence finalization"
Working copy  (@) now at: yxnyornz 44fd4c50 (empty) go-skill vb-2b4g evidence finalization
Parent commit (@-)      : pqomuxro ed267f78 go-skill vb-2b4g parity implementation
Added 50 files, modified 17 files, removed 29 files
```

```text
$ jj status
The working copy has no changes.
Working copy  (@) : yxnyornz 44fd4c50 (empty) go-skill vb-2b4g evidence finalization
Parent commit (@-): pqomuxro ed267f78 go-skill vb-2b4g parity implementation
```

### Artifact Integrity

```text
$ pwd -P && test -s ".beads/vb-2b4g/assurance-bundle.md" && test -s ".beads/vb-2b4g/proof-review.md" && test -s ".beads/vb-2b4g/test-plan-review.md" && test -s ".beads/vb-2b4g/test-suite-review.md" && test -s ".beads/vb-2b4g/formal-verification-report.md" && test -s ".beads/vb-2b4g/verification-ledger.jsonl" && test -s ".beads/vb-2b4g/black-hat-review.md" && test -s ".beads/vb-2b4g/machine-gate-report.md" && test -s ".beads/vb-2b4g/regression-diff.md" && jq -c . ".beads/vb-2b4g/delivery-scope.jsonl" >/dev/null && jq -c . ".beads/vb-2b4g/traceability-matrix.jsonl" >/dev/null && jq -c . ".beads/vb-2b4g/verification-ledger.jsonl" >/dev/null && jq -c . ".beads/vb-2b4g/formal-waivers.jsonl" >/dev/null && rtk grep -n '^STATUS: APPROVED$' ".beads/vb-2b4g/proof-review.md" ".beads/vb-2b4g/test-plan-review.md" ".beads/vb-2b4g/test-suite-review.md" ".beads/vb-2b4g/contract-verification-review.md" ".beads/vb-2b4g/formal-verification-report.md" ".beads/vb-2b4g/black-hat-review.md"; rc=$?; printf 'exit=%s\n' "$rc"; exit "$rc"
/tmp/opencode/go-skill-vb-2b4g
6 matches in 6 files:

.beads/vb-2b4g/black-hat-review.md:3:STATUS: APPROVED
.beads/vb-2b4g/contract-verification-review.md:3:STATUS: APPROVED
.beads/vb-2b4g/formal-verification-report.md:3:STATUS: APPROVED
.beads/vb-2b4g/proof-review.md:34:STATUS: APPROVED
.beads/vb-2b4g/test-plan-review.md:1:STATUS: APPROVED
.beads/vb-2b4g/test-suite-review.md:1:STATUS: APPROVED
exit=0
```

### Runtime Parity Gates

```text
$ rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture && rtk cargo test -p vb_codegen reduce_generated_parity -- --nocapture && rtk cargo test -p vb_codegen together_generated_parity -- --nocapture && rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture; rc=$?; printf 'exit=%s\n' "$rc"; exit "$rc"
cargo test: 3 passed, 364 filtered out (3 suites, 0.13s)
cargo test: 3 passed, 364 filtered out (3 suites, 0.28s)
cargo test: 2 passed, 365 filtered out (3 suites, 0.14s)
cargo test: 3 passed, 364 filtered out (3 suites, 0.40s)
exit=0
```

```text
$ rtk cargo test -p vb_codegen generated_source_contract -- --nocapture; rc=$?; printf 'exit=%s\n' "$rc"; exit "$rc"
cargo test: 3 passed, 364 filtered out (3 suites, 0.02s)
exit=0
```

```text
$ rtk cargo test -p vb_codegen journal_signature_generated_parity -- --nocapture; rc=$?; printf 'exit=%s\n' "$rc"; exit "$rc"
cargo test: 1 passed, 366 filtered out (3 suites, 0.54s)
exit=0
```

```text
$ rtk cargo test -p vb_codegen -- --nocapture; rc=$?; printf 'exit=%s\n' "$rc"; exit "$rc"
cargo test: 367 passed (4 suites, 3.08s)
exit=0
```

### Compile, Trybuild, Fmt, And PO-007

```text
$ rtk cargo test -p vb_codegen --test trybuild_tests && rtk cargo fmt --check && rtk cargo check -p vb_codegen --all-targets --all-features; rc=$?; printf 'exit=%s\n' "$rc"; exit "$rc"
cargo test: 3 passed (1 suite, 0.36s)
cargo build (1 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.38s
exit=0
```

```text
$ /home/lewis/.cargo/bin/cargo check -p vb_codegen --all-targets && /home/lewis/.cargo/bin/cargo test -p vb_codegen --test trybuild_tests && /home/lewis/.cargo/bin/cargo fmt --all -- --check; rc=$?; printf 'exit=%s\n' "$rc"; exit "$rc"
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/trybuild_tests.rs (target/debug/deps/trybuild_tests-80cba9a8d58856d4)

running 3 tests
test trybuild_compile_fail_tests_fails_when_compile_fail_fixture_dir_is_empty ... ok
   Compiling vb_codegen-tests v0.0.0 (/tmp/opencode/go-skill-vb-2b4g/target/tests/trybuild/vb_codegen)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s


test /tmp/opencode/go-skill-vb-2b4g/crates/vb_codegen/tests/compile-fail/pass/minimal_workflow.rs ... ok

test trybuild_pass_tests ... ok
    Checking vb_codegen-tests v0.0.0 (/tmp/opencode/go-skill-vb-2b4g/target/tests/trybuild/vb_codegen)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s


test /tmp/opencode/go-skill-vb-2b4g/crates/vb_codegen/tests/compile-fail/forbid_yaml_import.rs ... ok
test /tmp/opencode/go-skill-vb-2b4g/crates/vb_codegen/tests/compile-fail/forbid_unwrap.rs ... ok
test /tmp/opencode/go-skill-vb-2b4g/crates/vb_codegen/tests/compile-fail/forbid_unsafe.rs ... ok
test /tmp/opencode/go-skill-vb-2b4g/crates/vb_codegen/tests/compile-fail/forbid_unchecked_indexing.rs ... ok
test /tmp/opencode/go-skill-vb-2b4g/crates/vb_codegen/tests/compile-fail/forbid_panic.rs ... ok

test trybuild_compile_fail_tests ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s

exit=0
```

### Production Panic-Surface Gate

```text
$ rtk cargo clippy -p vb_codegen --lib --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use; rc=$?; printf 'exit=%s\n' "$rc"; exit "$rc"
cargo clippy: No issues found
exit=0
```

```text
$ rtk grep -n '(^|[^A-Za-z0-9_])(assert!|assert_eq!|assert_ne!|unreachable!)' crates/vb_codegen/src --glob '*.rs' --glob '!crates/vb_codegen/src/tests.rs' --glob '!crates/vb_codegen/src/proptests.rs'; rc=$?; printf 'exit=%s\n' "$rc"; exit 0
0 matches for '(^|[^A-Za-z0-9_])(assert!|assert_eq!|assert_ne!|unreachable!)'
exit=1
```

The scanner exit `1` is the expected no-match exit from `rg`; the command wrapper exited successfully after printing it.

```text
$ rtk grep -n 'vb_core::.*run_until_blocked|use vb_core::.*run_until_blocked' crates/vb_codegen/src/lib.rs crates/vb_codegen/src/tests.rs; rc=$?; printf 'search_exit=%s\n' "$rc"; exit 0
0 matches for 'vb_core::.*run_until_blocked|use vb_core::.*run_until_blocked'
search_exit=1
```

```text
$ rtk grep -n 'not_yet_implemented' crates/vb_codegen/src/lib.rs; rc=$?; printf 'search_exit=%s\n' "$rc"; exit 0
0 matches for 'not_yet_implemented'
search_exit=1
```

## Empathetic User Review

This bead is developer-facing, not end-user CLI work. The developer experience is mostly acceptable for scoped verification: the focused command set is short, deterministic, and produces clear pass counts. The two real friction points are outside the generated parity implementation: jj stale-working-copy handling can put the operator on the wrong parent if not checked, and `moon ci` currently fails under disk quota/resource pressure rather than producing release-confidence evidence.

No raw user-facing stack traces were observed in the commands run for this audit. The `zsh:1: read-only variable: status` error came from the audit wrapper, not the product code, and was corrected by rerunning with `rc`.

## Skeptical QA Review

The scoped `vb_codegen` evidence is strong for executable parity: repeat, reduce, together, collect, journal signature, generated-source contract, trybuild, full local package tests, fmt, cargo check, and strict production clippy all passed in the active context.

The audit did not find production assertion macros in the touched crate after excluding test-only modules, did not find forbidden `vb_core::run_until_blocked` imports or qualified calls in the touched codegen files, and did not find `not_yet_implemented` in `crates/vb_codegen/src/lib.rs`.

The remaining risks are disclosed rather than hidden:

- `moon ci` remains `DEFERRED_GLOBAL` because prior evidence shows disk quota/resource failures.
- Mutation evidence is absent.
- Formal/TLA+/Verus/Kani/theorem evidence is waived or not in scope, not passed.
- Runtime `RunFinished` terminal event evidence is synthesized by the test helper, which is acceptable for this bead but remains a harness-design residual risk.

## Mandated Improvements

- Free disk quota/resources and rerun `moon ci` before making final release confidence claims.
- Keep mutation confidence out of this bead's claims until a scoped mutation run is executed and reviewed.
- Keep formal/theorem confidence out of this bead's claims until the follow-up formal beads complete and are reviewed.
- Prefer native runtime terminal-event exposure in future runtime/oracle work so tests do not need to synthesize `RunFinished` evidence.

## Verdict

Truth Serum approves `vb-2b4g` for scoped evidence finalization and scoped landing consideration with residual risks disclosed. Truth Serum does not approve any global release-confidence, formal-proof, theorem-proof, mutation, or performance claim.
