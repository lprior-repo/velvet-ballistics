# Deferred Rust Codegen and Maxperf Track

Rust workflow code generation and `maxperf` are intentionally outside the current
Backend / IR Interpreter Complete milestone.

## Current Status

- Current execution target: compiled `CompiledWorkflow` IR through the IR interpreter.
- Current CLI compile target: `compile --emit ir` only.
- Current performance evidence target: IR interpreter, storage, IPC, direct API, scheduler.
- Current release blocker set excludes generated Rust equivalence, generated compile-fail tests,
  # allow-removed-feature: master §41 — historical deferred-scope document explicitly enumerates the removed tokens
  generated-vs-IR benchmark ratios, PGO, and `target-cpu=native` maxperf release gates.

## Deferred Scope

The following remain future work:

# allow-removed-crate: deferred-scope doc enumerates the removed codegen crate
1. `vb_codegen` as an active workspace crate.
2. `velvet-ballistics compile <workflow.yaml> --emit rust`.
3. Generated Rust execution for accepted workflow artifacts.
4. Generated Rust semantic equivalence against IR execution for:
   - terminal result,
   - typed error variants and fields,
   - final program counter,
   - slot values,
   - slot taints,
   - step states,
   - journal event sequence,
   - action tickets,
   - retry counts,
   - wait/ask scheduling,
   - replay behavior.
5. Generated Rust compile-fail tests forbidding unsafe, unwrap, expect, panic,
   unchecked indexing/slicing/casts/arithmetic, runtime YAML, JSON, HTTP, and runtime string lookup.
6. `maxperf` profile acceptance.
   # allow-removed-feature: master §41 — historical deferred-scope document explicitly enumerates the removed tokens
7. PGO training and `target-cpu=native` benchmark workflows.
8. Public generated-mode speed claims.

## Reactivation Contract

Codegen may return to the master scope only through a dedicated architecture/spec bead.
That bead must define:

- why IR interpreter performance is insufficient,
- which IR node families are accepted,
- how unsupported IR fails closed before emission,
- the exact equivalence harness,
- the compile-fail suite,
- the benchmark matrix,
- rollback behavior if generated execution diverges,
- evidence required before `maxperf` becomes a release gate.

Until then, generated Rust and maxperf are documentation-only future tracks.
