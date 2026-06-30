# vb-wg64 State 4 Proof Strategy

## Scope

- Bead: `vb-wg64`
- State: 4 proof planning
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-wg64`
- Contract source: `.beads/vb-wg64/contract.md`
- Strategy type: executable CI repair gates plus diff review. No proof code, production code, test code, CI config, or verifier harness changes in this state.

## Discovery Evidence

State 4 discovery was run from the isolated workspace:

```bash
pwd -P
test -s ".beads/vb-wg64/contract.md" && test -s ".beads/vb-wg64/traceability-matrix.jsonl" && test -s ".beads/vb-wg64/delivery-scope.jsonl"
rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" "xtask/src/forbidden_scan.rs" "crates/vb_cli/src/app_impl.rs" "crates/vb_cli/src/mode_error.rs" "crates/vb_cli/src/commands_ai_context.rs" "crates/vb_cli/src/mode_activation_tests.rs" "crates/vb_storage/tests/recovery_bdd_tests.rs"
rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" "xtask/src/forbidden_scan.rs" "crates/vb_cli/src/app_impl.rs" "crates/vb_cli/src/mode_error.rs" "crates/vb_cli/src/commands_ai_context.rs" "crates/vb_cli/src/mode_activation_tests.rs" "crates/vb_storage/tests/recovery_bdd_tests.rs"
```

Results:

- `pwd -P` returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-wg64`.
- Required State 2-3 planning inputs exist and are non-empty.
- Scoped keyword discovery found CI/tooling and test risks: assertions, proptest usage in existing mode activation tests, `forbid(unsafe_code)` declarations, unsafe-pattern scanner strings, retry/cancel CLI text, and ignored recovery BDD cases.
- Discovery did not justify new TLA+, Verus, Kani, Loom, Miri, fuzz, or Flux proof work for this bead because the contract permits only formatting, lint-safe local rewrites, import/unused cleanup, and test-module resolution.

## Risk-To-Gate Strategy

The proof strategy is operational: each required contract property is proven by an executable gate and then by final forced CI.

| Risk | Contract Mapping | Required Evidence |
| --- | --- | --- |
| Workspace rustfmt drift | `REQ-CI-002`, `INV-CI-001` | `rtk cargo fmt --all -- --check` exits 0 |
| xtask strict source lint | `REQ-CI-003`, `INV-CI-003`, `INV-CI-004` | `rtk cargo clippy -p xtask --all-targets -- -D warnings` exits 0 plus diff review for checked indexing/arithmetic |
| vb_cli strict source lint and test module resolution | `REQ-CI-003`, `REQ-CI-005`, `REQ-CI-006`, `INV-CI-001`, `INV-CI-003` | `rtk cargo clippy -p vb_cli --all-targets -- -D warnings` exits 0 plus diff review for output behavior |
| vb_storage recovery BDD warning cleanup | `REQ-CI-004`, `INV-CI-002` | `rtk cargo check -p vb_storage --test recovery_bdd_tests` exits 0, with no assertion/setup deletion in diff review |
| Clean-clone release gate | `REQ-CI-001`, `INV-CI-005` | `moon ci --base HEAD --head HEAD --force` exits 0 |
| CI weakening or broad allowlist | `INV-CI-003`, disallowed changes | Diff review finds no Moon/Cargo/CI weakening and no broad allowlist |

## Exact Planned Commands

Run targeted preflight commands after implementation, before forced CI:

```bash
rtk cargo fmt --all -- --check
rtk cargo clippy -p xtask --all-targets -- -D warnings
rtk cargo clippy -p vb_cli --all-targets -- -D warnings
rtk cargo check -p vb_storage --test recovery_bdd_tests
```

Run final acceptance gate only after targeted gates pass or any residual unrelated failure is separately tracked:

```bash
moon ci --base HEAD --head HEAD --force
```

Optional broader recovery compile confirmation if the targeted test compile passes but full test compile risk remains:

```bash
rtk cargo check -p vb_storage --tests
```

## Non-Applicable Formal Lanes

- TLA+: not applicable for this CI repair; no temporal workflow behavior may change under the contract.
- Verus/Lean/Flux: not applicable; no new refinement/type-state proof boundary is introduced.
- Kani: not applicable unless implementation changes executable state logic beyond lint/format/module repair.
- Loom: not applicable; no concurrent runtime behavior is in scope.
- Miri: not required for this bead because baseline records scoped Miri passed and this contract does not allow unsafe or memory-model changes.
- Fuzz: not applicable; no parser/input boundary behavior change is allowed.

## Acceptance Rule

State 4 acceptance is the existence of the planned artifacts only. Later implementation states must not close `vb-wg64` until all required obligations in `.beads/vb-wg64/proof-obligations.planned.jsonl` are executed and the final forced `moon ci` gate exits 0.
