# Velvet Ballistics — Full Crate Inventory for Fuzzing Research

Generated: 2026-05-24
Scope: Every crate under `crates/`, fuzz targets in `fuzz/`, and verification artifacts.
Focus: **PUBLIC API SURFACES** — the attack surface for fuzzing.

---

## CRATE COUNT SUMMARY

| Crate | Src Files | Test/Kani/Prop | Pub fn | Pub Types | Unsafe |
|-------|----------|----------------|--------|-----------|--------|
| `vb_core` | 95 | 52 | 150 | 101 | 0 |
| `vb_expr` | 42 | 15 | 59 | 14 | 0 |
| `vb_storage` | 92 | 38 | 144 | 63 | 0 |
| `vb_runtime` | 132 | 67 | 264 | 83 | 0 |
| `vb_ipc` | 40 | 4 | 107 | 40 | 0 |
| `vb_validate` | 50 | 15 | 116 | 40 | 0 |
| `vb_yaml` | 24 | 5 | 43 | 25 | 0 |
| `vb_compile` | 69 | 12 | 80 | 27 | 0 |
| `vb_codegen` | 10 | 10 | 40 | 30 | 0 |
| `vb_cli` | 59 | 35 | 48 | 24 | 0 |
| `vb_boundary_inventory` | 17 | 7 | 23 | 28 | 0 |
| `vb_doc` | 7 | 1 | 11 | 19 | 0 |
| `vb_ui_model` | 21 | 1 | 29 | 64 | 0 |
| `vb_ui_snapshot` | 23 | 10 | 72 | 38 | 0 |
| `vb_ui` | 84 | 3 | 801 | 361 | 0 |
| `vb_ui_makepad` | 15 | 3 | 106 | 41 | 0 |
| `vb_benchmark` | 2 | 0 | 6 | 13 | 0 |
| `vb_proof_kernels` | 6 | 0 | 44 | 19 | 0 |
| `vb_verification` | 1 | 0 | 0 | 0 | 0 |
| `workspace_tests` | 26 | 142 | — | — | 0 |

**TOTAL: 3,143+ pub items across 19 production crates. ZERO unsafe blocks.**

---

## FUZZ TARGET SUMMARY (60 targets)

### `fuzz/fuzz_targets/` (12 targets):


1. `check_doc_taint_consistency_accepts_arbitrary_markdown` — doc taint
2. `decode_record` — record decode fuzzing
3. `expr_eval` — expression evaluator
4. `journal_event` — journal events
5. `lex_expr` — expression lexer
6. `ui_redaction_artifact` — UI redaction
7. `vb_5xs4_generated_source_mapping` — source mapping
8. `vb_5xs4_inventory_report` — inventory reports
9. `vb_5xs4_label_sufficiency` — label scanning
10. `vb_5xs4_scan_source_text` — source text scanning
11. `vb_f04l_yaml_compiler_compile` — YAML compiler
12. `vb_storage_codec` — storage codec roundtrip

### `fuzz/src/bin/` (47 targets):

`accepted_artifact_decode`, `accepted_artifact_envelope_qi37_4_2`, `accessor_traversal`, `action_tracker`, `admission_flow`, `admission_fuzz`, `admission_input_surface`, `aggregate_artifact_budget`, `aggregate_workflow_budget`, `binary_payload_fuzz_boundary`, `boundary_evidence_reference`, `boundary_inventory_parser`, `boundary_metadata`, `budget_compute`, `capability_contract_schema`, `capability_name_schema`, `collect_page_pagination`, `compiled_ir`, `digest_coherence`, `expr_bytecode`, `expression`, `expr_eval`, `external_input_adapter_fuzz`, `extract_terminal`, `generated_compare`, `ipc_decode`, `ipc_frame`, `ipc_frame_fuzz_boundary`, `journal_event`, `readback_family_set`, `recover_runtime_frame_seed_contract`, `recovery_decode`, `replay_events`, `resource_budget`, `slot_value_roundtrip`, `step_budget_new`, `storage_envelope_fuzz_boundary`, `strict_artifact_decoder`, `strict_yaml_profile`, `structured_status_render_hostile`, `taint_propagation`, `vb_qi37_12_persisted_payload_decode`, `vb_ui_model_postcard_decode`, `verifier_gates`, `xtask_parse_argv_hostile`, `xtask_parse_options_hostile`, `yaml_events`

---

## UNSAFE AUDIT RESULT

**ZERO actual unsafe blocks in any production code.** All 68 grep hits are:
- `#![forbid(unsafe_code)]` directives (every crate)
- String literals like `"unsafe-adjacent-dependency-boundary"`
- Enum variants like `RetrySafety::Unsafe`
- Comments about unsafe replay semantics

---

## BENCHMARKS

Root `benches/`: action_dispatch, action_queuing, array_queue, cold_start, collect_page, ir_traversal, memory_footprint, pagination_cost, rtrb, snapshot_restore, snapshot_save, timer_wheel_tick, velvet_ballistics

Crate-level: `vb_core/benches/aggregate_resource_budget`, `vb_validate/benches/capability_schema`, `workspace_tests/benches/` (17 benchmarks)

---

## DEPENDENCY GRAPH (internal only)

```
vb_proof_kernels (leaf, no deps)
    ^
vb_core <-- vb_codegen, vb_validate, vb_storage, vb_expr, vb_runtime, vb_ipc, vb_verification, vb_ui_model
    ^        ^          ^            ^           ^         ^           ^        ^              ^
vb_compile --+          |            |           |         |           |        |              |
    ^                   |            |           |         |           |        |              |
vb_yaml ----------------+            |           |         |           |        |              |
                                     |           |         |           |        |              |
vb_runtime --------------------------+           |         |           |        |              |
    ^                                            |         |           |        |              |
vb_storage --------------------------------------+  vb_ipc -+          |        |              |
    ^                                               |     |            |        |              |
vb_validate ----------------------------------------+     |            |        |              |
                                                           |            |        |              |
vb_ui <-- vb_ipc, vb_core, vb_storage, vb_ui_model        |            |        |              |
vb_ui_makepad <-- makepad-widgets                           |            |        |              |
vb_ui_snapshot <-- vb_ui_model                              |            |        |              |
vb_ui_model <-- vb_core                                     |            |        |              |
vb_cli <-- vb_compile,vb_core,vb_codegen,vb_expr,vb_ipc,    |            |        |              |
          vb_runtime,vb_storage,vb_validate,vb_yaml          |            |        |              |
vb_boundary_inventory <-- serde_json                         |            |        |              |
vb_doc (leaf, no internal deps)                              |            |        |              |
vb_benchmark (leaf, no deps)                                 |            |        |              |
vb_verification <-- vb_core, vb_storage                      |            |        |              |
```

---

## FUZZING GAPS (notable missing coverage)

1. **vb_boundary_inventory** — `parse_inventory` and `validate_inventory` accept arbitrary bytes; low fuzz coverage
2. **vb_doc** — `plan_taint_doc_reconciliation` and `scan_for_stale_clean_only_text` take arbitrary text input; no fuzz targets exist
3. **vb_ui_model** — `encode_postcard`/`decode_postcard` roundtrip; only one fuzz target
4. **vb_benchmark** — `capture_metadata` and `check_evidence_gate` take arbitrary Duration values
5. **vb_codegen** — `emit_rust_workflow` accepts arbitrary `CompiledWorkflow`; only `generated_compare`
6. **vb_ui** — 801 pub fn, only 3 test files; no fuzz targets for UI rendering/state
7. **vb_proof_kernels** — `validate_header_before_alloc` takes raw header bytes; no fuzz targets
8. **vb_verification** — crate exists but has 0 pub API items (placeholder)
9. **IPC frame parsing** — `decode_frame_header` validates 12-byte magic; hostile magic byte fuzzing gap

---

## PER-CRATE DETAILS

The per-crate detailed inventory (module files, key public API signatures, existing test files, and Kani proof harnesses) is available in the raw data files:

- `/tmp/fuzz-research/pub-fn-raw.txt` — All 2,143 public function signatures
- `/tmp/fuzz-research/pub-types-raw.txt` — All 1,030 public type declarations
- `/tmp/fuzz-research/unsafe-raw.txt` — All 68 unsafe-related text hits


## 1. vb_core — Hot In-Memory Execution Core

**Cargo.toml**: `crates/vb_core/Cargo.toml`
**Dependencies**: `bytes`, `chrono`, `indexmap`, `serde`, `thiserror`, `vb_proof_kernels`
**Role**: Owns compiled workflow IR, numeric identifiers, runtime slot model, synchronous state-machine loop. No async, no storage, no HTTP, no YAML.

**Module structure** (95 src files):
`action.rs`, `accessors.rs`, `budget.rs`, `capability.rs`, `compiled_workflow.rs`, `diagnostic.rs`,
`engine/choose.rs`, `engine/error_routing.rs`, `engine/expr_eval/accessors.rs`, `engine/expr_eval/core.rs`,
`engine/expr_eval/mod.rs`, `engine/expr_eval/ops.rs`, `engine/expr_eval/ops_text_list.rs`, `engine/expr_eval/stack.rs`,
`engine/node_helpers.rs`, `engine/object_list.rs`, `engine.rs`, `engine/run_loop.rs`, `engine/signals.rs`,
`engine/step.rs`, `engine/validate.rs`, `error.rs`, `errors.rs`, `expressions.rs`, `frame.rs`, `ids/mod.rs`,
`lib.rs`, `limits.rs`, `mod.rs`, `nodes.rs`, `policy.rs`, `replay/choose.rs`, `replay/mod.rs`,
`replay/ops.rs`, `replay/step.rs`, `span.rs`, `validation/graph.rs`, `validation/nodes.rs`,
`validation/resource.rs`, `validation.rs`, `validation/targets.rs`, `value.rs`, `value_store.rs`,
`value_store/blobs.rs`, `value_store/lists.rs`, `value_store/objects.rs`, `value_store/symbols.rs`,
`workflow/mod.rs`

**Key public API** (150 pub fn, 101 pub types):
- Budget: `compute()`, `validate_aggregate_budget()`, `validate_step_ceilings()`, `WholeWorkflowBudget`, `AggregateResourceBudget`
- Action: `validate_idempotency_key_ingredients()`, `verify_idempotency()`, `validate_action_dispatch()`, `issue_action_ticket()`
- Workflow IR: `CompiledWorkflow`, `WorkflowParts`, `try_from_parts()`, `step_name()`, `node()`, `expression()`
- Engine: `eval_expr()`, `eval_expr_with_store()`, `eval_accessor()`, `build_list()`, `route_error_handler()`, `run_loop()`, `RunFrame`, `StepResult`
- Replay: `choose_action()`, `step_replay()`
- Validation: `validate_node_kind()`, `validate_graph()`, `validate_targets()`
- Value Store: `put_blob()`, `get_blob()`, `put_list()`, `get_list()`

**Test/Kani files** (52): 32 Kani harnesses (step budget, budget arithmetic, idempotency, taint, expr bounds, capability, workflow), 5 proptest/property files, 14 engine integration tests, budget/replay/workflow/value_store tests

**Fuzz targets**: `compiled_ir`, `expression`, `expr_eval` (x2), `expr_bytecode`, `budget_compute`, `resource_budget`, `step_budget_new`, `slot_value_roundtrip`, `extract_terminal`, `generated_compare`, `accessor_traversal`, `taint_propagation`, `collect_page`

**Unsafe: 0**

---

## 2. vb_expr — Expression Language

**Cargo.toml**: `crates/vb_expr/Cargo.toml`
**Dependencies**: `arrayvec`, `logos`, `thiserror`, `vb_core`
**Role**: Expression lexer, parser, typechecker, bytecode compiler, stack evaluator.

**Module structure** (42 src files):
`lib.rs`, `mod.rs`, `builtin_eval.rs`, `slot_eval.rs`, `stack_ops.rs`, `helpers.rs`,
`lexer/mod.rs`, `lexer/types.rs`, `parser/mod.rs`, `parser/types.rs`, `typecheck/mod.rs`,
`bytecode/mod.rs`, `bytecode/fold.rs`, `eval.rs`, `eval/environment.rs`, `eval/evaluate.rs`

**Key public API** (59 pub fn, 14 pub types):
- Lexer: `lex()`, `TokenKind`, `Token`
- Parser: `parse()`, `Expr`, `BinaryOp`, `UnaryOp`
- Bytecode: `compile()`, `ExprOp`, `Bytecode`, `fold_constants()`
- Evaluator: `evaluate()`, `evaluate_with_store()`, `slot_evaluate()`, `EvalEnv`, `EvalStack`
- Typechecker: `check_types()`, `ExprType`
- Builtins: `call_builtin()`

**Test/Kani files** (15): Kani stack analysis, proptest strategies, Miri tests (lexer/parser), bytecode adversarial, short-circuit tests, edge case tests, property tests (arithmetic_overflow, constant_folding, eval_bounds)

**Fuzz targets**: `expression`, `expr_eval` (x2), `expr_bytecode`, `lex_expr`

**Unsafe: 0**

---

## 3. vb_storage — Durability & Journal Storage

**Cargo.toml**: `crates/vb_storage/Cargo.toml`
**Dependencies**: `arrayvec`, `blake3`, `chrono`, `crc32c`, `fjall`, `postcard`, `rustix`, `serde`, `thiserror`, `vb_core`
**Role**: On-disk journal, record codec, admission persistence, recovery hydration.

**Module structure** (92 src files):
`admission.rs`, `artifacts.rs`, `batch.rs`, `binary.rs`, `blobs.rs`, `codec/mod.rs`, `codec/header.rs`,
`codec/payload.rs`, `codec/validation.rs`, `constants.rs`, `error/mod.rs`, `error/artifact.rs`,
`error/codes.rs`, `error/warnings.rs`, `events.rs`, `headers.rs`, `indexes.rs`, `keys.rs`,
`process_lock.rs`, `records.rs`, `slot_extra.rs`, `snapshots.rs`, `types.rs`,
`journal/mod.rs`, `journal/admission.rs`, `journal/append.rs`, `journal/batch.rs`,
`journal/core.rs`, `journal/incident.rs`, `journal/injection.rs`, `journal/internal.rs`,
`journal/parse.rs`, `journal/replay.rs`, `journal/source.rs`,
`queue/mod.rs`, `queue/batch.rs`, `queue/writer.rs`,
`recovery/mod.rs`, `recovery/hydrate.rs`, `recovery/hydrate_support.rs`, `recovery/recover.rs`,
`recovery/replay/core.rs`, `recovery/replay/summary.rs`, `recovery/types.rs`,
`trimming/mod.rs`, `trimming/helpers.rs`, `trimming/logic.rs`

**Key public API** (144 pub fn, 63 pub types):
- Codec: `encode_record_header()`, `decode_record_header()`, `encode_record()`, `decode_record()`, `verify_digest_match()`
- Admission: `submit_artifact()`, `submit_artifact_with_contracts()`, `admit_compiled_artifact()`, `compute_policy_digest()`
- Journal: `FjallJournal`, `put_run_header()`, `put_snapshot()`, `put_blob()`, `put_status_index()`, `append_event()`, `commit()`
- Batch: `JournalBatch` — new(), put_*, append_event(), strict(), commit()
- Recovery: `hydrate_run()`, `recover_artifacts()`, `replay_events()`, `hydrate_support()`
- Queue: `StorageQueue`
- Artifacts: `list_artifacts()`, `remove_artifact()`, `artifact_exists()`

**Test/Kani files** (38): 9 Kani harnesses (admission, codec, digest checks, record CRC/kind/magic/payload_len/schema, recovery hydrate, storage invariants), 3 proptest files, journal event tests, codec Miri tests, record/recovery/queue/snapshot/trimming/header/artifact/blob/edge/case/error code/hydrate/index/process lock/recover/security/type/vb_2bok durability tests

**Fuzz targets**: `journal_event` (x2), `vb_f04l_yaml_compiler_compile`, `vb_qi37_12_persisted_payload_decode`, `vb_storage_codec`, `recovery_decode`, `replay_events`, `strict_artifact_decoder`, `storage_envelope_fuzz_boundary`, `admission_input_surface`, `accepted_artifact_decode`, `accepted_artifact_envelope_qi37_4_2`, `admission_flow`, `admission_fuzz`, `binary_payload_fuzz_boundary`, `action_tracker`

**Unsafe: 0**

---

## 4. vb_runtime — Runtime Shard Engine

**Cargo.toml**: `crates/vb_runtime/Cargo.toml`
**Dependencies**: `chrono`, `crossbeam-queue`, `indexmap`, `rtrb`, `thiserror`, `vb_core`, `vb_storage`, `postcard`, `serde`
**Role**: Shard lifecycle, action queuing, admission, recovery, timer wheel, primitives (retry, for_each, together, collect).

**Module structure** (132 src files):
Major modules: `admission.rs`, `action_queue/`, `shard/` (30+ chunk files), `primitives/`, `engine.rs`, `error.rs`, `recovery.rs`, `runtime.rs`, `taint.rs`, `trace.rs`, `fanout.rs`, `for_each.rs`, `collect.rs`

**Key public API** (264 pub fn, 83 pub types):
- Admission: `admit_run()`, `admit_artifact_run()`, `admit_run_with_budget()`, `check_capability()`, `AdmissionContext`, `SharedArtifactStore`, `AcceptedArtifactStore`
- Action Queue: `ActionQueue` — enqueue, dequeue, len, is_empty, is_full, remaining_capacity, capacity
- Actor Registry: `ActorRegistry` — new, register, resolve_compile_time, dispatch
- Shard: `Shard`, `tick_shard()`, `dispatch_action()`, `start_shard()`, `ShardDirective`, `ShardState`
- Primitives: `evaluate_retry()`, `evaluate_for_each()`, `evaluate_together()`, `evaluate_collect()`
- Recovery: `recover_shard_state()`

**Test/Kani files** (67): 8 Kani harnesses (admission_store, capability, engine_yaml_admission, shard_command_queue, trace_ring, vt2f_runtime_facade, vt2f_shard_lower_semantics, idempotency_tracker), proptest, 30 shard test chunks, 7 lifecycle test chunks, action queue/for_each/reentry/engine/fanout/together/collect tests, timer wheel behavior, recovery integration/BDD/hydration, vb_jggy journal event/lifecycle/property tests

**Fuzz targets**: `action_tracker`, `admission_flow`, `admission_fuzz`, `external_input_adapter_fuzz`, `collect_page`, `ipc_frame`, `ipc_decode`, `readback_family_set`

**Unsafe: 0**

---

## 5. vb_ipc — IPC Binary Frame Protocol

**Cargo.toml**: `crates/vb_ipc/Cargo.toml`
**Dependencies**: `arrayvec`, `byteorder`, `bytes`, `crossbeam-channel`, `mio`, `postcard`, `serde`, `thiserror`, `vb_core`, `vb_runtime`, `vb_validate`
**Role**: Binary frame codec (12-byte header + variable payload), Unix socket client/server, command dispatch.

**Module structure** (40 src files):
`lib.rs`, `mod.rs`, `action_output.rs`, `bounded.rs`, `client/connection.rs`, `client/error.rs`,
`client.rs`, `codec.rs`, `commands.rs`, `constants.rs`, `error.rs`, `frame/codec.rs`,
`frame/io.rs`, `frame.rs`, `frame/validate.rs`, `frame_types.rs`, `ids.rs`, `ingress.rs`,
`metrics.rs`, `payloads.rs`, `queue/mod.rs`, `server/dispatch.rs`, `server/error.rs`,
`server/handlers/command.rs`, `server/handlers/event.rs`, `server/handlers/query.rs`,
`server/handlers.rs`, `server/handlers/session.rs`, `server/helpers.rs`, `server/impl_.rs`,
`server/mod.rs`, `server/ticket.rs`, `server/trace.rs`

**Key public API** (107 pub fn, 40 pub types):
- Frame codec: `encode_frame()`, `decode_frame_header()`, `decode_frame_payload()`, `read_frame_header()`, `read_frame_payload()`, `write_frame()`, `validate_frame_magic()`, `validate_frame_bounds()`, `IpcFrameHeader`, `BoundedPayload`
- Client: `connect_ipc()`, `send_command()`, `recv_response()`, `IpcClient` — health, shutdown, list_runs
- Payloads: `encode_payload()`, `decode_payload()`, `IpcPayload`, `IpcCommand`
- Commands: `IpcCommandKind` — from_u16
- Queue: `ArrayQueue` — lock-free bounded MPMC

**Test/Kani files** (4): Kani header validation, client/frame/array queue/server tests

**Fuzz targets**: `ipc_frame` (x2), `ipc_decode`, `ipc_frame_fuzz_boundary`

**Unsafe: 0**

---

## 6-9. vb_yaml, vb_validate, vb_compile, vb_codegen

**vb_yaml** (24 src, 43 pub fn, 25 pub types): YAML event collection (`collect_events()`), AST parsing (`WorkflowAst`, `StepAst`), source maps (`build_source_map()`), limits validation. Fuzz: `yaml_events`, `vb_f04l_yaml_compiler_compile`, `strict_yaml_profile`.

**vb_validate** (50 src, 116 pub fn, 40 pub types): 9 validation gates (07-15) — stack depth, accessor paths, slot refs, node kinds, loop graphs, action contracts, slot cycles, type consistency, determinism proofs. Plus control flow, schema, taint, references, diagnostics. Fuzz: `verifier_gates`. Kani: 3 harnesses.

**vb_compile** (69 src, 80 pub fn, 27 pub types): YAML→IR compiler. `compile_workflow()`, `compile_source()`, `lower_steps_to_ir()`, `lower_set/do/choose/for_each/together/collect/reduce/repeat()`. Fuzz: `vb_f04l_yaml_compiler_compile`. Kani: 9 harnesses.

**vb_codegen** (10 src, 40 pub fn, 30 pub types): Generated Rust workflow code. `emit_rust_workflow()`, `emit_ids()`, `emit_drive_function()`, `emit_step_function()`, `format_generated_rust()`, `compare_generated_to_ir()`. Fuzz: `generated_compare`.

---

## 10-13. vb_cli, vb_boundary_inventory, vb_doc, vb_ui_model

**vb_cli** (59 src, 48 pub fn, 24 pub types): CLI binary (velvet-ballistics). Argument parsing, commands (diff, incident, journal, status, system, verify, workflow), lifecycle, deliver_sink, naming_scan.

**vb_boundary_inventory** (17 src, 23 pub fn, 28 pub types): Workspace boundary scanner. `parse_inventory()`, `validate_inventory()`, `BoundaryInventory`, `BoundaryRecord`, `BoundaryStatus`.

**vb_doc** (7 src, 11 pub fn, 19 pub types): Documentation reconciliation. `plan_taint_doc_reconciliation()`, `scan_for_stale_clean_only_text()`, `EvidenceSupport`.

**vb_ui_model** (21 src, 29 pub fn, 64 pub types): UI data model & postcard codec. `canonicalize_ui_artifact()`, `encode_postcard()`, `decode_postcard()`, `encode_yaml()`, `OutputEnvelope`, `DiagnosticEntry`, `RunHeader`, `EnvelopeKind`. Fuzz: `vb_ui_model_postcard_decode`.

---

## 14-16. vb_ui, vb_ui_makepad, vb_ui_snapshot

**vb_ui** (84 src, 801 pub fn, 361 pub types): Ratatui UI application. Screens: execution_observatory, incident, replay, system, verify, workflow. Replay engine, graph renderer, theme, IPC bridge.

**vb_ui_makepad** (15 src, 106 pub fn, 41 pub types): Makepad widget renderer. Graph canvas, nodes, edges, design tokens.

**vb_ui_snapshot** (23 src, 72 pub fn, 38 pub types): Screenshot testing. `check_spelling()`, `check_color_drift()`, `verify_layout_kernel()`, `redact_sensitive_regions()`, `snapshot_screen()`. Fuzz: vb_5xs4 series, ui_redaction_artifact.

---

## 17-19. vb_benchmark, vb_proof_kernels, vb_verification

**vb_benchmark** (2 src, 6 pub fn, 13 pub types): Benchmark harness. `capture_metadata()`, `baseline_within_budget()`, `result_exceeds_threshold()`, `check_evidence_gate()`.

**vb_proof_kernels** (6 src, 44 pub fn, 19 pub types): Envelope header proofs. `validate_header_crc()`, `validate_header_before_alloc()`, `EnvelopeHeader`.

**vb_verification** (1 src, 0 pub items): Placeholder crate (deps: vb_core, vb_storage).

