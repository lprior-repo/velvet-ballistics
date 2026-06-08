# Round 3: Master-Contract Gap Analysis — Transcript Index (12 agents)

**Run date:** 2026-06-07 · **Workspace:** /home/lewis/src/velvet-ballistics
**Subagent type:** general (12 agents in parallel, one per master section)

## Score Matrix

| Section | Score | Top Drift |
|---------|------:|-----------|
| §13 Resource contracts | 64 | `BoundednessPolicy::DEFAULT` NOT on admission path; `MAX_STEPS=65_535` vs master 1000; 2 extra fields |
| §14 Core types | 99 | All 14 ID types, 4 value types, 18 CoreError ✓; `ids.rs` → `ids/mod.rs` path drift |
| §15 Final IR | 68 | 22/34 IR variants emitted; `LoadAccessor` missing; `ExprOp` 29 vs 30; dead `nodes.rs`/`expressions.rs` |
| §18 Fjall | 94 | 9/9 keyspaces, 7/7 magics, 20/20 record kinds ✓; 7 extra record IDs not in master |
| §19 ABI + §20 Shard | 74 | All 7 ShardCommand + 4 ShardDirective ✓; `BoundedActionCompletionQueue` wrong backend; 6 ABI drifts; `tick_shard` Cancel/Barrier stubbed |
| §21 IPC + §30 + §33 | 84 | 24-byte IPC frame ✓; 11/11 commands ✓; 30/30 CLI ✓; BUT IPC ingress uses `crossbeam_channel` (LETHAL) |
| §46-47 Helpers + Taint | 94 | Taint lattice ✓, AND/OR no-short-circuit ✓, 10/10 helpers ✓, F64 finiteness ✓; ExprOp 29 vs 30 |
| **§50 ArrayQueue** | **35 (LETHAL)** | Shard queue ✓; **action queue uses `Mutex<VecDeque>`**; **IPC ingress uses `crossbeam_channel`**; no MAJOR-1 bead |
| §16-17 Error codes | 65 | All 36 §16 codes ✓; 11/30 §17 codes dead-letter (incl. `SECRET_UNAVAILABLE` misrouted to `ARTIFACT_MALFORMED`); 2 self-laundering tests |
| **§65 SideEffect/RetrySafety** | **18 (LETHAL)** | Production 5+3 vs master 7+4; test files dead; gates enforce broken; no MAJOR-6 bead |
| §8-10 Language | 71 | All 11 primitives, 3 aliases, ID pattern, 4 triggers, 13 helpers ✓; BUT 5/8 reference roots rejected; `mod restrictions;` not declared |
| §36-39 Test/Bench | 62 | 16,041 tests, 3.99x density (master 5x); 4/11 §38 properties SHIP-BLOCKER; 2/22 §39 bench groups missing; BenchmarkMetadata 7/22 fields |

## Per-Agent Highlights (condensed)

### R3-A1: §13 Resource Contracts (Score 64)
- All 16 master fields present (workflow/types.rs:169-206)
- 2 undocumented extra fields: `max_transitions_per_tick` (duplicate of `max_step_budget_per_tick`), `allows_secret_results` (dead flag)
- `BoundednessPolicy::DEFAULT` only exercised at `explain_plan_limits.rs:40` and tests, NOT in production admission
- `MAX_STEPS_PER_WORKFLOW = 65_535`, `MAX_CONSTANTS = 65_535` exceed master values 1000/8192
- `vb-o5zb.3` closed 2026-06-05 against unmet acceptance criteria
- **Top concern:** Admission path gap + 1000x policy ceiling = 50,000-step workflow admitted without complaint

### R3-A2: §14 Core Types (Score 99)
- All 14 ID types, 4 value types, 18 CoreError variants ✓
- Taint 3-level lattice with `#[repr(u8)]` explicit discriminants (Clean=0, DerivedFromSecret=1, Secret=2)
- FiniteF64 rejects NaN/+inf/-inf in both debug AND release ✓
- `ids.rs` → `ids/mod.rs` path drift (cosmetic; modular layout is better design)

### R3-A3: §15 Final IR (Score 68)
- 34 IR variants exist in code, only 22 emitted
- Master says 30 ExprOp, code has 29 (section 46 mentions unary `-`)
- `LoadAccessor` opcode missing from all 3 evaluators (eval.rs, evaluate.rs, core.rs) — falls through to `UnknownOperator`
- Master cites `nodes.rs`/`expressions.rs` as canonical; actually canonical is `workflow/types.rs`
- `eval/evaluate.rs` duplicates helpers; `compiled_workflow.rs.removed` is stale twin (10_000 steps vs 1_000)

### R3-A4: §18 Fjall Persistence (Score 94)
- 9/9 keyspaces, 7/7 magics, 20/20 record kinds, 11/11 typed errors, 60-byte envelope ✓
- 7 extra record kind IDs (RunAdmission=24, RunResumed=25, RunRetried=26, RunAnswered=27, RunKilled=28, StepSucceeded=29)
- All envelope multi-byte fields are little-endian ✓
- Fjall keys are big-endian ✓
- `UnexpectedTrailingBytes` error is a strict-superset not in spec (acceptable)

### R3-A5: §19 Action ABI + §20 Shard (Score 74)
- All 7 ShardCommand + 4 ShardDirective variants ✓ (with 9 extra ShardCommands, 2 extra ShardDirectives)
- `ActionContract` complete; Idempotency 3 variants ✓
- `tick_shard` Cancel/Barrier stubbed (return `UnsupportedOperation`)
- 6 ABI field drifts: `ActionFailure.retryable: bool` vs `retry_policy: RetryPolicy`; 3 `ActionError` variants lost `ticket: ActionTicket`; `PayloadTooLarge` field names reversed
- **Top concern:** action completion queue not shard-owned; `BoundedActionCompletionQueue` orphan (tests only)

### R3-A6: §21 IPC + §30 + §33 (Score 84)
- 24-byte IPC frame: magic, version, command, flags, reserved, correlation, payload_len ✓
- 11/11 commands at IDs 1..=11; UnknownCommand catch-all ✓
- Pipelining supported (drain multiple frames per `handle_readable`) ✓
- `magic-before-allocation` validation ✓
- BUT IPC ingress uses `crossbeam_channel::bounded` (LETHAL Section 50)
- 30/30 CLI commands ✓ (Command enum, parse_args, dispatcher)
- `banned-token-gates` task is phantom (no command/script)
- `nightly-feature-cargo-probe` task body is `true` (no-op)

### R3-A7: §46-47 Helpers + Taint (Score 94)
- Taint 3-level lattice with explicit discriminants, monotonic join_taint ✓
- 12 Kani harnesses (H1-H12) over-evidence the taint lattice
- AND/OR no-short-circuit ✓ (3 enforcement layers: bytecode, pop_pair, expect_bool)
- 10/10 helpers ✓ (empty, unique, contains, starts_with, ends_with, has, append, append_if, merge, sum)
- F64 finiteness ✓ (rejects NaN/+inf/-inf in both debug+release)
- ExprOp 29 vs 30 (master says 30)

### R3-A8: §50 ArrayQueue (Score 35 — LETHAL)
- Shard command queue uses `crossbeam_queue::ArrayQueue` ✓
- Trace ring uses `rtrb::RingBuffer` (SPSC) ✓
- **Action completion queue uses `Mutex<VecDeque>`** (wrong backend; not in spec allowed list)
- **IPC ingress uses `crossbeam_channel::bounded`** (LETHAL)
- No MAJOR-1 bead exists
- Tests test public API behavior, not backend identity
- Forbidden-API scanner only catches `crossbeam_channel::unbounded(`, not the bounded variant

### R3-A9: §16-17 Error Codes (Score 65)
- All 36 Section 16 codes ✓
- 19/30 Section 17 codes ✓
- 11 Section 17 codes are DEAD LETTERS (defined but never constructed in production):
  INPUT_MAPPING_FAILED, STEP_SKIPPED_REFERENCE, WAIT_TIMEOUT, ASK_TIMEOUT, FOR_EACH_ITEM_FAILED, TOGETHER_BRANCH_FAILED, COLLECT_PAGE_FAILED, REDUCE_ITEM_FAILED, RESULT_REFERENCE_MISSING, REPLAY_DIVERGED
- SECRET_UNAVAILABLE is MISROUTED to ARTIFACT_MALFORMED (0x4017) — security classification failure
- 2 self-laundering tests assert the missing codes must NOT appear

### R3-A10: §65 SideEffect/RetrySafety (Score 18 — LETHAL)
- Production `SideEffect`: None, Writes, Sends, Creates, Destroys (5)
- Master: Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite, Process, UnsafeShell (7)
- Production `RetrySafety`: Safe, KeyRequired, Unsafe (3)
- Master: Idempotent, RequiresIdempotencyKey, NotRetrySafe, Unknown (4)
- Test files in `enums/tests/` are dead code (not declared in lib.rs)
- 3 production gate functions enforce broken taxonomy
- Test files have malformed `use vb_core {\nuse vb_core::action::ActionName;` import
- No MAJOR-6 bead

### R3-A11: §8-10 Language Spec (Score 71)
- All 11 primitives + 3 aliases ✓
- ID pattern enforced ✓
- 4 triggers ✓ (manual, schedule, event, webhook)
- Reference roots: 5/8 silently rejected as `UnknownReference` (step_id, loop_name, error, attempt, total)
- `mod restrictions;` NOT declared in `vb_compile/src/lib.rs:14-26`
- 19 dead tests in `restrictions/tests/attempt_number_tests.rs`
- `StepKindAst::Repeat` has no `body` field — parser drops body steps at `parse_repeat:381-385`

### R3-A12: §36-39 Test/Benchmark (Score 62)
- 16,041 `#[test]` attributes, 738 test files
- Test/prod LoC ratio: 3.99x (master requires 5x — 20% short)
- `tarpaulin-report.json` is 3 bytes (`{}` + newline)
- 4/11 Section 38 properties SHIP-BLOCKER missing (concurrency_safety, bytecode_ast_parity, taint_propagation, error_recovery)
- 2/22 Section 39 bench groups missing (warm_throughput, digest_computation)
- BenchmarkMetadata has 7/22 fields (only git commit on struct)
- 1 alias test + 4 missing tests in §38 are the gap
