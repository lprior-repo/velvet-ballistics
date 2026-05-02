# Generated Workflows

Generated Rust mode is the maximum-speed execution mode. The current
implementation emits Rust for a bounded subset of `CompiledWorkflow` IR and
rejects everything outside that subset before writing generated source.

## Current Path

```text
YAML source
strict cold compiler
CompiledWorkflow
generated subset validation
numeric-slot RunFrame
generated Rust drive loop
```

Workflows outside the generated subset must continue through the compact IR
interpreter/runtime path.

## Supported Subset

Generated mode currently accepts:

```text
scalar constants
slot copies
expression math and boolean comparisons
action dispatch
waits
asks
jumps
choices
error handlers
finish nodes
empty/root accessors as checked root-slot reads
```

The accepted expression operations are:

```text
LoadSlot
LoadConst
LoadAccessor for empty/root accessors only
Eq
NotEq
Gt
Gte
Lt
Lte
And
Or
Not
Add
Sub
Mul
Div
Exists
```

## Rejected Subset

Generated mode rejects unsupported IR with the typed diagnostic
`CodegenError::UnsupportedIr` before source emission. The display form is stable:

```text
unsupported generated Rust IR feature: <feature>
```

Known rejected features include:

```text
BuildObject
BuildList
ForEachStart
ForEachNext
ForEachJoin
TogetherStart
TogetherBranch
TogetherJoin
CollectStart
CollectPage
CollectNext
CollectFinish
ReduceStart
ReduceNext
ReduceFinish
RepeatStart
RepeatAttempt
RepeatCheck
RepeatFinish
RetryCheck
accessor traversal
contains
starts_with
ends_with
has
length
empty
append
append_if
merge
sum
count
unique
```

## Target Command

```bash
velvet-ballastics compile workflow.yaml --emit rust --out generated/issue_triage.rs
```

## Generated Code Rules

Generated Rust must obey the same first-party rules:

```text
no unsafe
no unwrap
no expect
no panic
no unchecked indexing
no JSON
no runtime string reference resolution
```

Generated artifacts must include:

```text
StepIdx constants
SlotIdx constants
expression functions
drive function
```

## Acceptance

Generated workflows must compile, produce the same results as IR mode, and beat or justify their performance versus IR mode in benchmark output.
