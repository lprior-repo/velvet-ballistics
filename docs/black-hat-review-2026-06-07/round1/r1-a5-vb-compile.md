# R1-A5: vb_compile Inventory

**Agent:** explore · **Date:** 2026-06-07
**Scope:** `crates/vb_compile/` (YAML → IR lowering, validation, ID binding, Section 65 gates)
**Files:** 113 .rs files, 28,491 LoC production + 9,103 LoC test = 37,594 LoC total
**Module tree:** lib.rs + ast/, mod_compile_core.rs, mod_compile_lowering/, mod_compile_errors/, mod_compile_validation/, mod_compile_idempotency/

## File Counts

| Type | Count | LoC |
|------|------:|----:|
| .rs production | 64 | 19,892 |
| .rs test | 38 | 6,501 |
| .rs kani harnesses | 4 | 1,289 |
| .rs proptest | 7 | 1,719 |
| **Total** | **113** | **37,594** |

Largest 5 files:
1. `crates/vb_compile/src/lib.rs` — 712 LoC (compile_workflow entry point + module declarations)
2. `crates/vb_compile/src/mod_compile_core.rs` — 1,156 LoC (Section 65 + Section 15 orchestration)
3. `crates/vb_compile/src/mod_compile_lowering/part_05.rs` — 921 LoC (set/do/choose lowering)
4. `crates/vb_compile/src/mod_compile_lowering/part_07.rs` — 823 LoC (for_each/together/collect)
5. `crates/vb_compile/src/mod_compile_lowering/part_09.rs` — 856 LoC (wait/ask/finish)

## Public API

- `compile_workflow(&[u8]) -> Result<CompiledWorkflow, CompileErrorList>`
- `CompiledWorkflow::try_from_parts(parts: WorkflowParts) -> Result<Self, CoreError>` (resource contract admission)
- `compile_step_primitive(StepAst) -> Result<CompiledNode, CompileError>`

## 22 of 34 IR Variants Emitted

`crates/vb_compile/src/mod_compile_lowering/` has 12 part files. Each part lowers a set of YAML primitives. The 22 emitted IR variants cover:
- All Set/Copy/EvalExpr/BuildObject/BuildList
- All Do/Choose/ChooseSlot
- All ForEachStart/ForEachNext/ForEachJoin
- All TogetherStart/TogetherBranch/TogetherJoin
- All CollectStart/CollectPage/CollectNext/CollectFinish
- All RepeatStart/RepeatAttempt/RepeatCheck/RepeatFinish
- WaitUntil/WaitEvent
- Ask/AskResume
- RetryCheck
- ErrorHandler
- Finish

NOT emitted (12 reserved for future use): Nop, ReduceStart/Next/Finish, Repeat attempts > 1, OnError body.

## §65 SideEffect/RetrySafety Drift

Master §65 requires:
- `SideEffect { Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite, Process, UnsafeShell }` (7 variants)
- `RetrySafety { Idempotent, RequiresIdempotencyKey, NotRetrySafe, Unknown }` (4 variants)

Production has:
- `SideEffect { None, Writes, Sends, Creates, Destroys }` (5 variants)
- `RetrySafety { Safe, KeyRequired, Unsafe }` (3 variants)

**The 5→7 and 3→4 mapping is not 1:1. The 5 production variants encode external-vs-local, but the master 7 split this into read/write/process/unsafe-shell. A `Process` action CANNOT be declared in production.**

## 3 Production Gates Enforce Broken Taxonomy

`is_compile_idempotency_gate_accepted`, `check_idempotency_gates`, `is_statically_idempotent_contract` — all 3 use the broken 5×3 taxonomy. A migration to 7×4 would require:
- Rename all 5 + 3 variants
- Update 28+ call sites
- Update 119 hits in `idempotency_contract_red.rs`
- Update 45 cases in `idempotency_parity.rs` and `kani_idempotency_parity.rs`
- Rewrite the 3 gates

The test files at `crates/vb_compile/src/enums/tests/side_effect_tests.rs` and `retry_safety_tests.rs` assert the MASTER 7×4 taxonomy (40 + 32 cases) but are DEAD CODE (not declared in `lib.rs`).

## mod restrictions NOT Declared

`crates/vb_compile/src/lib.rs:14-26`:
```rust
pub mod ast;
pub mod compile_idempotency;
pub mod mod_compile_core;
pub mod mod_compile_errors;
pub mod mod_compile_lowering;
pub mod mod_compile_validation;
// mod restrictions;  <-- MISSING (line 64 has a lying "TEMPORARILY DISABLED" comment)
pub mod proptest_idempotency;
```

`crates/vb_compile/src/restrictions.rs` is a 10-line stub. 19 tests in `restrictions/tests/attempt_number_tests.rs` are dead code.

## Cold AST Repeat has No Body

`crates/vb_compile/src/ast/types.rs:173`:
```rust
Repeat { max_attempts: u16 },
```

The parser at `parse_repeat:381-385` reads only `max_attempts` and SILENTLY DROPS the `steps:` body of every `repeat:` in user YAML. **A user who writes `repeat: { max_attempts: 3, steps: [log_attempt] }` gets a workflow with an empty repeat body.**

## Files Over 300 Lines

48 files over 300 lines, including:
- `mod_compile_core.rs` (1,156)
- `compiled_slug.rs` (583 — canonical production seam, NOT in source-length ledger)
- `lib.rs` (712)
- 8 part files in `mod_compile_lowering/`
- 7 part files in `mod_compile_validation/`

## Forbidden Pattern Audit

| Pattern | Production | Test |
|---------|----------:|-----:|
| `unwrap()` | 0 | 23 (test only) |
| `expect()` | 0 | 11 (test only) |
| `panic!()` | 0 | 1 (test only) |
| `unsafe` | 0 | 0 |

## verdict

**52 / 100 — Compiles, all primitives lower, but the §65 taxonomy drift is the most dangerous defect.**

Top concerns:
1. SideEffect 5 vs 7, RetrySafety 3 vs 4 (LETHAL drift; MAJOR-6 not filed)
2. 22/34 IR variants emitted (12 reserved)
3. `mod restrictions;` not declared (19 dead tests)
4. Cold AST Repeat has no body (silent body drop)
5. 48 files over 300 lines
6. 2 dead duplicate IR files
