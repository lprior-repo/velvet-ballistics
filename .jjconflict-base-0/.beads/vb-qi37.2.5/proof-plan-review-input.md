# Proof Plan Review Input — vb-qi37.2.5

## Bead Identity
- **Bead**: vb-qi37.2.5
- **Title**: quality: Boundedness adversarial tests
- **State**: 4 (Proof Planning → awaiting proof-reviewer)
- **Reviewer action**: Verify obligation-to-risk mapping is correct, lanes are cheapest sufficient, commands are accurate, and no critical risk is unmapped.

---

## Contract Clauses Under Review

| Clause | Description | Primary Risk |
|--------|-------------|---------------|
| INV-001 | StepBudget::remaining ∈ [0, MAX_STEP_BUDGET] always | overflow / invariant violation |
| INV-002 | ValueStore::total_arena_count ≤ max_arena_entries always | cap bypass / overflow |
| INV-003 | count_total_steps result bounded by MAX_STEPS_PER_WORKFLOW | u64 accumulator overflow |
| INV-004 | run_until_blocked terminates in ≤ initial_budget iterations | infinite loop with available budget |
| INV-005 | WholeWorkflowBudget fields non-decreasing across compute calls | monotonicity violation |
| INV-006 | StepBudget::try_take only mutator; decreases monotonically | monotonicity violation |
| PRE-001 | StepBudget::new(v) clamps to MAX_STEP_BUDGET without panic | panic on clamp |
| PRE-002 | ValueStore::with_max_slots enforces cap on inserts | cap bypass |
| PRE-003 | WholeWorkflowBudget::compute requires entry < nodes.len() | out-of-bounds access |
| POST-001 | try_take returns Ok(true) exactly initial_value times | count violation |
| POST-002 | StepBudget::new(v > MAX_STEP_BUDGET) returns clamped budget | clamp correctness |
| POST-003 | run_until_blocked returns StepBudgetExhausted when budget depletes | signal correctness |
| POST-004 | ValueStore insert_* returns BudgetExceeded before cap exceeded | cap enforcement |
| POST-005 | WholeWorkflowBudget::compute returns error when steps exceed limit | overflow propagation |
| POST-006 | BoundednessPolicy::validate returns Err when budget exceeds policy | validation correctness |

---

## Verifier Lane Justification

### Why Verus for INV-001 through INV-006?
These are Rust-local pure invariants in the kernel. Verus is the cheapest lane that can prove
loop termination (INV-004), monotonic decrease (INV-006), and cap enforcement (INV-002) without
requiring exhaustive enumeration of the input space. Kani complements with bounded model
checking but cannot prove loop invariants for all input sizes. Proptest provides adversarial
coverage but cannot prove absence of counterexamples.

### Why Kani for INV-001, INV-004, POST-004?
Kani provides bounded model checking as a complementary line of defense. It checks concrete
harnesses with honest bounds, finding any concrete counterexample that slips past Verus.
Kani's bounded unrolling is sufficient here because all loops have a hard external bound
(MAX_STEP_BUDGET = 10_000).

### Why Miri for INV-002?
ValueStore handles raw arena allocations and uses interior mutability. Miri catches UB,
use-after-free, and leaks that are orthogonal to functional correctness (covered by Verus)
but critical for memory safety. No other lane covers this risk.

### Why Proptest for PRE-001, PRE-002, POST-001, POST-006?
The input space (u64 for StepBudget::new, random insert sequences for ValueStore) is too
large for exhaustive testing. Proptest provides statistical coverage across input classes
with 10_000+ iterations per property, catching edge cases that static tools may miss.

### Why Fuzz for PRE-001?
FUZZ-001 targets the clamping boundary specifically — any u64 value including MAX.
Fuzz complements both Verus (which uses symbolic reasoning) and proptest (which uses
structured random generation) with corpus-driven adversarial exploration.

### Why Unit Tests for POST-003, POST-005?
Deterministic error path testing for run loop signals and overflow errors. These are
covered by integration tests in the existing test suite; proof-obligations.jsonl already
maps them.

### Why TLA+ Waived?
`run_until_blocked` is a single-threaded deterministic loop. There are no concurrent actors,
message queues, retry policies, lease mechanisms, or temporal liveness properties. The loop
termination invariant is proven by the Verus loop measure. TLA+ would not add value.

---

## Risk-to-Obligation Matrix

| Risk Tag | Covered By | Adequate? |
|----------|-----------|-----------|
| boundedness | VERUS-INV-001/002/003/004/005/006, KANI-INV-001/INV-004/POST-004, MIRI-INV-002, PROPTEST-PRE-001/PRE-002/POST-001/POST-006, FUZZ-001, UNIT-POST-003/POST-005 | Yes |
| performance | Not in scope for this bead (separate tracking bead) | N/A |
| user-visible-behavior | POST-001/002/003/004/005/006; POST-003 via UNIT-POST-003 | Yes |
| persistence | INV-002 (ValueStore cap) | Yes |
| public-api | INV-001/002/006 (StepBudget and ValueStore are public) | Yes |

---

## Open Questions

1. **vb-qi37.2.2 (value arena caps)**: If not resolved before proof execution, some Kani/Miri
   obligations may need additional assumptions about `ValueStore::with_max_slots` cap enforcement.
   Current plan assumes caps are enforced (as documented in contract.md).

2. **vb-qi37.2.4 (nested composition bounds)**: If not resolved, `count_total_steps` worst-case
   loop iteration multiplication may produce results exceeding `u64` before reaching `MAX_STEPS_PER_WORKFLOW`.
   Current plan (VERUS-INV-003) covers this; if vb-qi37.2.4 adds loop bounds, re-plan.

3. **vb_runtime (DEFERRED_GLOBAL)**: Does not affect vb_core obligations. No change to plan.

---

## Reviewer Checklist

- [ ] All contract clauses have at least one obligation mapped
- [ ] All obligations have a command with correct package/harness
- [ ] All obligations have assumptions stated explicitly
- [ ] owner_state and rerun_from are consistent with state machine (proof-writer=5, proof-reviewer=6, test-planner=7, test-writer=8)
- [ ] No safety-critical risk is unmapped or waived without compensating evidence
- [ ] TLA+ waiver is justified (single-threaded deterministic loop)
- [ ] Flux and Loom not_applicable are justified (no refinement types, no concurrency)
- [ ] DEFERRED_GLOBAL (vb_runtime) is correctly excluded from all obligations
- [ ] All commands use correct package names and harness names from delivery-scope.jsonl
