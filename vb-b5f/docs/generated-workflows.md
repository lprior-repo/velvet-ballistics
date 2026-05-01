# Generated Workflows

Generated Rust mode is the maximum-speed execution mode. It is not implemented yet; the current implementation executes compact IR.

## Current Path

```text
YAML source
strict cold compiler
CompiledWorkflow
numeric-slot RunFrame
synchronous engine loop
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
