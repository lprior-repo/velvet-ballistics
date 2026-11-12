**Domain:** Verification pipeline budget computation error handling.

**Error:** `WholeWorkflowBudget::compute` failure currently degrades to warning in all profiles, skipping policy validation and allowing verification to succeed.

**Boundary:** Phase 4 budget computation match arm in `run_verification`. Full profile must fail verification; Standard profile must warn without claiming successful budget/policy checks; Quick profile unchanged (no budget compute).

**Acceptance:**
- Full profile + budget compute error → `VerifyError::BudgetPolicy("budget computation failed: {e}")` → exit code 4
- Standard profile + budget compute error → warning "budget computation note: {e}", no "budget_computation" or "boundedness_policy" check recorded
- Quick profile → no budget computation attempted
- Existing policy-limit behavior unchanged (Full errors, Standard warns)
- Successful budget computation path unchanged
- Tests use real `WholeWorkflowBudget::compute` calls (e.g., empty node list) for deterministic error classification; no fakegate evidence or new verifier lanes
- If no public CLI YAML triggers budget compute error, disclose that coverage is internal boundary only
