# Proof Strategy: vb-f04l State 4 Attempt 3

## Scope

- Bead: `vb-f04l`.
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l`.
- Role: go-skill State 4 proof-planner.
- Write scope: `.beads/vb-f04l/proof-strategy.md`, `.beads/vb-f04l/proof-plan-review-input.md`, `.beads/vb-f04l/proof-obligations.planned.jsonl`, and `.beads/vb-f04l/STATE.md` append only.
- Source checkout writes: none.

## Replan Trigger

State 6 rejected prior proof work for vacuous Verus proofs, TLA+ graph-shape assumptions, and stale PO-range evidence mapping. Repaired State 3 now defines 49 canonical obligations and complete traceability for PRE/POST/INV/ERR clauses. Prior State 5 proof evidence is context only and is not accepted as current PASS evidence.

## Discovery

- `pwd -P`: exit 0, `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l`.
- `test -s .beads/vb-f04l/contract.md && test -s .beads/vb-f04l/traceability-matrix.jsonl && test -s .beads/vb-f04l/delivery-scope.jsonl`: exit 0.
- `rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" ...scoped files...`: exit 0, 137 matches in 11 files.
- `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" ...scoped files and verification artifacts...`: exit 0, 34 matches in 12 files.
- Blocked discovery commands: none.

## Proof Lanes

- TLA+: required for POST-006..POST-012 lifecycle/progress and INV-002 bounded lifecycle safety.
- Verus: required for PRE-007, POST-003..POST-012 local shape/bounds/dense-index/slot/determinism, and INV-001/003/004/005.
- Focused cargo tests: required for canonical admission, error taxonomy, validation bridge, coverage matrix, and Set/Finish regression.
- `moon ci`: required for canonical repository gate, unsupported primitive policy, runtime dependency boundary, forbidden production constructs, and no hidden legacy deletion.
- Kani, Loom, Miri, Flux, fuzz, and Lean are recorded explicitly as not-applicable or waived rows, with follow-up triggers.

## State 5 Repair Requirements

- Verus proof must use abstract lowering constructors/transitions or bridge invariants; it must not require the property it ensures.
- TLA+ must either model emitted graph structure enough to reject malformed shapes, or state a narrowed lifecycle-only claim over prevalidated graphs while Verus/test rows own graph shape.
- Proof evidence must map canonical IDs from this planned JSONL and repaired `proof-obligations.jsonl`; stale `PO-001..PO-013` ranges are not acceptable.

---

# State 4 Repair After State 11 Rejection

## Rejection Trigger

State 11 rejected this plan because 19 cargo-test proof obligation commands used stale filters such as `empty_steps`, `validation_success_path`, and `err_canonical_yaml`. Those commands exited 0 while selecting zero tests, except `duplicate_step_id`, which selected one unrelated legacy unit test and did not prove the planned top-level+nested duplicate-step evidence.

## Repair Decision

- Keep production code, tests, proof source, dependencies, and CI config unchanged.
- Repair only State 4 planning artifacts so exact cargo commands target the real State 8 integration test artifact: `crates/vb_compile/tests/v1_primitive_lowering.rs`.
- Replace stale bare filters with `cargo test -p vb_compile --test v1_primitive_lowering <real_test_name>` where one real test carries the obligation evidence.
- Use `cargo test -p vb_compile --test v1_primitive_lowering` for `INV-007`, because that obligation is a coverage-matrix claim over the whole target rather than one test function.

## Repaired Cargo Obligation Mapping

| Obligation IDs | Repaired command | Selection evidence |
|---|---|---|
| PRE-001, PRE-003, PRE-004, PRE-005, ERR-001, ERR-002, ERR-003, ERR-004 | `cargo test -p vb_compile --test v1_primitive_lowering compile_source_returns_exact_error_variants_for_contract_taxonomy` | `1 passed, 14 filtered out` |
| PRE-002 | `cargo test -p vb_compile --test v1_primitive_lowering yaml_compiler_compile_emits_supported_ir_when_each_scoped_primitive_is_valid` | `1 passed, 14 filtered out` |
| PRE-006, ERR-007 | `cargo test -p vb_compile --test v1_primitive_lowering compile_workflow_returns_step_field_shape_when_each_scoped_primitive_required_field_is_empty` | `1 passed, 14 filtered out` |
| POST-002, ERR-009 | `cargo test -p vb_compile --test v1_primitive_lowering public_helpers_return_exact_step_index_slot_index_limit_and_workflow_error_variants` | `1 passed, 14 filtered out` |
| POST-013, ERR-005, ERR-006 | `cargo test -p vb_compile --test v1_primitive_lowering public_compile_apis_preserve_set_and_terminal_finish_regression` | `1 passed, 14 filtered out` |
| INV-007 | `cargo test -p vb_compile --test v1_primitive_lowering` | `15 passed` |
| ERR-008 | `cargo test -p vb_compile --test v1_primitive_lowering public_lowering_helpers_return_exact_range_and_workflow_errors` | `1 passed, 14 filtered out` |
| ERR-010 | `cargo test -p vb_compile --test v1_primitive_lowering yaml_compiler_compile_returns_canonical_yaml_when_source_parse_fails` | `1 passed, 14 filtered out` |

## Completion Evidence

- Isolation command: `pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac`; exit 0.
- Repaired artifacts: `.beads/vb-f04l/proof-strategy.md`, `.beads/vb-f04l/proof-plan-review-input.md`, `.beads/vb-f04l/proof-obligations.jsonl`, `.beads/vb-f04l/proof-obligations.planned.jsonl`, `.beads/vb-f04l/STATE.md`.
- JSONL validation: required after this repair before State 11 retry consumes the files.
