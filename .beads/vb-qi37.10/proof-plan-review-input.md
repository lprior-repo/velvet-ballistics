# Proof Plan Review Input: vb-qi37.10

## Reviewer Focus

Review whether the planned proof obligations are narrow to `vb-qi37.10`, risk-triggered, non-vacuous, and executable by later go-skill states without modifying formal/proof files outside the bead artifact directory during State 4.

## Inputs Read

- `.beads/vb-qi37.10/STATE.md`
- `.beads/vb-qi37.10/codebase-map.md`
- `.beads/vb-qi37.10/delivery-scope.jsonl`
- `.beads/vb-qi37.10/contract.md`
- `.beads/vb-qi37.10/verification-layers.md`
- `.beads/vb-qi37.10/proof-obligations.jsonl`
- `.beads/vb-qi37.10/traceability-matrix.jsonl`

## Discovery Commands

```bash
pwd -P && test -s ".beads/vb-qi37.10/contract.md" && test -s ".beads/vb-qi37.10/traceability-matrix.jsonl" && test -s ".beads/vb-qi37.10/delivery-scope.jsonl"
rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" "crates/vb_codegen/src/lib.rs" "crates/vb_codegen/src/generated_storage_helpers.rs.txt" "crates/vb_codegen/src/tests.rs" "crates/vb_codegen/tests/trybuild_tests.rs" "crates/vb_runtime/src/engine/execute.rs" "crates/vb_runtime/src/primitives/together.rs" "crates/vb_runtime/src/primitives/collect.rs" "crates/vb_runtime/src/primitives/reduce.rs" "crates/vb_runtime/src/primitives/repeat.rs" "crates/vb_core/src/workflow/mod.rs" "crates/vb_core/src/engine/expr_eval/accessors.rs" "crates/vb_core/src/engine/expr_eval/ops.rs" "crates/vb_expr/src/builtin_eval.rs" "fuzz/src/lib.rs" "fuzz/src/bin/generated_compare.rs" "fuzz/fuzz_targets.rs"
rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" "crates/vb_codegen/src/lib.rs" "crates/vb_codegen/src/generated_storage_helpers.rs.txt" "crates/vb_codegen/src/tests.rs" "crates/vb_codegen/tests/trybuild_tests.rs" "crates/vb_runtime/src/engine/execute.rs" "crates/vb_runtime/src/primitives/together.rs" "crates/vb_runtime/src/primitives/collect.rs" "crates/vb_runtime/src/primitives/reduce.rs" "crates/vb_runtime/src/primitives/repeat.rs" "crates/vb_core/src/workflow/mod.rs" "crates/vb_core/src/engine/expr_eval/accessors.rs" "crates/vb_core/src/engine/expr_eval/ops.rs" "crates/vb_expr/src/builtin_eval.rs" "fuzz/src/lib.rs" "fuzz/src/bin/generated_compare.rs" "fuzz/fuzz_targets.rs"
```

## Discovery Result Summary

- Workspace guard passed in `/tmp/opencode/go-skill-vb-qi37-10`.
- Risk scan returned stateful generated/runtime surfaces, retry/collect state, serialization in runtime oracle files, generated source `forbid(unsafe_code)`, source-test assertions, and fuzz entry points.
- Formal-artifact scan did not find production-bound `kani::`, Verus `requires/ensures/proof fn`, Flux, Loom, Miri, or TLA artifacts for this bead scope.

## Planned Required Lanes

- `PO-001` support/rejection totality by executable tests.
- `PO-002` generated executable parity for `Repeat*`.
- `PO-003` generated executable parity for `Reduce*`.
- `PO-004` generated executable parity for `Together*`.
- `PO-005` generated executable parity or blocker-grade fail-closed evidence for `Collect*`.
- `PO-006` expression/accessor helper value and error parity.
- `PO-007` taint parity across helper/accessor/build/join/finish surfaces.
- `PO-008` text helper support or exact fail-closed rejection with blocker.
- `PO-009` generated source compile/rustfmt/clippy/static forbidden-construct gate.
- `PO-010` non-empty trybuild compile-fail gate.
- `PO-011` journal-signature parity.
- `PO-012` final focused/repository gate.

## Deferred Or Non-Applicable Lanes

- `PO-013` TLA+ is deferred as follow-up because production-bound model/config files do not exist; if created later, it must be bounded and include typed Err/overflow states.
- `PO-014` Verus is deferred as follow-up because no non-vacuous proof target binds to production codegen/helper APIs.
- `PO-015` Kani is deferred as follow-up because no production-bound harness exists; future harnesses must not use hardcoded dummy shapes.
- `PO-016` fuzz build is conditional and required only if generated-compare fuzz files are touched.
- Loom, Flux, and Miri are not applicable unless implementation changes introduce their trigger risks.

## Review Questions

- Is it acceptable for this bead to rely on executable generated-vs-runtime parity rather than blocking on new production-bound TLA/Verus/Kani targets?
- Should `Collect*` unsupported status be treated as non-closable for `vb-qi37.10` unless implementation adds parity or an approved blocker updates scope?
- Are the planned test names acceptable as exact commands for State 6, or should implementation choose existing repository test names and update evidence accordingly?
