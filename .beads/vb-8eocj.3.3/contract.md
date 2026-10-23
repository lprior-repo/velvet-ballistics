# vb-8eocj.3.3 — Global Counter Reservation & PairedCheck Topology

**Validator behavior:** Conservative GLOBAL counter reservation enforced before and after loop. No decoder change; `current==max` remains valid.

**Write permissions:**
- Allowed: Owning `RepeatStart` (`outer.output`) and properly paired `RepeatCheck` (`attempt_slot`/`outer.output`).
- Forbidden: Both owner and `Check` `outer.error_slot`.

**PairedCheck topology:** Must match owner body/done/counter. Ordered `body < check < done`, `next == body`. Prior topology validates nearest active owner.

**Implicit writer inventory:** `ForEachStart.item_slot`, `ForEachNext.iterator_slot`, `TogetherBranch.accumulator`, `CollectNext.collector_slot`, `ReduceStart.input+accumulator`, `ReduceNext.iterator_slot`, `RepeatCheck.attempt_slot`, `RetryCheck.policy_slot`, `ErrorHandler KIND.error_slot` (indirect failure writer). All `outer.output`/`outer.error_slot` checked.

**Validation complexity:** O(owners*nodes) cold validation. Conservative reservation compatibility requires independent review.

**Evidence:** V27 combined 46 PASS / 219 core PASS (previous-source). V28 ErrorHandler arm/test target NOT yet tested.

**Test plan:** Strong typed negative tests; unchanged compiler-source max1/max3 positive plan. Historical unconditional observational probe removed from autodiscovery after preserving exact .beads historical copy, replaced by strong regressions.

**Status:** No closure claim.