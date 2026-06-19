# Proof Strategy — vb-z280t

**Bead:** vb-z280t — bridge resource_budget spec (nat) to production saturating arithmetic
**Master sections:** §40, §44, §64
**Status:** PARTIAL → planning full closure

## 1. Problem Frame

`crates/vb_proof_kernels/src/resource_budget/spec.rs` defines `spec_loop_mul`
using `nat` (unbounded natural numbers) for all 12 fields, while production
`cargo_kernel::Budget::loop_mul` (in the same `budget.rs` file under
`#[cfg(not(verus_keep_ghost))]`) uses `u64::saturating_mul` on each field.

The current bead added a `lemma_loop_mul_saturated_eq_production` whose
`ensures` clause restates the saturation property. However, the lemma body
is empty (trivial `{}`), so it does NOT actually prove that the spec result
equals the production saturating result. The ensures clause reads:

```
forall |i: int| 0 <= i < 12 ==>
    spec_loop_mul_field_at(body, iterations, i) ==
        if spec_loop_mul_field_at(body, iterations, i) <= u64_max_int() {
            spec_loop_mul_field_at(body, iterations, i)
        } else {
            u64_max_int()
        },
```

This is a tautology — it says: a value equals itself when below the bound,
else equals the bound. The real question — does production
`saturating_mul` produce this value? — is unanswered.

## 2. Anti-Laundering Mandate

Per the planner skill's ANTI-VERIFICATION LAUNDERING MANDATE:

> The plan MUST EXPLICITLY FORBID the use of `#[verifier::external_body]`,
> `assume()`, or `axiom`.

The new bridge therefore:
- Writes an `exec fn exec_sat_loop_mul` whose body literally calls
  `u64::saturating_mul` on each field — matching production line-for-line.
- Writes `exec fn exec_sat_mul_u64(a: u64, b: u64) -> u64` whose body is
  `a.saturating_mul(b)` and whose ensures is non-trivial.
- Strengthens the lemma to use `assert(exec_sat_mul_u64(a, b) <= u64::MAX)`
  and a case-split on `(a as int) * (b as int) <= u64_max_int()`.

## 3. Lane Selections

| Lane | Required? | Rationale |
|---|---|---|
| Verus (L4) | YES | Saturating arithmetic must be reasoned about symbolically across the u64 boundary. |
| Kani (L3) | YES | Boundary inputs (u64::MAX, 0, 1) are exactly the cases where the saturation contract matters; Kani can enumerate. |
| proptest (L1) | YES | Random u64 pairs exercise the saturation contract across the full u64 range. |
| Flux | NO | Spec is non-linear (`a*b > bound`) — Flux's linear refinement types cannot express this. |
| Loom | NO | Synchronous single-threaded code. |
| cargo-fuzz | NO | No parser or codec. |
| TLA+ | NO | No temporal behavior. |

## 4. Risk Tags (from seed)

- `saturating-arithmetic`: u64::MAX clamp semantics must be proven.
- `production-binding`: spec must equal production's saturating_mul.
- `u64-overflow`: the predicate `(a*b) <= u64::MAX` is the failure boundary.

## 5. Execution Order

1. Add `exec_sat_mul_u64(a: u64, b: u64) -> u64` to spec.rs, body =
   `a.saturating_mul(b)`, ensures = `result <= u64::MAX && (result ==
   (a as int) * (b as int) || result == u64::MAX)`.
2. Add `exec_sat_loop_mul(body: Budget, iterations: u64) -> Budget` whose
   body is 12 lines of `field.saturating_mul(iterations)`.
3. Strengthen `lemma_loop_mul_saturated_eq_production` to invoke the exec
   fn and prove the equality by case-split on the bound predicate.
4. Write Kani harness with u64::MAX, 0, 1, under-bound, over-bound inputs.
5. Write proptest property for random u64 pairs.
6. Run all gates, update bead.

## 6. Out of Scope

- Modifying production `Budget::loop_mul` body. The proof obligation is to
  bind to existing production, not redesign.
- Other proof kernels (step_state, taint, envelope_header, vb_kyyf, profile)
  which have their own dual-mode registration entries.