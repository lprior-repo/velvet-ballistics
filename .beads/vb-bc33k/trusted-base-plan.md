# Trusted Base Plan — vb-bc33k

## Trusted Surfaces

The following inputs/dependencies are trusted at the type_enforcer layer.
Each must be either already proven by another obligation or carry its own
proof obligation.

| Surface | Kind | Trust Argument | Follow-up |
|---|---|---|---|
| `vb_core::SlotValue` enum definition | type definition | Tagged `#[non_exhaustive]` closed enum with 8 named variants; no unsafe; covered by `proof_seed_id=vb-core-slotvalue-def` (already registered as obl-vb-core-slotvalue-enum-001 in proof_obligations.yaml). | none |
| `vb_core::ids::{SymbolId, ListId, ObjectId}` | newtype wrappers | Newtype pattern around u64/u32; no behavior. | none |
| `crate::ExprResult` and `crate::ExprError` | Result/Error | Existing obligations in vb_expr cover ExprError classification. | cross-reference obl-vb-expr-error-* |
| `Spec/type_name()` method on SlotValue | trait method | Implementation is in vb_core and covered by `obl-vb-core-slotvalue-type-name-001`. | none |

## Assumptions

- `SlotValue` is a closed enum with exactly 8 variants: `Bool`, `I64`, `F64`,
  `Symbol`, `List`, `Object`, `Blob`, `Null`. Adding a new variant would
  invalidate the partition lemma.
- `ExprError::TypeMismatch` is the only error variant returned by
  type_enforcers. Other variants (e.g., EvalPanic, Missing) are not produced
  here.

## Stubs / Model Reductions

None. The plan mandates `exec_expect_*` to use the **same** match arms as
production. No model reduction is acceptable at this layer.

## External Body / Trusted Proxies

Explicitly forbidden. The anti-laundering mandate applies; see
proof-strategy.md §2.

## Bound / Range Constraints

None at this layer — the type enforcers operate on the full SlotValue type
without additional bounds.