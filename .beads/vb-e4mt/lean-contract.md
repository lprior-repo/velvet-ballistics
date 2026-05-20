# Theorem Kernel Projection — vb-e4mt

**Bead**: vb-e4mt — bdd: Resource bounds and budget enforcement acceptance scenarios
**State**: 3 (contract)

---

## Boundary

| Layer | Owner | Scope |
|-------|-------|-------|
| TLA+ temporal model | `WorkflowBudgetSpec`, `AggregateResourceSpec`, `StepBudgetSpec` | Workflow admission boundedness, aggregate lifecycle, step budget signaling |
| Verus-owned Rust core | `WholeWorkflowBudget::compute`, `BoundednessPolicy::validate`, `AggregateResourceUsage` methods | Pure budget computation, policy validation, aggregate accounting |
| Theorem kernel | `vb_proof_kernels::resource_budget` | Sequential/branch/loop composition correctness |
| Rust/runtime shell | Frame pool, step budget consumption, expression stack | I/O-free state management |
| External systems | Action execution, persistence, network | Excluded from proof |

---

## Theorem-Owned Clauses

### THM-BUDGET-001: sequential_compose preserves policy bounds
- **Contract clauses**: INV-001, PRE-002
- **Rust/spec target**: `vb_proof_kernels::resource_budget::sequential_compose`
- **Lean module**: `ResourceBudget.SequentialCompose`
- **Theorem shape**: `forall p: Policy, a b: Budget, sequential_compose(a, b).within(p) = a.within(p) ++ b.within(p)`
- **Model**: Abstract `Budget` and `Policy` with 5 fields (steps, actions, parallel, run_time, result_bytes); saturation modeled as `min(value + other, u64::MAX)`
- **Refinement**: `sequential_compose` in kernel refines `WholeWorkflowBudget::compute` sequential node traversal
- **Shell exclusions**: I/O, async, storage, wall-clock time, frame pool, step budget
- **Evidence command**: `moon run :verify-proof` (Verus lane) or Lean if extracted

### THM-BUDGET-002: branch_compose max preserves policy bounds
- **Contract clauses**: INV-001, PRE-002
- **Rust/spec target**: `vb_proof_kernels::resource_budget::branch_compose`
- **Lean module**: `ResourceBudget.BranchCompose`
- **Theorem shape**: `forall p: Policy, a b: Budget, branch_compose(a, b).within(p) = a.within(p) ++ b.within(p)`
- **Model**: `max` composition for all fields
- **Refinement**: `branch_compose` in kernel refines conditional/branch node traversal in `WholeWorkflowBudget::compute`
- **Shell exclusions**: Same as THM-BUDGET-001

### THM-BUDGET-003: loop_compose multiplicative bound
- **Contract clauses**: INV-001, PRE-002
- **Rust/spec target**: `vb_proof_kernels::resource_budget::loop_compose`
- **Lean module**: `ResourceBudget.LoopCompose`
- **Theorem shape**: `forall p: Policy, body: Budget, n: u64, loop_compose(body, n).within(p) = body.within(p)` when `n = 0`, else violations if `body.violates(p)` or `body * n` exceeds policy
- **Model**: `saturating_mul` modeled as `min(body * n, u64::MAX)`
- **Refinement**: `loop_compose` in kernel refines loop/repeat node traversal in `WholeWorkflowBudget::compute`
- **Shell exclusions**: Same as THM-BUDGET-001

### THM-BUDGET-004: Policy::within soundness
- **Contract clauses**: INV-001, POST-002
- **Rust/spec target**: `vb_proof_kernels::resource_budget::Policy::within`
- **Lean module**: `ResourceBudget.PolicyWithin`
- **Theorem shape**: `forall p: Policy, b: Budget, p.within(b) = []` iff `b` satisfies all policy limits
- **Refinement**: Kernel `Policy::within` refines `BoundednessPolicy::validate` 5-dim check
- **Shell exclusions**: Same as THM-BUDGET-001

---

## Verus-Owned Clauses (Not Extracted to Lean)

These remain in Verus as they are tightly bound to Rust types and don't need Lean extraction:

| Clause | Rust Target | Reason |
|--------|------------|--------|
| PRE-001 / POST-001 | `WholeWorkflowBudget::compute` | Entry bounds, overflow-safe IR walk, finite output |
| POST-002 | `BoundednessPolicy::validate` | 8-dim exact bound checks, not the 5-dim kernel subset |
| POST-003 / POST-004 | `AggregateResourceUsage::try_add_budget / fits_within` | 12-dim aggregate accounting, overflow detection |
| INV-004 | `check_expr_stack_bound` | Expression stack depth computation |

---

## Waivers

- **WAIVER-LEAN-001**: The 15-dimension `WholeWorkflowBudget` is not fully extracted to Lean; the kernel uses 5 dimensions. Rationale: the composition theorems (`sequential_compose`, `branch_compose`, `loop_compose`) are dimension-agnostic; the 5-dim model is representative. Full 15-dim proof would require significant Lean engineering with marginal returns for this bead.
- **WAIVER-LEAN-002**: Frame pool boundedness is not in the theorem kernel. Rationale: `FramePoolKey = (u16, u16)` key space is trivially finite; covered by type bounds and integration tests.
- **WAIVER-LEAN-003**: Step budget per-tick exhaustion is runtime behavior, not a pure composition problem. Rationale: covered by TLA+ model and integration tests.
