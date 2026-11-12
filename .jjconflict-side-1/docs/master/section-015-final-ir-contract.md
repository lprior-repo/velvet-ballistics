---
section: 15
title: "Final IR Contract"
parent: velvet-ballistics-MASTER.md
---

## 15. Final IR Contract


The runtime executes compiled IR only. YAML AST nodes never reach the runtime. High-level YAML primitives may lower into multiple primitive IR nodes.

Required IR coverage:

```text
Nop
SetConst
Copy
EvalExpr
BuildObject
BuildList
Do
Choose
ChooseSlot
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
WaitUntil
WaitEvent
Ask
AskResume
RetryCheck
ErrorHandler
Jump
Finish
```

Current execution is through the IR interpreter only. Generated Rust execution is removed from active master scope.

**`Finish` taint contract:** The `Finish` IR node reads the taint from the result slot and emits `EngineSignal::Finished(SlotValue, Taint)`. Taint is joined from all slots contributing to the result. Runtime preserves `Clean`, `DerivedFromSecret`, and `Secret` result taints; validation does not reject tainted finish results.

Final choose contract: `Choose { branches, otherwise }` evaluates `ExprBranch { condition: ExprIdx, target }` in order and jumps to the first true expression; `ChooseSlot { branches, otherwise }` reads `SlotBranch { condition: SlotIdx, target }` in order after those slots have been materialized by prior IR. `ChooseSlot` condition slots must be validated as boolean slots. If no branch matches, `otherwise` is taken; missing `otherwise` with no match is `CoreError::MissingNextStep { step: current }`. Untyped or string-condition choose nodes are migration-only and are not part of final IR.

---
