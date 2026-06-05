# Compiled IR

Compiled IR is the runtime contract between the cold compiler and the hot engine.

## Current Shape

`CompiledWorkflow` contains:

```text
workflow name for cold diagnostics
WorkflowDigest
boxed CompiledNode array
boxed constant pool
boxed expression program table
boxed accessor table
boxed action contract table
ResourceContract
slot_count
entry StepIdx
```

`WorkflowParts` is untrusted compiler output. `CompiledWorkflow::try_from_parts` validates numeric references before the hot runtime can execute it.

## Current Node Kinds

The authoritative `CompiledNodeKind` enum is defined in `crates/vb_core/src/nodes.rs`.
It contains **34 variants** covering all workflow primitives:

| Category | Variants |
|----------|----------|
| Control | `Nop`, `Jump`, `Finish` |
| Slot ops | `SetConst`, `Copy`, `EvalExpr` |
| Composite | `BuildObject`, `BuildList` |
| Branching | `Choose`, `ChooseSlot` |
| Iteration | `ForEachStart`, `ForEachNext`, `ForEachJoin` |
| Parallel | `TogetherStart`, `TogetherBranch`, `TogetherJoin` |
| Collection | `CollectStart`, `CollectPage`, `CollectNext`, `CollectFinish` |
| Reduction | `ReduceStart`, `ReduceNext`, `ReduceFinish` |
| Repetition | `RepeatStart`, `RepeatAttempt`, `RepeatCheck`, `RepeatFinish` |
| Temporal | `WaitUntil`, `WaitEvent` |
| External I/O | `Do`, `Ask`, `AskResume` |
| Error handling | `RetryCheck`, `ErrorHandler` |

Outputs are variant-specific so missing writer output slots are unrepresentable.

## Validation Invariants

The IR constructor rejects:

```text
empty node arrays
entry outside node array
step targets outside node array
slot references outside slot_count
constant references outside constant pool
expression references outside expression table
accessor references outside accessor table
action references outside action table
resource contracts outside bounded policy
```

## Current Artifact Boundary

The current master treats these as current-scope artifact concerns, not future extensions:

```text
WorkflowId
ExprIdx
AccessorIdx
ActionId
expression bytecode table
accessor table
native action dispatch nodes
versioned binary IR encoding
```

Generated Rust parity tests are removed from the current Backend / IR Interpreter Complete milestone. They belong to the deferred codegen track documented in `docs/deferred-codegen-maxperf.md`.
