# Trusted Base Plan — vb-puvkn

## Trusted Surfaces

| Surface | Kind | Trust Argument | Follow-up |
|---|---|---|---|
| `Runtime::shard_index` production method | u64-modulo function | Implementation uses `checked_rem(0).unwrap_or(0)`; covered by Verus exec fn + Kani arbitrary. | none |
| `RunId::shard_index` const fn | u64-modulo const fn | Implementation in vb_core/src/ids/mod.rs:350 is `self.0 % shard_count`. | none |
| `u64::checked_rem` | std library primitive | Documented in std as `Some(a % b)` if b != 0, else `None`. | none |
| Spec_shard_index case split | Verus spec | Predicate `shard_count == 0` is decidable; `run_id % shard_count` is decidable when `shard_count > 0`. | SMT trustworthiness |

## Assumptions

- `Runtime::shard_index` body remains `run_id.checked_rem(shard_count).unwrap_or(0)`.
- The fallback `unwrap_or(0)` matches the spec's zero branch.
- `vb_core::RunId` newtype remains a thin wrapper around u64.

## Stubs / Model Reductions

None. The exec fn body must literally mirror the production method body.

## External Body / Trusted Proxies

The production annotation option (a) uses `#[verifier::external]` to
declare the external function signature, plus an `extern_spec` block in
runtime_facade_api.rs. The exec body is NOT external_body — only the
production method's body remains opaque. The ensures clause on the
extern_spec IS the binding. This is the standard Verus pattern for
binding to opaque production code.

If option (b) is chosen instead (extract a shared helper), the helper
body is fully visible to Verus and no extern_spec is needed.

## Bound / Range Constraints

- `shard_count == 0` → result is `0` (production fallback).
- `shard_count > 0` → result is `run_id % shard_count < shard_count`.