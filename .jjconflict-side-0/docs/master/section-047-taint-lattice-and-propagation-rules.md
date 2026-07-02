---
section: 47
title: "Taint Lattice and Propagation Rules"
parent: velvet-ballistics-MASTER.md
---

## 47. Taint Lattice and Propagation Rules


### Lattice Ordering

```text
Clean < DerivedFromSecret < Secret
```

`join_taint` returns the input unchanged. The lattice ordering is enforced by the propagation rules below: Secret never downgrades to Clean, and Clean never upgrades without explicit action.

### Propagation by Operation

| Operation | Taint behavior |
|-----------|---------------|
| `SetConst` | Always `Clean` — constants are compile-time values with no secret origin |
| `Copy` | Preserves source taint — `write_slot_with_taint(output, value, source_taint)` |
| `EvalExpr` | Output taint is the join of expression operand slot taints. |
| `BuildObject` | Output taint is the join of field slot taints. |
| `BuildList` | Output taint is the join of item slot taints. |
| `Do` (DeterministicPure) | Output ≥ input. `TaintViolation` if input is not Clean. Clean input → Clean output. |
| `Do` (IdempotentExternal) | Same propagation as DeterministicPure |
| `Do` (AtLeastOnceExternal) | Secret input → `DerivedFromSecret` output. `DerivedFromSecret` input → `DerivedFromSecret`. Clean input → Clean. |
| `Choose` / `ChooseSlot` | No taint tracking on branch conditions |
| `Finish` | Result taint passed through. No rejection of Secret or DerivedFromSecret results. |

### Control-Flow Taint

v1 does **not** track control-flow taint. A secret value can choose which public value is returned without triggering a taint violation. Example:

```yaml
choose:
  - if: "$secrets.token == 'x'"
    then: return_a
  - otherwise: return_b
```

Both `return_a` and `return_b` are Clean regardless of `$secrets.token` taint. This is an explicit v1 design decision. If control-flow taint is needed, it must be added as a v2 feature with a dedicated bead and evidence.

### Secret Storage

Secrets are referenced by `SymbolId` at runtime. The runtime never holds raw secret values — only taint markers. Secret values are resolved at compile time and stored as taint flags on the corresponding input slots.

---
