# Independent CODE review: F2 composites, budget v2, retained guard/Reduce

Date: 2026-10-23 UTC. Read-only source review; no source edits, builds, formatting, or tests executed by this reviewer.

## Verdicts

- Budget v2: scoped CODE approval. Native compilation, formatting, focused tests, and unchanged compiler regression remain pending.
- Retained Repeat counter guard: the two reported implicit-writer omissions are fixed. The original body reset through a foreign output remains rejected.
- Retained Reduce evidence: correct for canonical distinct body/done targets, but a constructor-admissible equal-target shape still silently omits actual writes. This is a confirmed source-level defect, not a test result.
- F1/F2 source integration: the V43 top-level Do output reservation omission remains unresolved in the reviewed retained pin.
- No whole-F1, whole-F2, runtime, CI, or release acceptance is implied.

## Exact review inputs

1. V43 source: `6b591076ebc4423dbe01e320bd738f4b9eaa5aa7`, bookmark `codex/vb-8eocj-f1f2-v43-source`.
2. Rejected budget v1: `/workspace/shared/velvet-f2-budget-depth-v1.patch`, SHA256 `2551a9eccb476624baa07cd0f501a6ac0fc74959eab18392255dab29a559b062`, based on V43.
3. Approved budget v2: `/workspace/shared/velvet-f2-budget-depth-v2.patch`, SHA256 `d01a2754bfa60b8f2068bc38c1ba94a43059f25be26663fe7b53e7c6baf98280`, based on V43. Only `crates/vb_core/src/budget.rs` and new `crates/vb_core/tests/vb_8eocj_budget_depth.rs` change.
4. Retained guard/Reduce pin: `73f0713df1bce936491d85bf56b361d46af20b0b`, parent `5362d0934a77673dbd3916ce76802ad7d28a9601`. Reduce primitive/evidence files are unchanged from V43. Guard changes add the two writer cases below.

## Confirmed source findings

### A. V43 top-level Do outputs are absent from the pre-reservation pass

Files/functions: `vb_compile/src/mod_compile_lowering/source_slot_reservation.rs::reserve_source_slots`, `canonical_do.rs::compute_output_slot`, `part_02.rs::lower_canonical_step`.

The prepass reserves authored Set output positions, but not newly supported top-level Do output positions. Do always writes its authored step index. An earlier composite can allocate that position as private state.

Minimal trigger: Repeat at authored index 0 with a constant one-step body, then Do at index 1 with numeric input `0`, then Finish referring to that Do. Input 0 reserves one slot; Repeat allocates counter 1 and body result 2; Do writes slot 1. The counter ownership guard then rejects the source with `RepeatCounterForeignWrite`. Other composite/Do orders can overlap non-counter scratch instead.

Minimal repair: reserve top-level Do output positions alongside Set positions before any composite allocation. Regression: source compilation and execution with Repeat/Together before Do, plus interleaved composites. Sent directly to the F1 owner. No fixture was executed here.

### B. V43 Together depth counted siblings and retained completed scopes

File/functions: `vb_core/src/budget.rs::compute_fanout_and_depth`, `compute_child_depth`.

V43 increments depth for TogetherStart and each TogetherBranch, then carries depth through joins into sequential groups. Ten branches in one group therefore appear eleven levels deep and fail the unchanged policy limit of eight. Removing only branch increments does not repair sequential groups.

Budget v1 restored exit depth, but cached nodes only by incoming depth. Equal-depth visits under different owners can have different later exit restoration. Counterexample: Choose 0 visits ForEachStart 2 before ForEachStart 1; owner 1 has body 3/done 12; owner 2 has body 3/done 6; shared nodes 3→4→5→6→TogetherStart 7, whose join is 11, then Finish 12. V1 first reaches shared body at depth 1 under owner 2, restores zero at 6, and records Together at depth 1. The later owner-1 visit to shared body is skipped, hiding the required depth 2. Existing constructor forward/nesting checks admit the noncrossing spans [1,12), [2,6), [7,11). This was source-proven, not run.

V2 removes path-sensitive depth caching. It retains once-only reachable metrics, gathers reachable opening owners, sorts by numeric start, retires exits before starts at the same position, and checks active-span depth. It is order-independent and conservative for alternate-entry generic IR; ordinary canonical scopes remain exact. It retains policy limits, checked depth overflow, step counting, and once-only metrics. The 18 test declarations include sibling/sequential cases, both shared-body traversal orders with constructor acceptance, genuine loop depth 8/9, fanout/overflow, and unreachable-owner exclusion. These were inspected, not executed.

### C. Retained Repeat guard closes both reported inventory omissions

File: `vb_core/src/workflow/repeat_counter_ownership.rs::writes_slot`.

At V43, the declared conservative GLOBAL reservation omitted:

- CollectStart.source when outer output is None. Runtime `collect.rs::build_collect_start_plan` then selects source as the collector write destination.
- TogetherJoin.accumulator. Runtime `together_join` can append the final scalar result through `append_to_accumulator`, which writes that accumulator.

Pin 73f0713d adds both cases exactly. The new constructor negatives keep outer output None/private, so they target the implicit writers rather than accidentally exercising the existing outer-output check. The owned Start/paired Check exceptions and error-slot rejection remain intact.

Qualification: these implicit handlers require list data. The finding concerned the documented global reservation contract, not a demonstrated way to reset an active I64 counter. The original foreign Set/Copy output reset is rejected. Zero maximum remains invalid, while current attempt equal to maximum remains a valid exhausted state; no decoder weakening or `>=` rejection was introduced.

### D. Retained Reduce evidence is ambiguous when body equals done

Files/functions: `vb_runtime/src/engine/drive/reduce_evidence.rs::classify_reduce_start` (lines 49–64), `classify_reduce_next` (66–80); `primitives/reduce.rs::reduce_start` (67–91), `reduce_next` (95–128); `vb_core/src/workflow/mod.rs::validate_reduce_start`, `validate_reduce_next`, `validate_loop_start_edges`.

The classifier checks post-execution PC against done before body. The constructor checks target bounds and forward Start edges but does not require these targets to differ.

Minimal Start shape: ReduceStart 0 has input slot 0, accumulator 1, iterator output 2, and body=done=1; Finish 1 reads accumulator 1. With a valid nonempty source list, runtime writes accumulator, first-item input, and iterator, then jumps to 1. Evidence recognizes only the done case and emits accumulator alone. Productive ReduceNext with equal body/done similarly writes item and iterator, but emits no SlotWritten events. No evidence-gap error exposes these omissions.

Canonical lowering uses distinct body and done targets and is unaffected. For distinct targets the inspected write sets match runtime: empty Start writes accumulator only; productive Start writes accumulator/input/iterator; exhausted Next writes nothing; productive Next writes item/iterator. Deduplication handles coincident destination slots and evidence reads final values/taints.

Minimal owning repair: enforce an explicit typed body-before-done invariant for accepted Reduce IR, or propagate actual executed write-branch information rather than inferring it from ambiguous PC. Add equal-target productive negatives or exact write-set oracles, with empty/exhausted controls. No repair was made and no probe was run by this reviewer.

## Evidence, constraints, and remaining gates

- Read applicable `AGENTS.md`, `velvet-ballistics-MASTER.md`, JJ guidance, `nasa-jpl-standards.md`, and `runtime-performance-architecture.md`. AGENTS/master are unchanged between V43 and the retained pin.
- Executed source inspection via `git show`, `git diff`, `git grep`, and patch SHA256 checks. `git fetch origin codex/vb-8eocj-f1f2-v43-source` succeeded. Fetching retained pin was initially denied before execution; one identical retry after new explicit user authorization evidence succeeded. No alternative route or further retry was used.
- Power-of-Ten review: budget v2 uses checked depth conversion/increment and finite owner/node-sized collections; the new structural helpers are cold and bounded. Existing recursive metrics traversal is retained, not replaced with exponential per-context rewalking. No unsafe or panic-based repair was added. Zero-warning, formatting, and native execution gates are unverified here.
- Performance scope: no hot-path optimization or latency/throughput claim. New budget work is cold validation, with O(nodes + owners log owners + owners²) worst-case sweep work and O(nodes + owners) storage. No benchmark/profiler was run.
- Parent-reported V43 status, not independently executed here: strict suite 68/68 passed; compiler library 504/505 with ten-sibling budget failure; Collect 169/176 with legacy-contract assertions; CI timed out at 600 seconds. Later reported results must be pinned separately and do not follow from this CODE review.
- Native gates remain required for budget v2 and retained repairs. F1 lexical work and other pre-existing runtime/validation issues are outside this bounded report. No broad audit or full acceptance claim was made.
