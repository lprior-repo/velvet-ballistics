# ForEach Lowering — WIP Contract

## Status: DRAFT — NOT ACCEPTED
- Draft author: actual local Qwen on RTX 5090, CLI V214; bead assignee: Lewis
- Source: vb-8eocj source gap review, local5090
- Independent 3090 audit pending
- Implementation/tests/reviews pending
- No feature resolved at contract stage

## Scope
Document and bound the lowering gap for the canonical ForEach primitive.
This contract is advisory until the bead completes the full Rust workflow:
contract, implementation, tests, code review, test review.

## Source Facts Verified
1. `lower_canonical_for_each` (crates/vb_compile/src/mod_compile_lowering/part_02.rs:213)
   emits ForEachStart, one Set/Do via emit_single_body_set, then ForEachNext.
   It does NOT emit a ForEachJoin or materialize an output list.
2. Body target is id+1, Next is id+2, done is id+3, using checked_step_offset.
   Any future N-step flat body requires checked width/Next/done recalculation;
   no unchecked arithmetic.
3. Distinct input, private iterator, private item, and private body result
   slots are preserved. Body output aliases must stay inside the body.
4. Proposed multiple body Set/Do steps would use the actual shared
   emit_scoped_flat_body helper and a borrowed CanonicalScope only after
   its API/loop assumptions are reviewed.
5. Canonical ForEach currently parses numeric input using slot_from_text
   and does not receive CanonicalScope. Do not claim named input/item/
   expression support is already wired there.
6. The source AST (crates/vb_yaml/src/yaml_ast/types/step.rs:89) has a
   variable:String field; the canonical dispatcher currently ignores it.
   Item spelling/binding rules must be source-backed before implementation;
   do not declare $item universally canonical.
7. The compiler currently uses at_once.unwrap_or(1) as ForEachStart.limit.
   The source AST documents at_once as maximum concurrency. Runtime
   (crates/vb_runtime/src/primitives/for_each.rs:34) compares the full
   input list length with limit and returns IterationLimitExceeded if
   too large. The master separates item cardinality and concurrency bounds.
   Record this as a suspected source-semantics mismatch requiring
   independent qualification and a separate cardinality-bound policy/API
   decision. Do not endorse at_once as an iteration bound.

## Future Required Boundaries
- compile_workflow from authored source
- execute its unchanged output
- check input preservation
- full repeated body execution through real runtime wiring
- No hand-built IR substitute, tests ignored, weaker errors,
  secret values, gate waivers, or new runtime assumptions.

## Future Error Classes (Not Implemented)
Empty bodies, duplicate local names/IDs, forward/out-of-scope references,
invalid types/bounds, and slot/step overflow need exact existing/proposed
typed errors. Do not fabricate variant names when not verified.

## Out of Scope for This Contract
Nested control primitives, step controls, secrets/results/examples,
ordered-output collection, and other loop expansions remain parent F1 work.
No full ForEach or F1 completion claim.

## Blocked On
Independent 3090 audit of this draft. This contract may remain blocked
on domain decisions; it does not assert resolution of the concurrency
vs. cardinality semantics mismatch.
