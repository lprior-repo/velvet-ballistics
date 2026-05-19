# Test Plan — vb-0sps: Generated-vs-IR Parity BDD Acceptance

## Bead
- **ID**: `vb-0sps` / `VB-BDD-CATALOG-007`
- **State**: 7 (test-planner — post-State-6 proof approval)
- **Isolated workdir**: `/home/lewis/src/bd-vb-0sps-bdd`
- **Target**: `crates/workspace_tests/tests/vb_0sps_generated_ir_parity_bdd.rs`
- **Proof approval**: State 6 proof-reviewer APPROVED (attempt 7); TLC 1.6M/1.9M states, depth 10, exit 0 on all 4 positive configs; divergence_sanity expected fail at depth 2

---

## Summary

| Dimension | Count |
|---|---|
| Behaviors identified | 14 |
| Trophy: unit | 11 |
| Trophy: integration | 14 |
| Trophy: e2e | 2 |
| Trophy: static analysis | ∞ (clippy/cargo-deny) |
| BDD scenarios | 18 |
| Proptest invariants | 6 |
| Fuzz targets | 3 |
| Kani harnesses | 2 |
| Mutation checkpoints | 9 |

**Deviation from 60/30/5/5**: Integration is primary because the bead IS a cross-component parity comparison (IR oracle vs generated candidate). Unit layer is narrow (pure parity comparators + fixture constructors). Static analysis is continuous.

---

## 1. Behavior Inventory

### B-001: Deterministic terminal parity
"IR oracle and generated candidate produce identical terminal SlotValue, Taint, status, final PC, executed-step count, all slot values, all slot taints, and all step states when the workflow is deterministic and supported by generated subset."

### B-002: Deterministic journal/event parity
"Normalized event sequence preserves order, event kind, run id, step, slot, value handle, taint, action ticket/id, retry count, wait/ask metadata, terminal event, and typed failure fields."

### B-003: Taint lattice parity at every slot write
"Taint values are compared at every written slot and at the terminal result — not just at display/debug string level."

### B-004: Suspension boundary — kind and metadata match
"IR and generated modes block with identical suspension kind (action/wait_until/wait_event/ask/budget), step, resume PC, action id/input/output slots, ticket fields, retry attempt, wait deadline/event/timeout fields, and ask prompt/answer/timeout fields."

### B-005: Suspension boundary — no advance past boundary
"Neither IR nor generated mode advances PC or writes post-boundary output before consuming a matching resume input."

### B-006: Resume parity — identical input yields identical output
"Identical resume input causes matching output slot write, taint, event, PC, step-state transition, completion/answer/failure event, and later terminal result."

### B-007: Typed error parity — variant and semantic fields
"Typed error class and all semantic fields match exactly between IR and generated modes, or match through a documented normalized adapter whose mapping is part of the BDD assertion."

### B-008: Unsupported generated subset fail-closed
"`validate_generated_subset` or `emit_rust_workflow` returns `CodegenError::UnsupportedIr { feature }` before source emission, compile, run, or silent fallback to IR oracle."

### B-009: No source emission on unsupported path
"Unsupported generated features produce zero bytes of emitted Rust source; `sourceEmitted` flag is FALSE in the paired model and no generated binary exists."

### B-010: Unsupported not counted as generated parity
"BDD assertions do not count any fallback-to-IR path as generated parity evidence; IR oracle remains the sole semantic reference."

### B-011: Catalog closure
"`acceptance_catalog.rs` row `VB-BDD-CATALOG-007` has `executable_evidence_target: Some("crates/workspace_tests/tests/vb_0sps_generated_ir_parity_bdd.rs")` and `deferred_follow_up_bead: None` after the BDD target exists and passes."

### B-012: Positive path — generated subset validated before emission
"For positive parity scenarios, `validate_generated_subset(&workflow)` succeeds before any generated source is emitted or executed."

### B-013: Step-state sequence legal transitions
"Step-state sequences obey the master `StepState` transition contract for both IR and generated machines; terminal states do not reopen."

### B-014: No maxperf/speed claims or release-gate activation
"BDD evidence does not claim maxperf, PGO, generated-vs-IR speed ratio, `compile --emit rust` release readiness, or generated execution as current milestone gate."

---

## 2. Trophy Allocation

| Behavior | BDD/Unit | Integration | E2E | Static |
|---|---|---|---|---|
| B-001 terminal parity | BDD | integration fixture | — | clippy |
| B-002 journal parity | BDD | integration fixture | — | clippy |
| B-003 taint parity | unit + BDD | — | — | clippy |
| B-004 suspension kind/metadata | BDD | integration fixture | — | clippy |
| B-005 no advance past boundary | BDD | — | — | clippy |
| B-006 resume parity | BDD | integration fixture | — | clippy |
| B-007 typed error parity | BDD | integration fixture | — | clippy |
| B-008 unsupported fail-closed | BDD + unit | — | — | clippy |
| B-009 no source emission | unit | — | — | clippy |
| B-010 IR oracle is sole reference | BDD | — | — | clippy |
| B-011 catalog closure | — | — | e2e catalog gate | — |
| B-012 validate before emit | unit | — | — | clippy |
| B-013 step-state transitions | BDD | — | — | clippy |
| B-014 no speed/release claims | review | — | — | — |

**Rationale**: This bead is inherently integration-heavy — every scenario compares two live runtimes. Unit layer covers pure fixture constructors, parity comparator functions, and error-type classification. Static analysis is continuous (clippy on generated+IR code). E2E is minimal: only catalog gate and one smoke test.

---

## 3. BDD Scenarios

### Family 1: Deterministic Terminal Parity (B-001, B-002, B-003, B-013)

#### Scenario 1.1 — Deterministic workflow finishes: terminal state matches
```
Given: A CompiledWorkflow accepted by validate_generated_subset with a deterministic Do+Finish path
  And: identical initial slot values, taints, value-store contents, PC=0, step states, run id, budget, resume payloads
When: IR interpreter executes run_until_blocked to terminal
  And: generated runtime executes run_until_blocked to terminal
Then: terminal SlotValue, Taint, status, final PC, executed-step count match exactly
  And: all slot values match
  And: all slot taints match
  And: all step states match
  And: terminal event kind, run id, step, slot, value, taint match
```

**Test function**: `fn deterministic_workflow_terminal_parity_when_ir_and_generated_finish()`

#### Scenario 1.2 — Taint passes through every slot write
```
Given: A workflow with Do step writing slots with clean, tainted_a, and tainted_b taints
  And: identical inputs to both modes
When: both modes execute to terminal
Then: at every SlotWritten event, IR taint == Gen taint
  And: at terminal result, IR result taint == Gen result taint
```

**Test function**: `fn taint_parity_at_every_slot_write_and_terminal()`

#### Scenario 1.3 — Step-state sequence is legal and terminal states don't reopen
```
Given: A deterministic workflow
When: both modes execute
Then: every step state transition in ir_steps and gen_steps is legal per ValidTrans
  And: once a step state is "terminal", no further state changes occur
```

**Test function**: `fn step_state_sequence_legal_and_terminal_states_do_not_reopen()`

---

### Family 2: Suspension Parity (B-004, B-005)

#### Scenario 2.1 — Do action blocks: suspension kind and metadata match
```
Given: A workflow with a Do step that blocks (e.g., pending action completion)
  And: identical initial observations
When: both modes execute run_until_blocked
Then: both are blocked with kind = "action"
  And: blocked step index matches
  And: resume PC matches
  And: action id, input slot, output slot match
  And: ticket fields match
  And: neither mode advances PC past the blocked step
```

**Test function**: `fn do_action_blocks_suspension_metadata_matches_and_pc_does_not_advance()`

#### Scenario 2.2 — WaitUntil timer blocks: suspension metadata matches
```
Given: A workflow with WaitUntil step
When: both modes execute run_until_blocked
Then: both blocked with kind = "wait_until"
  And: deadline, event, step, resume PC, ticket fields match
  And: no mode advances past boundary
```

**Test function**: `fn wait_until_blocks_metadata_and_pc_matches()`

#### Scenario 2.3 — Ask blocks: prompt and ticket metadata matches
```
Given: A workflow with Ask step
When: both modes execute run_until_blocked
Then: both blocked with kind = "ask"
  And: prompt, answer slot, step, resume PC, ticket match
  And: no mode advances past boundary
```

**Test function**: `fn ask_blocks_metadata_and_pc_matches()`

---

### Family 3: Resume Parity (B-006)

#### Scenario 3.1 — Resume action completion: output and events match
```
Given: A workflow blocked on Do action with identical suspension state in both modes
When: identical action completion (value, taint, ticket) is supplied to both modes
Then: output slot write value and taint match
  And: completion event (kind, step, slot, value, taint, action_id, ticket, retry) matches
  And: PC advances to same next step
  And: step state transitions match
  And: subsequent terminal result matches
```

**Test function**: `fn resume_action_completion_parity_output_taint_event_pc_and_final_result()`

#### Scenario 3.2 — Resume ask answer: output and events match
```
Given: A workflow blocked on Ask with identical suspension state
When: identical ask answer (value, taint) is supplied to both modes
Then: output slot write, taint, answer event, PC, step state, and terminal result match
```

**Test function**: `fn resume_ask_answer_parity_output_taint_event_pc_and_final_result()`

#### Scenario 3.3 — Resume WaitUntil timer: deadline and events match
```
Given: A workflow blocked on WaitUntil with identical suspension state
When: identical timer event is supplied to both modes
Then: wait_fired event, PC, step state, and terminal result match
```

**Test function**: `fn resume_timer_parity_event_pc_and_final_result()`

---

### Family 4: Typed Error Parity (B-007)

#### Scenario 4.1 — Missing slot error: variant and fields match
```
Given: A workflow fixture referencing a slot index outside the workflow's slot count
When: both modes execute
Then: both return/parity-error with error class = "missing_slot"
  And: slot index field matches
  And: step index matches
```

**Test function**: `fn missing_slot_error_parity_variant_and_fields()`

#### Scenario 4.2 — Divide by zero: variant and fields match
```
Given: A workflow with an expression performing division where divisor is always zero
When: both modes execute
Then: both return/parity-error with error class = "div_by_zero"
  And: step index matches
```

**Test function**: `fn divide_by_zero_error_parity_variant_and_fields()`

#### Scenario 4.3 — Type mismatch: variant and fields match
```
Given: A workflow with a type-mismatch expression (e.g., add string to u64)
When: both modes execute
Then: both return/parity-error with error class = "type_mismatch"
  And: step index matches
```

**Test function**: `fn type_mismatch_error_parity_variant_and_fields()`

#### Scenario 4.4 — Budget exhaustion: variant and fields match
```
Given: A workflow with a step count that exceeds StepBudget
When: both modes execute
Then: both return/parity-error with error class = "budget_exhausted"
  And: step index matches
```

**Test function**: `fn budget_exhausted_error_parity_variant_and_fields()`

---

### Family 5: Unsupported Generated Fail-Closed (B-008, B-009, B-010)

#### Scenario 5.1 — Unsupported accessor: validate_generated_subset returns UnsupportedIr before emission
```
Given: A CompiledWorkflow containing an unsupported accessor (e.g., runtime symbol store access)
When: validate_generated_subset is called
Then: returns Err(CodegenError::UnsupportedIr { feature: "accessor:<kind>" })
  And: no Rust source is emitted
  And: emit_rust_workflow also returns UnsupportedIr
```

**Test function**: `fn unsupported_accessor_returns_unsupported_ir_before_source_emission()`

#### Scenario 5.2 — Unsupported expression: validate_generated_subset returns UnsupportedIr before emission
```
Given: A CompiledWorkflow containing an unsupported expression (e.g., runtime string helper)
When: validate_generated_subset is called
Then: returns Err(CodegenError::UnsupportedIr { feature: "expr:<kind>" })
  And: no Rust source is emitted
```

**Test function**: `fn unsupported_expression_returns_unsupported_ir_before_source_emission()`

#### Scenario 5.3 — Unsupported node kind: validate_generated_subset returns UnsupportedIr
```
Given: A CompiledWorkflow containing an unsupported node (e.g., Choose with weight)
When: validate_generated_subset is called
Then: returns Err(CodegenError::UnsupportedIr { feature: "node:<kind>" })
  And: no Rust source is emitted
```

**Test function**: `fn unsupported_node_returns_unsupported_ir_before_source_emission()`

#### Scenario 5.4 — Fallback to IR is not counted as generated parity
```
Given: A workflow with an unsupported feature
When: BDD assertions run
Then: the scenario is classified as unsupported (not parity-pass)
  And: no assertion compares IR output to generated output for this fixture
```

**Test function**: `fn unsupported_workflow_not_counted_as_generated_parity()`

---

### Family 6: Catalog and Contract Integrity (B-011, B-012, B-014)

#### Scenario 6.1 — Catalog VB-BDD-CATALOG-007 points to executable target
```
Given: acceptance_catalog.rs with VB-BDD-CATALOG-007 row
When: the catalog is validated
Then: executable_evidence_target == Some("crates/workspace_tests/tests/vb_0sps_generated_ir_parity_bdd.rs")
  And: deferred_follow_up_bead == None
  And: the target file exists and its tests pass
```

**Test function**: `fn catalog_007_points_to_executable_target_and_deferred_is_none()`

#### Scenario 6.2 — Positive parity scenarios validate generated subset before execution
```
Given: All positive-parity fixtures in the BDD module
When: each fixture is executed
Then: validate_generated_subset succeeds before any generated emission or execution
```

**Test function**: `fn all_positive_parity_fixtures_pass_validate_generated_subset_before_execution()`

#### Scenario 6.3 — No maxperf/speed/PGO release claims in test documentation
```
Given: The BDD test module and its documentation
When: static review runs
Then: no claim of maxperf, PGO, speed ratios, emit_rust release readiness, or current milestone gate
```

**Test function**: `fn no_maxperf_speed_pgo_release_claims_in_bdd_documentation()`

---

## 4. Proptest Invariants

### P-001: Deterministic workflow — terminal result stable
```
Function: run_ir_observed + run_generated_observed
Invariant: For any valid deterministic CompiledWorkflow accepted by validate_generated_subset,
          running both modes produces bit-identical terminal SlotValue, Taint, status, final PC.
Strategy: any valid CompiledWorkflow (generated via WorkflowParts arbitrary)
Anti-invariant: Workflow with nondeterministic elements (rand, time) — use explicit fixture instead
```

### P-002: Taint propagation — every write has matching taint
```
Function: compare_observed_runs (field-level)
Invariant: For every SlotWritten event at index i, ir_journal[i].taint == gen_journal[i].taint
Strategy: any valid workflow with at least one slot write
Anti-invariant: Workflow with taint-changing operations that differ between modes
```

### P-003: Journal prefix — ordered events match field-for-field
```
Function: compare_observed_runs (journal)
Invariant: For all i in 1..min(len(ir_journal), len(gen_journal)),
          ir_journal[i].kind == gen_journal[i].kind
          AND ir_journal[i].step == gen_journal[i].step
          AND ir_journal[i].slot == gen_journal[i].slot
          AND ir_journal[i].value == gen_journal[i].value
          AND ir_journal[i].taint == gen_journal[i].taint
Strategy: any valid deterministic workflow
```

### P-004: Suspension metadata — identical blocked state
```
Function: compare_suspension (blocked metadata)
Invariant: ir_blocked.kind == gen_blocked.kind
          AND ir_blocked.step == gen_blocked.step
          AND ir_blocked.resume_pc == gen_blocked.resume_pc
Strategy: any workflow with Do/Wait/Ask that blocks before resume
Anti-invariant: Workflow with mismatched block conditions between modes
```

### P-005: Resume — output slot write matches input
```
Function: resume + compare_observed_runs
Invariant: After identical resume input, the resulting slot write value and taint
          are identical in both modes AND equal to the resume input where applicable
Strategy: any blocked workflow with valid resume payload
```

### P-006: Error classification — variant is stable
```
Function: typed_error classification
Invariant: For any error E, classify_error(ir_error) == classify_error(gen_error)
Strategy: any invalid bounded workflow fixture
Anti-invariant: Workflow that produces different error classes in IR vs generated mode
```

---

## 5. Fuzz Targets

### F-001: Journal event deserialization
```
Target: GeneratedRun::journal event parsing
Input type: bytes (raw journal event bytes from generated runtime)
Risk: Panic on malformed event bytes, OOM on oversized fields, logic error on mis-ordered events
Corpus seeds: valid SlotWritten, action_complete, ask_answer, wait_fired, step_end, typed_failure events
```

### F-002: CompiledWorkflow fixture construction
```
Target: CompiledWorkflow::try_from_parts
Input type: arbitrary struct (WorkflowParts with random slot count, node count, taint values)
Risk: Panic on out-of-bounds slot index, illegal state on mismatched node/slot counts
Corpus seeds: minimal valid workflow, max-slot workflow, single-Do workflow, wait+ask workflow
```

### F-003: validate_generated_subset with arbitrary node/expression/accessor
```
Target: validate_generated_subset
Input type: arbitrary CompiledWorkflow with random node kinds, expressions, accessors
Risk: Panic on unhandled node variant, OOM on deeply nested expressions, wrong error variant
Corpus seeds: valid deterministic workflow, workflow with unsupported accessor, unsupported expression
```

---

## 6. Kani Harnesses

### K-001: compare_observed_runs — no panic on any ObservedRun pair
```
Property: compare_observed_runs(ir, gen) never panics for any valid ObservedRun inputs
Bound: All fields within contract bounds (MaxStep=2, MaxSlot=2, MaxEvent=4 per TLA config)
Rationale: This is the core comparator — panics here would corrupt parity evidence. Formal
           verification needed because proptest cannot exhaust all field combinations.
```

### K-002: SlotValue + Taint comparison — no overflow/underflow
```
Property: Comparing SlotValue and Taint at every slot index is always well-defined
Bound: MaxSlot=2, all taint lattice values
Rationale: SlotValue arithmetic and taint lattice operations must not overflow in the
           comparison loop. Kani provides bounded proof of absence of arithmetic failure.
```

---

## 7. Mutation Checkpoints

Critical mutations that MUST be caught (≥90% kill rate target):

| Mutation | Behavior at Risk | Catching Test |
|---|---|---|
| `?` short-circuit in AND/OR helper | B-007 typed error parity | `typed_error_parity_variant_and_fields` |
| Skip taint comparison on terminal result | B-003 taint parity | `taint_parity_at_every_slot_write_and_terminal` |
| IR-only journal event emission | B-002 journal parity | `deterministic_workflow_terminal_parity_when_ir_and_generated_finish` |
| Missing `sourceEmitted = FALSE` guard | B-009 no source emission | `unsupported_accessor_returns_unsupported_ir_before_source_emission` |
| Generated PC advances past blocked step | B-005 no advance past boundary | `do_action_blocks_suspension_metadata_and_pc_does_not_advance` |
| Wrong error variant mapped | B-007 typed error parity | `missing_slot_error_parity_variant_and_fields` |
| Resume input consumed before slot write | B-006 resume parity | `resume_action_completion_parity_output_taint_event_pc_and_final_result` |
| Wrong slot index in suspension metadata | B-004 suspension metadata | `do_action_blocks_suspension_metadata_matches_and_pc_does_not_advance` |
| Catalog deferred_follow_up_bead not cleared | B-011 catalog closure | `catalog_007_points_to_executable_target_and_deferred_is_none` |

**Threshold**: ≥90% mutation kill rate on core parity functions.

---

## 8. Combinatorial Coverage Matrix

### Terminal Parity (Unit)

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| deterministic finish | valid workflow, clean taint | Ok(identical terminal) | unit |
| deterministic finish | valid workflow, tainted_a taint | Ok(identical terminal with taint) | unit |
| terminal mismatch | ir=success, gen=success (same) | Ok(()) | unit |
| terminal mismatch | ir=success, gen=different value | Err(ParityError::TerminalMismatch) | unit |
| step count differs | executed count differs | Err(ParityError::TerminalMismatch { field: "executed_count" }) | unit |
| final PC differs | PC differs at terminal | Err(ParityError::TerminalMismatch { field: "final_pc" }) | unit |

### Suspension (Integration — IR + Generated pair)

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| Do blocks | action pending | blocked=action, metadata match | integration |
| Do blocks + resume | identical completion value | output+taint match, PC advance match | integration |
| WaitUntil blocks | timer not fired | blocked=wait_until | integration |
| WaitUntil fires | identical timer event | wait_fired event match, terminal match | integration |
| Ask blocks | no answer yet | blocked=ask, prompt match | integration |
| Ask answered | identical answer value | output+taint match, terminal match | integration |
| Budget exhausted | step count > budget | error = "budget_exhausted" both sides | integration |

### Error Parity (Integration)

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| missing_slot | OOB slot index | class="missing_slot", index field matches | integration |
| div_by_zero | divisor = 0 | class="div_by_zero" | integration |
| type_mismatch | wrong type combine | class="type_mismatch" | integration |
| bad_pc | PC > node count | class="bad_pc" | integration |
| overflow | value > MaxU64 | class="overflow" | integration |

### Unsupported Fail-Closed (Unit + Integration)

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| unsupported accessor | runtime symbol store accessor | UnsupportedIr, no source emitted | unit |
| unsupported expression | runtime string helper | UnsupportedIr, no source emitted | unit |
| unsupported node | Choose with weight | UnsupportedIr, no source emitted | unit |
| supported path | valid accessor/expression | validate succeeds, source emitted | unit |

---

## 9. Open Questions

1. **Error adapter for POST-002**: The contract allows a "documented normalized adapter" for error field mapping. Is there an existing adapter, or does the BDD scenario document the mapping inline? **Action**: If adapter does not exist, document the expected mapping in the BDD fixture comments and assert both class AND semantic fields explicitly.

2. **Journal event adapter for POST-005**: The TLA model distinguishes 7 event kinds. Does the generated runtime emit the same event kind labels as the IR journal? **Action**: Verify event kind labels match exactly (`action_complete` vs `ActionComplete`, etc.) and assert both IR and generated event kinds explicitly.

3. **compare_observed_runs function signature**: The contract lists `fn compare_observed_runs(ir: &ObservedRun, generated: &ObservedRun) -> Result<(), ParityError>` but `ObservedRun` type is not yet in vb_codegen. Does it exist, or does State 6 implementation introduce it? **Action**: Verify the type exists in the implementation crate before test-writer proceeds.

4. **SlotValue/Taint concrete types**: Are `SlotValue` and `Taint` concrete types in vb_core, or are they type aliases? Exact field-level comparison requires knowing the field names. **Action**: Test-writer must read `vb_core/src/slot.rs` before writing field-level assertions.

5. **Generated runtime compile dependency**: The BDD test may need to compile generated Rust source at test time. Is rustc available in the test environment, or should generated code be pre-compiled as a fixture? **Action**: Check `rustc` availability in `moon ci` test environment; if unavailable, use pre-compiled fixture artifacts.

6. **Catalog update timing**: `acceptance_catalog.rs` must be updated as part of State 6 implementation. Does the test-writer also update the catalog, or is that a separate implementation step? **Action**: Confirm catalog update is in-scope for State 6 implementation.

---

## Exit Criteria (Test-Writer Checklist)

Before this plan is considered complete, the test-writer must confirm:

- [ ] Every behavior B-001 through B-014 has at least one BDD scenario with exact assertions
- [ ] Every error variant in `CodegenError` and `ParityError` has an explicit test scenario
- [ ] Every pure function (`compare_observed_runs`, `validate_generated_subset`, `assert_unsupported_fail_closed`) has a unit test
- [ ] Every pair of IR/generated behavior has an integration test using real runtimes
- [ ] Every proptest invariant P-001 through P-006 is implemented with a strategy
- [ ] Every fuzz target F-001 through F-003 has a corpus seed directory
- [ ] Every Kani harness K-001 and K-002 uses `kani::any()` for structural inputs (no hardcoded shapes)
- [ ] No test asserts only `is_ok()` or `is_err()` without the exact value/variant
- [ ] Mutation kill rate ≥90% stated as threshold in mutation checkpoint section
- [ ] `test-plan.md` is written to the bead directory: `/home/lewis/src/bd-vb-0sps-bdd/.beads/vb-0sps/test-plan.md`
