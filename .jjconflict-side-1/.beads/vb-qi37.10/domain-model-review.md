# Domain Model Review: vb-qi37.10

## Review Boundary

This review covers the domain model needed for generated final IR coverage/parity only. It treats `vb_core::CompiledWorkflow`, `CompiledNodeKind`, expression/accessor programs, `vb_runtime` primitive semantics, and storage journal event kinds as existing domain facts. It does not propose production-code changes.

## Core Model

- `CompiledWorkflow`: accepted artifact-level workflow model. Generated mode must not read YAML or perform runtime string reference/action lookup.
- `CompiledNodeKind`: final IR state transition vocabulary. Every variant requires an explicit generated support/rejection decision.
- `RunFrame`: runtime mutable state: pc, executed counter, step states, slots, and taint. Generated mode may represent this directly or in generated-specific structs, but observable behavior must refine the same model.
- `StepState`: finite state machine. Generated mode must preserve legal transition traces and typed invalid-transition errors.
- `SlotValue`: handle/scalar value domain. Generated mode must preserve handle identity semantics enough for parity tests over values, lists, objects, blobs, and symbols.
- `Taint`: value confidentiality lattice with `Clean`, `DerivedFromSecret`, and `Secret`. Generated helpers must join taint as the IR/runtime oracle does.
- `JournalEvent`/journal signature: storage byte envelopes are out of scope, but semantic event kind/order/essential fields are in scope for parity.

## Outcome Lattice

Generated and IR execution must map to this finite observable lattice:

```text
Outcome ==
  Finished(value, taint, pc, steps, journal_signature)
  Suspended(Action | Wait | Ask, ticket_or_deadline, pc, step_states, journal_signature)
  StepBudgetExhausted(pc, remaining_budget, step_states, journal_signature)
  Failed(typed_error_variant, typed_error_fields, pc, step_states, journal_signature)
  RejectedAtCodegen(typed_codegen_error_variant, fields)
```

Contract implication: `RejectedAtCodegen` is valid only for explicitly unsupported generated features. If a feature is accepted by validation, runtime mismatch must be treated as a bead-local defect, not as a permissible outcome.

## Final IR Family Review

| Family | Generated contract decision |
|---|---|
| `Nop`, `SetConst`, `Copy`, `Jump`, `Finish` | Must remain accepted with executable parity evidence for pc/state/slot/taint/result. |
| `EvalExpr`, `BuildObject`, `BuildList` | Must remain accepted with helper/accessor/taint parity evidence. |
| `Choose`, `ChooseSlot` | Must preserve ordered branch evaluation, boolean-slot checks, missing-otherwise typed error, and pc parity. |
| `ForEachStart`, `ForEachNext`, `ForEachJoin` | Must remain accepted and serve as a parity pattern for bounded iteration families. |
| `TogetherStart`, `TogetherBranch`, `TogetherJoin` | Must be implemented with branch/join/result/taint/journal parity or fail-closed with blocker; closure cannot claim full final IR coverage if unsupported. |
| `ReduceStart`, `ReduceNext`, `ReduceFinish` | Must be implemented with accumulator/item/tail/finish-taint parity or fail-closed with blocker. |
| `RepeatStart`, `RepeatAttempt`, `RepeatCheck`, `RepeatFinish` | Must be implemented with packed attempt state, check routing, attempt bounds, and finish parity or fail-closed with blocker. |
| `CollectStart`, `CollectPage`, `CollectNext`, `CollectFinish` | Highest-risk family. Must preserve pagination state, page lineage, duplicate/stale handling, materialization, typed errors, and bounded side state or remain a named blocker. |
| `Do`, `WaitUntil`, `WaitEvent`, `Ask`, `AskResume`, `RetryCheck`, `ErrorHandler` | In scope only for generated coverage/parity touched by this bead; detailed suspension-error expansion is `vb-qi37.11`. |

## Expression/Accessor Review

- Accepted generated helpers (`Append`, `AppendIf`, `Merge`, `Sum`, `Count`, `Unique`, `Has`, `Exists`, `Length`, `Empty`) require executable value and taint parity.
- Accessor traversal requires object/list/missing/type/taint parity.
- Text helpers (`Contains`, `StartsWith`, `EndsWith`) currently lack generated symbol/text-store semantics. They are either implemented with parity evidence or rejected with a blocker; silent fallback is invalid.
- F64 paths are high-risk because generated code must not use unchecked casts. Any numeric implementation must use finite checked conversion semantics or reject unsupported conversions typed.

## Bounded Store Review

Generated stores are domain objects, not convenience vectors. Required properties:

- explicit maximum capacity from resource contract or generated static bound;
- checked handle creation and lookup;
- no unchecked indexing/slicing/arithmetic;
- typed capacity/index/overflow errors;
- stable taint association for every stored item/field;
- deterministic iteration order matching runtime oracle.

## Journal Signature Review

The bead should compare normalized signatures, not storage byte envelopes:

- monotonic sequence position;
- event kind;
- step id where applicable;
- slot id, value kind/digest, and taint for slot writes;
- action/wait/ask/retry ticket fields where applicable;
- terminal success/failure/cancel event;
- typed error identity and fields for failures.

Full persistence envelope validation and replay hydration remain storage/recovery scope, not this bead.

## Rejected Domain Smells

- Treating source-pattern counting in `compare_generated_to_ir` as semantic parity evidence.
- A trybuild compile-fail test suite that passes with zero fixtures.
- Generic string errors for generated unsupported features.
- Generated helper stores using unbounded growth or unchecked handles.
- Implementing only the duplicate `src/codegen/mod.rs` surface.
- Closing `vb-qi37.10` while required final IR families remain unsupported without an approved blocker and revised acceptance contract.
