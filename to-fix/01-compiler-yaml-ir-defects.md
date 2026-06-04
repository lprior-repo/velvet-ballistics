# Compiler, YAML, Validation, and IR Defects

## Status Update 2026-06-03

All defects in this file are historical unless fresh command evidence contradicts the bead graph. `vb-xi2f` and all 40 children are CLOSED, including primitive lowering (`vb-xi2f.1`, `.2`, `.12`-.`.25`, `.36`, `.37`), validated `try_from_parts` emission (`vb-xi2f.4`), trigger/vocabulary/reference parity (`vb-xi2f.5`-.`.7`, `.15`-.`.17`), diagnostics (`vb-xi2f.9`, `.10`), and digest semantic coverage (`vb-xi2f.8`, `.18`-.`.20`, `.26`-.`.39`).

The original findings below are retained as audit history, not as current open work.

## P0: Canonical compiler lowering rejects required v1 primitives

Evidence:

- `crates/vb_compile/src/mod_compile_lowering/part_02.rs:26-83` handles `Set`, `Finish`, `ForEach`, `Together`, `Collect`, `Aggregate`, `Repeat`, `Wait`, and `Ask`, then returns `UnsupportedStepPrimitive` for everything else.
- `crates/vb_yaml/src/ast/types.rs` defines `Do` and `Choose` as YAML primitives per subagent inspection.
- `crates/vb_compile/src/mod_compile_lowering/part_01.rs:126-135` only accepts `Set` inside primitive bodies; all other nested/body primitives are rejected.

Master violated:

- Section 10: required primitives are `set`, `do`, `choose`, `for_each`, `together`, `collect`, `reduce`, `repeat`, `wait`, `ask`, `finish`.
- Section 28: compiler must lower every primitive.
- Section 44 point 3: every primitive validates, compiles, runs, persists, recovers, and replays.

Impact: Valid v1 workflows using actions or branching cannot compile through the canonical source-to-IR path. Nested primitive bodies are also not v1-complete.

Suggested bead: `P0 complete source-to-final-IR lowering for all v1 primitives`

## P0: Compiler emits unchecked compiled workflows

Evidence:

- `crates/vb_compile/src/mod_compile_lowering/part_01.rs:44-57` constructs `WorkflowParts` and returns `CompiledWorkflow::from_parts_unchecked(parts)`.
- The master requires `CompiledWorkflow` construction through validated `try_from_parts()`.

Master violated:

- Section 14: `CompiledWorkflow` constructed via `try_from_parts()`.
- Section 24: IR validation is mandatory.
- Section 28: validation must be integrated into compile pipeline.

Impact: The compiler can bypass structural checks for node bounds, transition targets, resource contracts, expression stack bounds, and table references.

Suggested bead: `P0 route compiler artifact emission through CompiledWorkflow::try_from_parts`

## P0: Trigger schema diverges from master

Evidence:

- `crates/vb_compile/src/mod_compile_validation/part_05.rs:178-197` requires `webhook.path` and `webhook.method`, while the master declares `webhook: {}`.
- `crates/vb_compile/src/mod_compile_validation/part_05.rs:214-217` accepts only `event.name`, while the master declares `event.type`.

Master violated:

- Section 9 trigger contract.

Impact: Master-valid trigger examples are rejected and compile/parser/validator behavior can diverge.

Suggested bead: `P0 align trigger parser-validator-compiler with v1 contract`

## P0: Primitive vocabulary is stale in validation/compile paths

Evidence:

- `crates/vb_compile/src/mod_compile_validation/part_05.rs:65-100` reserves `parallel` and `aggregate`, not canonical `together` and `reduce` as the only current names.
- Subagent inspection found `vb_yaml` and `vb_validate` accepting `parallel`/`aggregate` and lacking canonical parity in places.

Master violated:

- Section 10 allows only canonical names plus aliases `save`, `run`, and `foreach`.

Impact: Non-contract primitive names can be accepted, and canonical v1 authoring can fail or lower through legacy naming.

Suggested bead: `P0 replace parallel-aggregate language drift with together-reduce parity`

## P0: Canonical references to prior step outputs are not implemented as specified

Evidence:

- Subagent inspection found `crates/vb_validate/src/references.rs` rejecting declared direct step-result references and returning `FutureReference` for `$steps.<id>...` shapes.

Master violated:

- Section 8: allowed references include `$step_id.x`, and all references must compile to `SlotIdx` or `AccessorIdx` before runtime.

Impact: Multi-step dataflow cannot use the documented reference model.

Suggested bead: `P0 implement canonical step output reference resolution`

## P1: Compiled digest does not cover full semantics

Evidence:

- Subagent inspection found `canonical_digest` hashes full data only for `Set` and `Finish`; other primitive details contribute mostly primitive names.

Master violated:

- Section 28: digest computation.
- Section 38: digest stability.
- Section 44 point 14: recovery/replay detect digest mismatches.

Impact: Different workflows with the same primitive sequence can plausibly share digest material despite different action, branch, loop, wait, or resource semantics.

Suggested bead: `P1 make compiled digest cover full canonical IR semantics`

## P1: Diagnostics lack real path/span contract

Evidence:

- Subagent inspection found `diagnostic_from_error` producing `Span::ZERO`, tests asserting zero span, and no YAML path field in core `Diagnostic`.

Master violated:

- Section 36: diagnostics must have code, path, span, and message.
- Section 44 point 18: stable code, path, source span, message, and cold side-table context.

Impact: AI/operator repair loops cannot reliably locate offending fields.

Suggested bead: `P1 add YAML path and real span diagnostics`

## P1: Diagnostic code format may not match stable symbolic contract

Evidence:

- Subagent inspection found numeric codes such as `E0101`, while master Section 16 lists symbolic codes such as `DUPLICATE_KEY`, `FORBIDDEN_YAML_FEATURE`, and `UNKNOWN_REFERENCE`.

Master violated:

- Section 16 validation error codes.

Impact: External tools cannot match diagnostics to the documented stable symbolic codes without undocumented translation.

Suggested bead: `P1 expose Section 16 symbolic diagnostic codes`

## P2: Validation duplication remains

Evidence:

- `vb_validate` and `vb_compile` both implement source-level schema/trigger/primitive checks.
- The trigger and primitive mismatches above are direct symptoms of this duplication.

Master violated:

- Phase 42 validation deduplication.

Impact: Divergent parser/validator/compiler behavior will keep reappearing.

Suggested bead: `P2 finish validation deduplication for source schema-control-type-taint`
