STATUS: PASS

# Moon `--no-actions` Diagnostic: vb-qi37.7.3

## Question Under Test

Does Moon source setup/sync action execution explain tracked repair files reverting or becoming unformatted during State 8 validation?

## Verdict

No. The forced Moon run with actions disabled still observed the same failure class: formatter diffs, missing public APIs, and compile/lint failures. This means the mutation/reverted-state symptom is reproducible without Moon action execution and should be routed as workspace/state corruption or stale-tree reconciliation, not as a Moon setup/sync action side effect.

## Commands / Evidence

### Initial exact formatter check

Command:

```bash
rustup run nightly-2026-04-28 cargo fmt --all --check
```

Working directory:

```text
/home/lewis/src/Velvet-ballistics-femdation-p0p1-25
```

Observed result:

```text
exit: 0
stdout: <empty>
stderr: <empty>
```

### Forced Moon CI with actions disabled

Command:

```bash
GIT_DIR=/home/lewis/src/Velvet-ballistics/.git GIT_WORK_TREE=/home/lewis/src/Velvet-ballistics-femdation-p0p1-25 moon ci --force --no-actions
```

Output artifact:

```text
/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/moon-ci-force-no-actions.out
```

Observed result:

```text
EXIT_STATUS:1
Tasks: 4 completed, 6 failed, 10 skipped
```

Key observed lines:

```text
velvet-ballastics:lint-src | error: used `assert_eq!` with a literal bool
velvet-ballastics:nightly-feature-cargo-probe | error[E0432]: unresolved imports `vb_core::workflow::validate_resource_references`, `vb_core::workflow::validate_symbol_references`
velvet-ballastics:nightly-feature-cargo-probe | error[E0432]: unresolved import `vb_validate::shared::validate_action_references`
velvet-ballastics:check | error[E0432]: unresolved import `vb_validate::shared::validate_action_references`
velvet-ballastics:check | error[E0432]: unresolved imports `vb_core::workflow::validate_resource_references`, `vb_core::workflow::validate_symbol_references`
velvet-ballastics:fmt | Diff in /home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_compile/src/lib.rs:3797:
velvet-ballastics:fmt | Diff in /home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballastics/src/main.rs:3469:
velvet-ballastics:fmt | Diff in /home/lewis/src/Velvet-ballistics-femdation-p0p1-25/xtask/tests/integration_gates.rs:519:
```

### Post-no-actions exact formatter check

Command:

```bash
GIT_DIR=/home/lewis/src/Velvet-ballistics/.git GIT_WORK_TREE=/home/lewis/src/Velvet-ballistics-femdation-p0p1-25 rustup run nightly-2026-04-28 cargo fmt --all --check
```

Observed result:

```text
exit: non-zero
actual: formatter diffs remained after the `--no-actions` run
expected: no formatter diffs if the initial clean tree remained stable
```

### Required API surface search after no-actions run

Search result in implementation workspace:

```text
matches only in tests:
/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_core/tests/vb_qi37_7_3_red.rs
/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_validate/tests/vb_qi37_7_3_red.rs
```

Expected implementation surface from `/home/lewis/src/Velvet-ballistics/.beads/vb-qi37.7.3/implementation.md`:

```text
vb_core::workflow::validate_symbol_references(parts: &WorkflowParts) -> Result<(), WorkflowError>
vb_core::workflow::validate_resource_references(parts: &WorkflowParts) -> Result<(), WorkflowError>
vb_validate::shared::validate_action_references(parts: &WorkflowParts, action_contracts: &[ActionContract]) -> Result<(), ValidationError>
```

Actual: implementation definitions were absent from source after the no-actions run.

## Failure Classification

- Existing category file: `ci-failure-category.txt` says `FORMAT`.
- In the no-actions output, `velvet-ballastics:fmt` explicitly emitted diffs, so `FORMAT` remains valid.
- First explicit error line by output order is `velvet-ballastics:lint-src | error: used assert_eq! with a literal bool`, so a secondary classification is `LINT`.
- Compile/API regressions are also present via unresolved imports for the three bead APIs.

## Source Mutation / Reversion Conclusion

The initial exact formatter check passed. The `moon ci --force --no-actions` run then observed formatter diffs and missing implementation APIs that should have been present according to the implementation evidence. Since `--no-actions` disables Moon actions, the observed source-state regression is not explained by Moon setup/sync actions. Route as State 8 workspace/state corruption, stale checkout/JJ/Git worktree reconciliation, or uncommitted repair visibility problem.

## State 8 Routing Recommendation

Do not accept this bead into final State 8 validation on the current workspace state. First reconcile the implementation workspace so the effective `GIT_WORK_TREE` contains:

1. rustfmt-clean source,
2. restored public API definitions,
3. passing focused red suite,
4. stable Git/JJ status without source disappearing between exact formatter check and Moon execution.

Then rerun `moon ci` from a clean, committed or otherwise stable tree.
