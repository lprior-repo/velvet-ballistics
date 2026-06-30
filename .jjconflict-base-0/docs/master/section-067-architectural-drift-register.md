---
section: 67
title: "Architectural Drift Register"
parent: velvet-ballistics-MASTER.md
---

## 67. Architectural Drift Register


This section tracks known architectural defects discovered through adversarial review. Each entry states the defect, the root cause, the resolution contract, and the phase that resolves it. Entries are removed when the resolution phase is complete and evidenced.

### DRIFT-1: Runtime Taint Tracking — RESOLVED

**Resolution evidence:**
- `EvalExpr` reads taint from all `LoadSlot` operands and joins into output taint (`crates/vb_expr/src/eval.rs`).
- `BuildObject` joins taint from all field slots into output taint.
- `BuildList` joins taint from all item slots into output taint.
- `Finish` node reads slot taint and emits `EngineSignal::Finished(SlotValue, Taint)` (`crates/vb_core/src/nodes.rs`, `node_helpers.rs`).
- `EngineSignal::Finished` carries taint alongside value in the result signal.
- Compile-time taint validation remains as defense-in-depth.

**DRIFT-1 is closed.**

### DRIFT-2: Crash Recovery Cannot Reconstruct Live State — PARTIALLY RESOLVED

**Original defect:** Earlier builds recorded no slot values, no slot taint, and no step lifecycle events (`StepStarted`/`StepSucceeded`) for deterministic steps. After a crash, `UnsupportedRecoveryState` could report `slot_values: true`, `slot_taint: true`, while hydration still proceeded with empty frames. The system was not crash-recoverable for workflows that performed deterministic computation between suspension points.

**Current evidence:** `SlotWrittenEvent` can carry encoded `SlotValue` and taint evidence; `RecoveryFrameSeed` reconstruction applies recovered slots and step states; `DurableFrameRecoveryBoundary::hydrate_run_frame` rejects unsupported live-frame state instead of silently producing a broken frame. Closed beads `vb-x0mt`, `vb-9fy4`, and `vb-vs7k` record Phase 44/recovery hydration work.

**Remaining gap:** Pending action hydration remains gated as unsupported when unresolved actions are present, summary-only hydration still returns `UnsupportedFullRecoveryHydration`, and `UnsupportedAsyncStrictAck` still marks strict async acknowledgement limitations. Release acceptance still requires end-to-end crash recovery evidence for all live recovery paths.

**Root cause:** Journal events are only emitted at suspension points (action dispatch, wait, ask). Deterministic steps between suspensions are treated as atomic but the journal cannot reconstruct them.

**Resolution contract:**
1. Every deterministic step must emit `SlotWritten` events (value + taint) to the journal before advancing PC.
2. `StepStarted`/`StepSucceeded` events must be emitted for every step, not just suspension points.
3. Recovery must reconstruct slot values and taint from journal events.
4. `UnsupportedRecoveryState` must gate hydration: if `slot_values == true`, hydration must fail with a typed error, not produce a broken frame.
5. Journal error handling in shard.rs must not use `Ok(()) | Err(_) => {}`. Journal write failures must propagate as runtime errors or at minimum log a diagnostic.

**Performance note:** Emitting `SlotWritten` per deterministic step increases journal write volume. Under `Journaled` durability, these batch via the writer queue. Under `Strict`, each step gets an fsync — this is the correct safety tradeoff. `Volatile` mode remains zero-journal for testing.

**Coding style:** No async. No channels. Synchronous journal append within the shard's single-threaded drive loop. Bounded writer queue absorbs burst. If queue is full, the step blocks (backpressure), not silently drops.

**Resolves in:** Phase 44 (Recovery Evidence Chain); remaining live pending-action recovery evidence is still a release gate.

### DRIFT-3: No Aggregate Resource Budget Across Primitive Composition — RESOLVED

**Original defect:** Individual primitive bounds existed (`ForEach limit`, `Together branches`, `Repeat max_attempts`) but their composition was unbounded. `ForEach(limit=1000)` wrapping `Together(branches=256)` could create 256,000 sequential step executions and 256,000 ValueStore arena entries in a single run. The `ValueStore` had no cap on total arena entries (symbols, lists, objects, blobs are all append-only with no GC).

**Root cause:** Bounds are per-primitive, not per-run. No dataflow analysis propagates bounds through nested compositions. `ResourceContract` defaults (`max_fanout: u16::MAX`, `max_collect_items: u32::MAX`, `max_step_budget_per_tick: u64::MAX`) are effectively unbounded.

**Resolution contract:**
1. Phase 37 (Whole-Workflow Boundedness) computes `WholeWorkflowBudget` from IR — this resolves the static analysis gap.
2. `ValueStore` must have a per-run arena cap (e.g., `max_arena_entries: u32`). Insert methods must check the cap and return a typed error on overflow.
3. `ResourceContract` defaults must be tightened from `u16::MAX`/`u32::MAX`/`u64::MAX` to policy-specified defaults.
4. `StepBudget` per tick must have a hard ceiling (e.g., 100,000) regardless of configuration.
5. Collect global `Mutex<Vec>` must be replaced with per-run pagination state to eliminate cross-run interference.

**Resolution evidence:** Phase 37 whole-workflow budget computation is represented in `vb_core::budget` and called from `vb_compile`; Phase 45 resource enforcement added `ValueStore` arena caps, `BudgetExceeded`, tightened defaults, and hard `StepBudget` ceilings. Closed beads `vb-u7vj`, `vb-i9sn`, and `vb-qwdn` record this work.

**DRIFT-3 is closed, subject to normal full-gate evidence refresh.**

### DRIFT-4: IR Validation Is Bounds-Only, Not Structural — RESOLVED

**Original defect:** `try_from_parts` validated that numeric indices were within array bounds but did not validate structural correctness: reachable nodes, forward-only edges, well-formed loop structures, valid `SymbolId` references, or accessor path segment validity. A postcard-deserialized artifact from untrusted input could bypass compiler-level structural validation.

**Root cause:** The compiler's structural validations (control flow, reference, type/taint) operate on the AST, not on the compiled IR. They are never re-checked at the IR level.

**Resolution contract:**
1. `try_from_parts` must validate that every node is reachable from `entry`.
2. `try_from_parts` must reject backward edges (Jump targets, Choose targets, loop body/done targets must be forward).
3. `try_from_parts` must validate that loop primitives are paired correctly (ForEachStart has a matching ForEachNext and ForEachJoin).
4. `try_from_parts` must validate that BuildObject SymbolIds are within the symbol table range.
5. `try_from_parts` must validate AccessorProgram path segments (Field SymbolId range, Index bounds).
6. The artifact loading path (`run-compiled` CLI command) must treat the artifact as untrusted input.

**Coding style:** Straightforward `for` loops over nodes. Checked indexing. No recursion (bounded by node count). Each check returns a typed `IRValidationError` identifying the specific node and check that failed.

**Resolution evidence:** `CompiledWorkflow::try_from_parts` now calls structural validators for accessor path symbols, reachability, and forward edges; workflow tests exercise unreachable-node, invalid edge, and accessor validation. Closed beads `vb-honk` and `vb-w1ww` record Phase 46 completion.

**DRIFT-4 is closed, subject to normal full-gate evidence refresh.**

### DRIFT-5: Validation Logic Duplicated Between vb_validate and vb_compile — PARTIALLY RESOLVED

**Defect:** Both `vb_validate` and `vb_compile` contain parallel modules (schema, references, control_flow, type_taint) that must be kept in sync manually. The two crates operate on different input types (document model vs AST) but enforce the same rules.

**Root cause:** Historical. `vb_validate` was built first on the document model. `vb_compile` was built later with its own validation on the AST. Both must accept the same workflow language.

**Resolution contract:**
1. Single validation pipeline on a shared intermediate representation.
2. Both crate public APIs preserved for backward compatibility.
3. Internal delegation to one implementation.
4. Remove the sync requirement.

**Coding style:** No traits, no generics, no higher-order functions. A plain `pub fn validate(parts: &WorkflowParts) -> Result<ValidationOutput, ValidationError>` that each crate calls.

**Current evidence:** `vb_compile` delegates compiled `WorkflowParts` validation through `vb_validate::shared::validate` / `validate_with_contracts`, re-exports validation errors, and shares reference validation via `vb_validate::references::RefTables` and `validate_single_reference`. Closed bead `vb-2pp9` records the Phase 42 reference/shared-parts deduplication work.

**Remaining gap:** Source-level schema/control-flow/type-taint modules still exist in both crates because they operate on different input representations. DRIFT-5 is not fully closed until the remaining duplicated source validation paths are either removed, proven equivalent by contract-as-data tests, or explicitly documented as representation-specific wrappers over one shared implementation.

**Resolves in:** Phase 42 (Validation Deduplication) plus remaining validation parity evidence.

---
