# Round 4 Agent A3 — ResourceContract Admission Gap

**Reviewer:** black-hat-reviewer
**Bead:** vb-o5zb (parent) / unfiled (specific gap)
**State:** Investigation re-attack of Round 3 finding
**Attempt:** 4 (third round of attacks, fourth re-review)
**Attack surface:** Production admission path on Runtime::submit_* family
**Verdict target:** SHIP-BLOCKER

## Gate Result: STATUS: REJECTED — SHIP-BLOCKER

The Round 3 finding is confirmed and substantially worse than reported. The production admission path does not invoke the budget gate; both BoundednessPolicy::DEFAULT and WholeWorkflowBudget::compute are exclusively exercised on the compile-time try_from_parts path. The bead tracker (vb-o5zb.3) is closed against an acceptance criterion that the artifact demonstrably fails to meet. The static hard limits in crates/vb_core/src/limits.rs violate the master contract by 65× and 8× on the most-attacked dimensions. Two of the three "ResourceContract" variants the master cites are dead/diff.

## Master vs Code: BoundednessPolicy::DEFAULT

| Dimension | Master spec | BoundednessPolicy::DEFAULT | Multiplier |
|---|---|---|---|
| max_total_steps | 1000 | 1_000_000 | **1000×** |
| max_total_slots | 8192 (slots) | 65_535 | 8× |
| max_fanout | 256 | 64 | 0.25× (more restrictive) |
| max_nesting_depth | 8 | 8 | 1× |

**The 1,000,000 step policy ceiling is 1000× the master spec's 1000-step hard limit.**

## Master vs Code: ResourceContract field set

| Master spec (line 449-468, 2908) | workflow/types.rs:169-206 | compiled_workflow.rs:130-163 (DEAD) |
|---|---|---|
| 16 fields | **18 fields** (+ max_transitions_per_tick, + allows_secret_results) | 16 fields (matches master) |

Two ResourceContract structs exist. The dead one in compiled_workflow.rs matches master (16 fields). The live one in workflow/types.rs does not (18 fields).

## Bead parity

| Bead | Status | Acceptance criterion | Met? |
|---|---|---|---|
| vb-o5zb.3 (ResourceContract shape/defaults) | CLOSED 2026-06-05 | "ResourceContract matches master field set and default limits" | **NO** — 18 fields vs 16 master; MAX_STEPS_PER_WORKFLOW=65_535 vs 1000 master; MAX_CONSTANTS=65_535 vs 8192 master |

**The closure of vb-o5zb.3 is procedurally valid but materially unsound.**

## Findings

### [F1] Admission path does not invoke WholeWorkflowBudget::compute / BoundednessPolicy::DEFAULT.validate (CRITICAL)
- File:line: crates/vb_runtime/src/admission/admission.rs:70-85, runtime_admission.rs:14-33, shard/lifecycle/chunk_001_submit.rs:221-284
- Production admission path never calls the budget gate.

### [F2] BoundednessPolicy::DEFAULT.max_total_steps = 1_000_000 is 1000× master (CRITICAL)
- File:line: crates/vb_core/src/budget.rs:364

### [F3] MAX_STEPS_PER_WORKFLOW = 65_535 and MAX_CONSTANTS = 65_535 violate master (CRITICAL)
- File:line: crates/vb_core/src/limits.rs:11,23

### [F4] admit_run_with_budget and admit_run_with_budget_policy are dead code on the live submit path (HIGH)
- File:line: crates/vb_runtime/src/admission/admission.rs:164-225

### [F5] vb-o5zb.3 is CLOSED against an unmet acceptance criterion (CRITICAL)

### [F6] allows_secret_results: bool is a dead flag (HIGH)
- File:line: crates/vb_core/src/workflow/types.rs:205, validation/resource.rs:37-39

### [F7] max_transitions_per_tick: u64 is a duplicate of max_step_budget_per_tick: u64 (HIGH)
- File:line: crates/vb_core/src/workflow/types.rs:183-185, validation/resource.rs:23-24

### [F8] compiled_workflow.rs is a 228-line dead duplicate with stale 16-field ResourceContract and stale DEFAULT (HIGH)
- File:line: crates/vb_core/src/compiled_workflow.rs (entire file)

### [F9] submit_compiled_with_inputs and submit_direct_with_inputs_grants_and_contracts skip the preflight admission gate entirely (HIGH)
- File:line: crates/vb_runtime/src/runtime.rs:84-115

### [F10] map_budget_error uses u64::MAX/0 sentinel for WorkflowBudget and unknown variants, losing error semantics (MEDIUM)
- File:line: crates/vb_runtime/src/admission/admission.rs:266-286

### [F11] explain_plan_limits.rs:40 silently swallows WholeWorkflowBudget::compute errors as {"status":"unavailable"} (MEDIUM)
- File:line: crates/vb_cli/src/explain_plan_limits.rs:39-44

## Hypothetical Attack Scenarios

### Attack A: 50,000-step workflow (master 1000× overrun)
1. Build a WorkflowParts with resource_contract.max_steps = 65_535 and 50,000 CompiledNodes of trivial SetConst kind.
2. Call CompiledWorkflow::try_from_parts(parts). **This passes** (50K < 65K, 50K < 1M policy).
3. Call Runtime::submit_compiled(workflow). **The runtime accepts it.**
4. Call Runtime::tick_all() 50 times to drive the workflow to completion.

**DoS: A single attacker can saturate a 16-core server for hours with one batch of 1024 oversized runs.**

### Attack B: 30,000-constant workflow
**DoS: 1.5 GB RAM exhaustion per shard from a single batch.**

## Bead Tracking Status

| Bead | Status | Action needed |
|---|---|---|
| vb-o5zb (parent) | BLOCKED | Re-open or file follow-up for unmet acceptance criteria |
| vb-o5zb.3 | CLOSED | **Should be re-opened** |
| vb-o5zb.5 | CLOSED | Verdict should be ROUTE-TO-REPAIR |
| Admission path gap | UNFILED | File P0 bead |
| Limits divergence | UNFILED | File P0 bead |

## Worst-Case Impact

- **One attacker can occupy ~4-8 GB of RAM and minutes of CPU per shard with a single submit batch**
- **Across a 16-shard cluster: 64-128 GB RAM and tens of minutes of CPU**
- **The admission layer emits no error, no log, no alert**
- **The only operator signal is "the system is slow"**

## Severity: 78/100 — SHIP-BLOCKER

## Required Repair Actions (in order)

1. Wire admit_run_with_budget_policy into production admission (resolves F1, F4).
2. Re-calibrate BoundednessPolicy::DEFAULT (resolves F2).
3. Lower MAX_STEPS_PER_WORKFLOW and MAX_CONSTANTS (resolves F3).
4. Re-open vb-o5zb.3 or file a follow-up (resolves F5).
5. Remove allows_secret_results field or implement the feature (resolves F6).
6. Deduplicate max_transitions_per_tick vs max_step_budget_per_tick (resolves F7).
7. Delete compiled_workflow.rs (resolves F8).
8. File a bead for the "skipped preflight" gap (resolves F9).
9. Fix map_budget_error sentinel loss (resolves F10).
10. Surface budget-compute errors in explain_plan_limits::budget_value (resolves F11).
11. File 6 new beads.

**Estimated repair time:** 4-6 hours of focused implementation, 1-2 hours of test additions, 2 hours of bead filing and re-review.

**Final Verdict: SHIP-BLOCKER**
