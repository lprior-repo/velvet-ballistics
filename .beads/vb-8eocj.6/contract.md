# vb-8eocj.6 — Canonical ForEach Increment Contract

**Status:** PROPOSED / NOT ACCEPTED — pending independent actual3090 review  
**Author:** actual5090  
**Assignee:** Lewis  
**Scope:** Proposed source-to-runtime flat multi-step ForEach over direct declared bounded list inputs; all proposed behavior below awaits implementation and independent review.

## 1. Input and Bound Extraction

- Input must be a declared list slot with a finite `max` bound in its schema.
- Compiler extracts `max` via checked `u32` conversion; carries bound metadata alongside slot and type maps.
- Missing or non-finite `max` is rejected at compile time. No implicit `u32::MAX` or policy fallback.
- `max: 0` means empty-only; any non-empty list fails at runtime.
- Derived or unproven list-bound references are unsupported and rejected.

## 2. Cardinality vs Concurrency

- `ForEachStart.limit` receives the independent cardinality bound from schema `max`.
- `at_once` is a separate concurrency parameter: omitted or `1` means explicitly SERIAL.
- `at_once` values of `0` or `>1` are rejected with a precise compile error (existing or newly defined).
- A list with multiple items is valid at `at_once: 1` given an independent finite `max`.


The new internal bound metadata is a proposal, not an existing `CanonicalScope` API. Key it by the resolved declared input `SlotIdx`, retain direct declaration origin and element type, and prevent aliases/derived lists from accidentally acquiring bounds. Shared default resource/boundedness validation remains mandatory; a large explicit maximum does not bypass policy.


## 3. Proposed type contract

- Item ExprType derives from declared element schema: number→I64, boolean→Bool, text→Text, object→Object, any→Unknown.
- Unsupported or nested element types are rejected by existing schema validation; no synthetic I64 for `any`.
- No inferred Do result type; typed unsupported arithmetic fails closed at compile time.
- All bindings are local and cloned; item and body locals do not persist after loop completion.

## 4. Proposed exact diagnostics using existing variants

- Missing, non-list, derived, or numeric ForEach input: `StepFieldShape{step, field:"for_each.input", expected:"direct declared list input with finite schema max"}`.
- Unknown `$input.missing` fails resolver first as `UnknownReferenceName`; not converted to generic shape error.
- `at_once` 0 or >1: `StepFieldShape{step, field:"for_each.at_once", expected:"omitted or 1 until concurrent for_each lowering is implemented"}`.
- `max` > u32::MAX: `InvalidInputSchema{field:"inputs.max", expected:"list maximum representable as u32"}`.
- Reserved/invalid variable name: `InvalidName{field:"for_each.variable", value:actual}`.
- Item binding collides with inherited/local output names or step IDs: `DuplicateOutputName{name:variable}`.
- Ordinary duplicate step IDs retain `DuplicateStepId`; negative/malformed bounds preserve prior schema diagnostics.

## 5. Proposed lexical binding

- Valid bare name bound as bare outputs key; direct `$variable` resolver strips dollar prefix.
- Declared `$input.foo` keys retain dollar prefix in resolver namespace.
- Item binding cannot be overwritten by body Set output or step ID, even if shared emitter permits shadowing.
- Explicit proposed protection required before shared emission to prevent accidental binding capture.

## 6. Proposed checked layout and ownership

- For N≥1 flat body: Start at S; body at S+1 through S+N; Next at S+N+1; done at S+N+2 (actual next top-level start).
- All offset/usize/u16 conversions checked via existing `StepIndexOutOfRange`/`checked_step_offset`.
- Start.output is fresh iterator; fresh item; each body output fresh; Next.output is item; same iterator private reference.
- Body last.next→Next, earlier.next→following body; Start/Next.body→first body.
- No extra result slot or Join promise; empty body retains exact existing body rejection.
- Nested primitive/control rejects retained; shared `emit_scoped_flat_body` supports flat Set/Do lexical local refs.
- Actual runtime `body_entry` rearm must be exercised across repeated items.

## 7. Required future acceptance behavior

- `compile_workflow(&[u8])` → unchanged actual compiled workflow → real shared runtime.
- len 0/1/3 with schema max 3 and at_once 1 includes all three full body iterations.
- len 4/max 3 yields exact `EngineError::IterationLimitExceeded{resource:"for_each_limit"}`.
- max 0 accepts empty list, rejects non-empty; input list/taint preserved, item refreshed each iteration.
- Distinct private output slots; two dependent named Set outputs plus registered real Do each iteration.
- Action input/result/effect exact ordered observations; out-of-scope references rejected.
- Schema max changes digest/budget/IR limit independently at once; original numeric source topology fixtures migrate to valid bounded source only while retaining exact ownership/layout assertions.

## 8. Delivery and limits

- Contract author: actual local5090 responses V216/V219; root mechanically assembled these sections. Independent actual3090 review PENDING.
- Implementation, behavioral tests, independent CODE review, independent TEST review and native acceptance are PENDING. No code has been changed for this contract.
- Raw numeric helper APIs remain separate and unchanged. Existing canonical source fixtures may migrate only to valid bounded source while preserving exact original ownership/layout assertions.
- Ordered output/Join, concurrency greater than one, derived lists, arbitrary nesting/control fields and source action-name resolution remain parent F1 work. This increment does not complete ForEach or F1.
- No Go-skill/full proof-planning stage; formal-harness GOD RULES and the master acceptance contract remain in force.
