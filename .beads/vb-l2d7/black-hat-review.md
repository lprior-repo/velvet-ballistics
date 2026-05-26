# Black Hat Review — vb-l2d7 Retry 15

STATUS: APPROVED

## Scope

- Workspace: `/home/lewis/src/vb-l2d7`
- State: Femdation State 5.5 Black Hat
- Retry: 15
- Bead: `vb-l2d7`
- Review mode: adversarial review only. No production/test code edits made by this pass.

## Commands Executed

```text
rtk cargo nextest run -p velvet-ballistics-workspace --test vb_l2d7_doc_reconciliation_contract_red
=> cargo nextest: 65 passed (1 binary, 1.931s)

rtk cargo nextest run -p vb_runtime --test vb_l2d7_joined_taint_propagation_red
=> cargo nextest: 24 passed (1 binary, 0.301s)

python scripts/check-doc-taint-consistency.py velvet-ballistics-MASTER.md
=> doc taint consistency: PASS

L="$HOME/.claude/skills/red-queen/liza-advanced.nu"; nu "$L" validate vb-l2d7-redqueen-r15
=> validation printed 9/9 passed and ALL CHECKS PASS; the tool then emitted a stale task lookup error. I did not count that trailing tool-state glitch as semantic failure because the nine checks executed and the direct commands/probes below independently reproduced the evidence.
```

Additional adversarial Finish probes executed by this review:

```text
valid_plus_secret_reject:
  input: Finish emits EngineSignal::Finished(SlotValue, Taint). Compile-time rejects Secret finish results.
  result: rc=1, doc taint consistency: FAIL, finish rejection contradiction

valid_plus_derived_reject:
  input: Finish emits EngineSignal::Finished(SlotValue, Taint). Finish rejects DerivedFromSecret result taint.
  result: rc=1, doc taint consistency: FAIL, finish rejection contradiction

valid_plus_runtime_reject:
  input: Finish emits EngineSignal::Finished(SlotValue, Taint). Finish rejects Secret taint values at runtime.
  result: rc=1, doc taint consistency: FAIL, finish rejection contradiction

allowed_no_rejection:
  input: Finish emits EngineSignal::Finished(SlotValue, Taint). No rejection of Secret or DerivedFromSecret results.
  result: rc=0, doc taint consistency: PASS
```

Static audit commands also reported:

```text
hardcoded_paths=none for:
- scripts/check-doc-taint-consistency.py
- crates/vb_doc/src/reconcile.rs
- crates/vb_runtime/src/taint.rs
- crates/vb_runtime/tests/vb_l2d7_joined_taint_propagation_red.rs

function length scan:
- no functions over 25 nonblank/noncomment lines in focused files

panic/unsafe scan in focused production files:
- no unwrap/expect/panic/todo/unimplemented/dbg/unsafe/as findings
```

## Phase 1 — Contract & Bead Parity

PASS.

- `velvet-ballistics-MASTER.md:602` and `2015` use `Finished(SlotValue, Taint)`.
- `velvet-ballistics-MASTER.md:609` states validation does not reject `Secret` or `DerivedFromSecret` finish results and runtime preserves result-slot taint.
- `velvet-ballistics-MASTER.md:658`, `2127`, `2557`, and `3537` agree: `Finish` passes result taint through and does not reject tainted finish results.
- Stale Clean-only wording remains only for `SetConst` (`velvet-ballistics-MASTER.md:2041`), which is not one of the resolved DRIFT-1 joined-taint nodes.
- Focused doc suite passed 65/65.

## Phase 2 — Farley Engineering Rigor

PASS.

- The previous oversized `validate_taint_vocabulary_consistency` blob is split. Focused function-length scan found no function over 25 nonblank/noncomment lines.
- `scripts/check-doc-taint-consistency.py` now rejects stale `Finished(SlotValue)` via regex and rejects broad Finish rejection contradictions through `is_finish_rejection_contradiction` rather than one brittle exact sentence.
- Runtime tests now assert behavior: joined taint output, empty contributor rejection, and Finish taint preservation. The previous constructor-layout mirror-test bulk is gone.

## Phase 3 — Holzman Rust

PASS.

- Non-empty contributor domain exists: `ContributorTaints` in `crates/vb_runtime/src/taint.rs:5-22`.
- `ResolvedNodeTaintInput::{eval_expr, build_object, build_list}` parse raw vectors through `ContributorTaints::try_new` at construction (`crates/vb_runtime/src/taint.rs:40-57`). Empty contributors are not representable inside the node variants.
- `resolved_node_output_taint` consumes trusted typed input and preserves `Finish` taint (`crates/vb_runtime/src/taint.rs:64-72`).
- Focused production scan found no panic vector or unsafe vector.

## Phase 4 — DDD / Scott Wlaschin

PASS.

- The contract language is now coherent: data-flow taint joins for `EvalExpr`, `BuildObject`, `BuildList`; `Finish` passes result-slot taint to `EngineSignal::Finished(SlotValue, Taint)`; v1 control-flow taint remains explicitly out of scope.
- The script is still string-based, but it now encodes the bead-owned invariant that mattered: valid Finish taint wording plus contradictory rejection text fails closed. That is enough for this documentation reconciliation bead.

## Phase 5 — Bitter Truth

PASS.

The retry finally kills the previous green-check theater. The exact exploit class — good `Finished(SlotValue, Taint)` wording plus contradictory `Secret`/`DerivedFromSecret` rejection text — now fails closed. Runtime companion behavior is typed, boring, and adequately tested for this bead.

## Residual Non-Blocking Notes

- `crates/vb_doc/src/reconcile.rs` is 384 lines. That is not pretty. I am not blocking this bead on it because the enforced Farley hard gate is function length, and the focused function scan is clean. File splitting can be a follow-up if the project wants a strict <300-line file policy here.
- Red Queen validation command printed all 9 checks passing, then emitted a stale task lookup error. Direct command reproduction and probe evidence are clean; if the orchestration layer cares, file that against Red Queen/Liza state handling, not this bead's implementation.

## Verdict

STATUS: APPROVED

Next state target: State 6 / landing-quality gate, with no Black Hat blockers remaining for vb-l2d7.
