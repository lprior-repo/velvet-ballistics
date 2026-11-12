# Codebase Map: `run --step` Implementation

## Overview

The `run --step` command executes a single workflow step in isolation with structured output reporting pc/slot/taint/state deltas.

---

## 1. CLI Entry Point and Command Structure

### Primary Files
- **`/crates/vb_cli/src/main.rs`** - Binary entry point, calls `app_impl::run_from_env()`
- **`/crates/vb_cli/src/app_impl.rs`** - Command dispatcher and implementation (~4956 lines)
  - `run_from_env()` parses args and dispatches to command handlers
  - Line 131: `cmd_run_step()` is called when `StepTarget` is present
  - Lines 1470-1618: `cmd_run_step()` implementation for single-step execution

### Argument Parsing
- **`/crates/vb_cli/src/args.rs`** - Argument parsing and `Command` enum
  - Line 108-115: `Command::Run` variant includes `step: Option<StepTarget>`
  - Line 276-281: `StepTarget` struct definition (`step_id: u16`, `step_input: PathBuf`)
  - Lines 873-887: `parse_optional_step()` parses `--step <id> --step-input <file>`

### Command Re-exports
- **`/crates/vb_cli/src/commands.rs`** - Facade re-exporting run, storage, bench commands

---

## 2. Existing Run/Submit/Simulate Commands

### Run Command (Full Workflow)
- **`/crates/vb_cli/src/run.rs`** - Standalone run command implementations
  - `cmd_run()` - Runs workflow with YAML source + binary inputs
  - `cmd_run_compiled()` - Runs pre-compiled IR workflow
  - `cmd_validate()` - Validates workflow syntax
  - `cmd_compile()` - Compiles YAML to IR/Rust

### Workflow Execution Helper
- **`/crates/vb_cli/src/workflow.rs`** - Workflow execution helpers
  - `run_compiled_workflow()` - Full runtime execution with tick loop
  - `runtime_journal_for_mode()` - Durability journal setup

### Command Implementations in app_impl.rs
- Lines 1164-1221: `cmd_run()` - Full workflow run with durability
- Lines 1471-1509: `cmd_run_step()` - Single step isolation (already exists!)
- Lines 1712-1783: `cmd_run_compiled()` - Compiled IR run

### Workflow Analysis
- **`/crates/vb_cli/src/commands_workflow.rs`** - Pure workflow analysis
  - `simulate_workflow()` - Dry-run without execution
  - `generate_dot()` - DOT graph generation

---

## 3. Workflow Execution Engine and Step Handling

### Core Step Execution
- **`/crates/vb_core/src/engine/step.rs`** - Single-step engine
  - `step_once()` (line 20) - Executes one step, returns `EngineSignal`
  - `execute_node()` - Dispatches to node-type handlers
  - `execute_boundary_node()` - Handles Do, Wait, Ask, Jump, Finish
  - `resume_action_completion()` (line 137) - Resumes after action success
  - `resume_action_failure()` (line 184) - Resumes after action failure

### Engine Signals
- **`/crates/vb_core/src/engine/signals.rs`** - `EngineSignal` enum
  - `Continue` - Run made progress
  - `Finished(SlotValue, Taint)` - Run completed
  - `AwaitingAction` - Suspended on Do node
  - `AwaitingWait` - Suspended on WaitUntil/WaitEvent
  - `AwaitingAsk` - Suspended on Ask
  - `StepBudgetExhausted` - Budget depleted

### Runtime
- **`/crates/vb_runtime/src/runtime.rs`** - Multi-shard runtime
  - `submit_compiled_with_inputs()` (line 142) - Submit run with inputs
  - `tick_all()` (line 193) - Process one tick on all shards
  - `tick_shard()` (line 216) - Process one tick on specific shard

### Frame and State
- **`/crates/vb_core/src/frame.rs`** - `RunFrame` and `StepState`
  - `RunFrame::new()` - Creates frame with step/slot arrays
  - `StepState` enum: Pending, Running, Succeeded, Failed, Waiting, Asking, etc.

### Key Identifiers
- **`/crates/vb_core/src/ids/mod.rs`** - ID types
  - `StepIdx`, `SlotIdx`, `RunId`, `ActionId`, etc.

---

## 4. Structured Output Implementation

### Output Format Enum
- **`/crates/vb_cli/src/args.rs`** (lines 7-17)
  - `OutputFormat::Text` - Human-readable (default)
  - `OutputFormat::Json` - JSON object
  - `OutputFormat::Jsonl` - JSON Lines

### JSON Output Functions
- **`/crates/vb_cli/src/app_impl.rs`**
  - `json_out()` - Outputs JSON/JSONL with `serde_json::json!`
  - `json_error()` - Outputs error in structured format
  - `write_failure_message()` - Handles error output based on format

### Step Result Output
- **`/crates/vb_cli/src/app_impl.rs`** (lines 1620-1698)
  - `print_step_result()` - Prints step, kind, slots, output, taint, signal
  - `print_input_slots()` - Prints all slot values
  - `print_output_slot()` - Prints output slot value
  - `print_taint()` - Prints taint for output slot
  - `node_kind_name()` - Maps `CompiledNodeKind` to string

### Signal Output
- **`/crates/vb_cli/src/app_impl.rs`** (lines 1700-1710)
  - `signal_name()` - Maps `EngineSignal` to string name

---

## 5. Test Patterns for CLI Commands

### Integration Tests
- **`/crates/vb_cli/tests/cli_integration.rs`** - Main integration test file (~3217 lines)
  - Uses `run_cli()` helper to spawn binary
  - Tests with real compiled workflows via `WorkflowParts`
  - `write_test_file()` for temp file creation

### Argument Tests
- **`/crates/vb_cli/src/args/tests/run.rs`** - Run command arg parsing tests
- **`/crates/vb_cli/src/args/tests/workflow.rs`** - Workflow arg tests

### Main Tests
- **`/crates/vb_cli/src/main_tests.rs`** - Unit tests for app_impl helpers
  - `signal_name()` tests (line 721)
  - `StepTarget` construction tests

---

## 6. Key Files/APIs to Modify or Use

### For `run --step` Implementation

| File | Relevance | Purpose |
|------|-----------|---------|
| `crates/vb_cli/src/app_impl.rs` | HIGH | `cmd_run_step()` exists but needs enhanced structured output |
| `crates/vb_cli/src/args.rs` | HIGH | `StepTarget` parsing already exists |
| `crates/vb_core/src/engine/step.rs` | HIGH | `step_once()` for step execution |
| `crates/vb_core/src/engine/signals.rs` | HIGH | `EngineSignal` types for output |
| `crates/vb_core/src/frame.rs` | HIGH | `RunFrame` for slot/taint state |
| `crates/vb_cli/src/workflow.rs` | MEDIUM | May need variant for single-step journal |
| `crates/vb_runtime/src/runtime.rs` | MEDIUM | For durability-gated execution |
| `crates/vb_cli/tests/cli_integration.rs` | HIGH | Test patterns to follow |

### APIs to Use
- `vb_core::step_once(plan, run, store) -> Result<EngineSignal, EngineError>`
- `RunFrame::new(run_id, step_idx, step_count, slot_count)`
- `frame.read_slot(slot_idx)`, `frame.read_taint(slot_idx)`
- `EngineSignal` variants for structured output

### Acceptance Criteria Mapping
1. **Execute exactly one step**: `step_once()` executes single step
2. **Reports pc/slot/taint/state deltas**: Need to enhance `print_step_result()` with JSON output
3. **Respects durability gates**: Current `cmd_run_step()` requires `DurabilityMode::None` (line 1476)
4. **Tests for valid/invalid step requests**: Follow patterns in `cli_integration.rs`

---

## 7. Existing `cmd_run_step` Analysis

Current implementation at lines 1471-1509:
```rust
fn cmd_run_step(...) {
    // Requires durability=none
    // Reads workflow, compiles
    // Gets node at step_idx
    // Reads step inputs from file
    // Builds RunFrame
    // Calls vb_core::step_once()
    // Prints result via print_step_result()
}
```

**Gap**: Currently only outputs text format. Needs structured JSON/JSONL output for pc/slot/taint/state deltas as per acceptance criteria.

---

## 8. Architecture Notes

- **No unsafe, unwrap, expect, panic, todo** - All error handling via `Result`/`?`
- **postcard** for binary serialization (inputs, IR)
- **serde_json** for structured output
- **blake3** for digests
- **vb_runtime journal** for durability (Fjall key-value store)
