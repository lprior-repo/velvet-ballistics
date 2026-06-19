# Trusted Base Plan — vb-z280t

## Trusted Surfaces

| Surface | Kind | Trust Argument | Follow-up |
|---|---|---|---|
| `Budget` struct (cargo_kernel) | u64-field struct | 12 named u64 fields, no unsafe, no shared state. Already registered as `VB-PROOF-KERNELS-RESOURCE-BUDGET` (dual-mode kernel). | none |
| `u64::saturating_mul` | std library primitive | Documented in std as `a*b` if it fits in u64, else `u64::MAX`. Provenance: Rust std lib. | none |
| `u64::MAX` constant | literal | `18446744073709551615`. Verified by `verus!` constant. | none |
| `verus::int` for spec math | Verus builtin | Unbounded integer arithmetic; no overflow possible at spec level. | none |
| Spec_sat_mul_u64 case split | Verus spec | Predicate `(a as int) * (b as int) <= u64_max_int()` is decidable by SMT for u64. | SMT solver trustworthiness |

## Assumptions

- `Budget` fields remain exactly 12 in number. Adding a 13th field would
  invalidate the `0 <= i < 12` quantifier in the lemma.
- `u64::saturating_mul` matches the Verus-spec `spec_sat_mul_u64` predicate
  exactly. This is a stdlib contract trust assumption — not derived from
  Verus here. Follow-up: register an explicit `obl-vb-stdlib-sat-mul-verus-001`
  if stdlib saturation semantics ever change.

## Stubs / Model Reductions

- The exec fn body is allowed to use `u64::saturating_mul` directly (no
  extracted production helper) because the cargo_kernel uses the same
  primitive.

## External Body / Trusted Proxies

Explicitly forbidden by anti-laundering mandate. The exec fn must call
`saturating_mul` directly, not via an external body.

## Bound / Range Constraints

- `body_field_i * iterations <= u64::MAX` is the binding precondition for
  the no-overflow case.
- `body_field_i * iterations > u64::MAX` triggers the saturation clamp.