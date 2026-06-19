# Trusted Base Plan — vb-h39ky

## Trusted Surfaces

| Surface | Kind | Trust Argument | Follow-up |
|---|---|---|---|
| `proof_obligations.yaml` | YAML registry | Validated by `python3 -c "import yaml; yaml.safe_load(...)"`; existing entries already load. | none |
| `rg` (ripgrep) | search tool | Standard CLI; output is deterministic for fixed input. | none |
| Existing `verus_registry_targets` RETIRED notes | editorial | Manual decisions from vb-dzibx proof-writer repair. | None — already trusted. |

## Assumptions

- The 329-block count from bead close reason is authoritative.
- The 33 explicitly-registered count is authoritative.
- The 296 in-crate `#[cfg(verus)]` count is authoritative.

## Stubs / Model Reductions

None. This is registry work.

## External Body / Trusted Proxies

Not applicable (no Verus artifacts).

## Bound / Range Constraints

- Each group decision must reference at least one production file path or
  cite an existing RETIRED note.