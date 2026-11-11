---
section: 47
title: "Taint Lattice and Propagation Rules"
parent: velvet-ballistics-MASTER.md
---

## 47. Taint Lattice and Propagation Rules

### Lattice Ordering

```text
Clean < DerivedFromSecret < Secret < Random < TimeDependent
```

`Taint` is a single enum marker. `join_taint(a, b)` returns the higher-ranked input; it does not preserve independent confidentiality and nondeterminism flags. Joining Clean with Secret preserves Secret; expression evaluation does not downgrade it to DerivedFromSecret. The propagation rules below apply this ordering to each operation.

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

v1 does **not** track control-flow taint. An admitted Boolean input named `secret_flag`, declared with `secret: true`, can choose which public value is returned without triggering a taint violation. Example:

```yaml
choose:
  - if: "$input.secret_flag"
    then: return_a
  - otherwise: return_b
```

Both `return_a` and `return_b` are Clean regardless of the condition's taint. This is an explicit v1 design decision. If control-flow taint is needed, it must be added as a v2 feature with dedicated evidence.

### Secret Storage

Entries in the YAML `secrets` mapping bind source aliases to host requirement names. Compilation retains those names and immutable Secret-tainted input roots carrying opaque `SymbolId` references. Admission checks every declared requirement against host-confirmed availability before accepting a run. Raw credential values remain outside the runtime core and compiled artifact; expressions over these references operate on opaque identities, not credential contents. Ordinary user inputs declared with `secret: true` retain their admitted payload and taint under the separate input-schema contract.

---
