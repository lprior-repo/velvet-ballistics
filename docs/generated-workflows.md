# Generated Workflows

> **Deferred:** Generated Rust workflow execution is not part of the current
> Backend / IR Interpreter Complete milestone. See
> `docs/deferred-codegen-maxperf.md` for the reactivation contract.

Generated Rust mode is the preserved future maximum-speed execution track. The
historical prototype emitted Rust for a bounded subset of `CompiledWorkflow` IR
and rejected everything outside that subset before writing generated source.

## Historical / Target Path

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

The deferred generated-mode prototype accepted:

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

## Deferred Target Command

```bash
velvet-ballistics compile workflow.yaml --emit rust --out generated/issue_triage.rs
```

## Generated Code Rules

If reactivated, generated Rust must obey the same first-party rules:

```text
no unsafe
no unwrap
no expect
no panic
no unchecked indexing
no JSON
no runtime string reference resolution
```

Future generated artifacts must include:

```text
StepIdx constants
SlotIdx constants
expression functions
drive function
```

## Acceptance

If reactivated, generated workflows must compile, produce the same results as IR mode, and beat or justify their performance versus IR mode in benchmark output.
