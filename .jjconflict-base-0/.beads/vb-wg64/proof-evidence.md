# vb-wg64 Proof Evidence Ledger

## Scope

- Bead: `vb-wg64`
- State: 5 proof-writer evidence scaffold
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-wg64`
- Contract: `.beads/vb-wg64/contract.md`
- Planned obligations: `.beads/vb-wg64/proof-obligations.planned.jsonl`

This file records current pre-repair evidence references and the post-repair commands that State 11 must execute. It does not claim any post-repair gate has passed.

## Current Pre-Repair Evidence References

| Evidence | Reference | Status |
| --- | --- | --- |
| Clean-clone forced CI failed before repair | `.beads/vb-wg64/baseline-report.md:8` | FAIL_PRE_REPAIR |
| Known failing lanes: fmt, lint-src, check | `.beads/vb-wg64/baseline-report.md:10-15` | FAIL_PRE_REPAIR |
| `rtk cargo fmt --all -- --check` showed workspace formatting drift | `.beads/vb-wg64/codebase-map.md:20`, `.beads/vb-wg64/codebase-map.md:55-57`, `.beads/vb-wg64/codebase-map.md:71` | FAIL_PRE_REPAIR |
| `rtk cargo clippy -p xtask --all-targets -- -D warnings` failed in `xtask/src/forbidden_scan.rs` | `.beads/vb-wg64/codebase-map.md:18-27`, `.beads/vb-wg64/codebase-map.md:72` | FAIL_PRE_REPAIR |
| `rtk cargo clippy -p vb_cli --all-targets -- -D warnings` failed in `vb_cli` mapped files | `.beads/vb-wg64/codebase-map.md:29-44`, `.beads/vb-wg64/codebase-map.md:73` | FAIL_PRE_REPAIR |
| `rtk cargo check -p vb_storage --tests` exited 0 with recovery BDD warnings | `.beads/vb-wg64/codebase-map.md:46-51`, `.beads/vb-wg64/codebase-map.md:74` | WARN_PRE_REPAIR |
| Scoped Miri checks passed in previous truth-serum run | `.beads/vb-wg64/baseline-report.md:15` | PASS_PRE_REPAIR_SCOPE_ONLY |
| Contract requires final forced CI and forbids CI weakening | `.beads/vb-wg64/contract.md:13-21`, `.beads/vb-wg64/contract.md:88-105` | CONTRACT |
| State 4 strategy marked formal lanes not applicable | `.beads/vb-wg64/proof-strategy.md:65-72` | PLANNED_WAIVER |

## Obligation-To-Evidence Binding

| Obligation | Required State 11 Evidence | Current State 5 Status |
| --- | --- | --- |
| `PO-001` | `rtk cargo fmt --all -- --check` exits 0 after repair. | NOT_RUN_POST_REPAIR |
| `PO-002` | `rtk cargo clippy -p xtask --all-targets -- -D warnings` exits 0; diff review confirms checked access/arithmetic and no broad lint suppression. | NOT_RUN_POST_REPAIR |
| `PO-003` | `rtk cargo clippy -p vb_cli --all-targets -- -D warnings` exits 0; diff review confirms `mode_error` resolution and unchanged `json_out` output semantics. | NOT_RUN_POST_REPAIR |
| `PO-004` | `rtk cargo check -p vb_storage --test recovery_bdd_tests` exits 0; diff review confirms recovery BDD assertions, setup side effects, and scenario intent remain. | NOT_RUN_POST_REPAIR |
| `PO-005` | `moon ci --base HEAD --head HEAD --force` exits 0 in the isolated workspace after all repairs. | NOT_RUN_POST_REPAIR |
| `PO-006` | `git diff -- xtask/src/forbidden_scan.rs crates/vb_cli/src/app_impl.rs crates/vb_cli/src/mode_error.rs crates/vb_cli/src/commands_ai_context.rs crates/vb_cli/src/mode_activation_tests.rs crates/vb_storage/tests/recovery_bdd_tests.rs Cargo.toml .moon moon.yml .cargo` reviewed for allowed changes only. | NOT_RUN_POST_REPAIR |
| `PO-007` | Scope review proves State 4 was artifact-only; State 5 adds only proof-writer artifacts and `STATE.md`. | SATISFIED_FOR_STATE_5_SCOPE |
| `PO-008` | No formal artifacts required unless implementation exceeds allowed CI repair categories. | WAIVED_NOT_APPLICABLE |

## Planned Post-Repair Commands

State 11 machine execution must run these commands from `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-wg64` and record exact exit status plus relevant stdout/stderr summaries:

```bash
rtk cargo fmt --all -- --check
rtk cargo clippy -p xtask --all-targets -- -D warnings
rtk cargo clippy -p vb_cli --all-targets -- -D warnings
rtk cargo check -p vb_storage --test recovery_bdd_tests
moon ci --base HEAD --head HEAD --force
```

Optional broader confirmation if the targeted recovery BDD check passes but broader test compile risk remains:

```bash
rtk cargo check -p vb_storage --tests
```

## Formal Lane Status

No formal proof commands are required in State 5.

| Lane | Status | Rationale |
| --- | --- | --- |
| TLA+ | NOT_APPLICABLE | No temporal workflow behavior change is allowed. |
| Verus/Lean/Flux | NOT_APPLICABLE | No refinement, theorem, or type-state boundary changes are allowed. |
| Kani | NOT_APPLICABLE | No executable state logic change is allowed beyond lint-safe repair categories. |
| Loom | NOT_APPLICABLE | No concurrency behavior change is allowed. |
| Miri | NOT_REQUIRED_FOR_THIS_REPAIR | Baseline scoped Miri passed and no unsafe or memory-model changes are allowed. |
| proptest/fuzz | NOT_APPLICABLE | No parser or input-boundary behavior change is allowed. |

Formal lanes must be reopened if a later implementation changes runtime behavior, unsafe boundaries, parser behavior, concurrency, storage recovery semantics, state transitions, or any contract invariant.
