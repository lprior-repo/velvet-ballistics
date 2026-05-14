# vb-h8h0 Review: Codegen Equivalence

## Finding

The claim that `compare_generated_to_ir` is "pattern counting only" is **PARTIALLY CORRECT**.

`compare_generated_to_ir` (lib.rs:2133) does pattern rejection and counting ONLY. However, there IS a separate execution equivalence test at `proptests.rs:369-375`:

```rust
let generated = generated_equivalence_stdout(&workflow, ...)?;  // Runs generated code
let interpreted = ir_equivalence_trace(&workflow)?;            // Runs IR interpreter
prop_assert_eq!(generated, interpreted);  // Compares stdout
```

This test verifies **terminal results and taint parity** for the fixed 6-step workflow.

## Remaining Gaps

1. **Journal event parity** - stdout comparison doesn't verify journal events
2. **Error parity** - only happy path tested
3. **Workflow shape coverage** - only fixed 6-step workflow tested

## Verdict

Partial implementation exists. Full parity testing (journal + errors + workflow shapes) requires more significant work.

## Status

**DEFERRED** - Requires dedicated implementation effort for journal/event comparison infrastructure.