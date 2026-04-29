# Compiled IR

Compiled IR is the runtime contract between the cold compiler and the hot engine.

## Current Shape

`CompiledWorkflow` contains:

```text
workflow name for cold diagnostics
WorkflowDigest
boxed CompiledNode array
boxed constant pool
slot_count
entry StepIdx
```

`WorkflowParts` is untrusted compiler output. `CompiledWorkflow::try_from_parts` validates numeric references before the hot runtime can execute it.

## Current Node Kinds

```text
SetConst { output: SlotIdx, value: ConstIdx, next: StepIdx }
Copy { output: SlotIdx, source: SlotIdx, next: StepIdx }
Choose { condition: SlotIdx, on_true: StepIdx, on_false: StepIdx }
Finish { result: SlotIdx }
```

Outputs are variant-specific so missing writer output slots are unrepresentable.

## Validation Invariants

The IR constructor rejects:

```text
empty node arrays
entry outside node array
step targets outside node array
slot references outside slot_count
constant references outside constant pool
```

## Future IR Extensions

Upcoming phases add:

```text
WorkflowId
ExprIdx
AccessorIdx
ActionId
expression bytecode table
accessor table
native action dispatch nodes
versioned binary IR encoding
generated Rust parity tests
```
