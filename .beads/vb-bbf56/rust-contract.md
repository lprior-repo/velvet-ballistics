# Rust Contract: Production-Inner Drift Repair

## Preserved Bindings

All production_inner mirrors must remain faithful verbatim copies of production slices. Repairs synchronize:

1. **Type variant sets**: Mirror enum variants must match production exactly (additions/deletions/renames).
2. **Type field sets**: Mirror struct/variant fields must match production exactly.
3. **Method signatures**: Mirror method signatures must match production.
4. **Binding ledgers**: Extern file ledger entries must point to current production locations.
5. **Identifier names**: All structural identifiers (types, variants, fields, functions, constants) must be present in mirrors.

## What is NOT Changed

- Mathematical contracts, bounds, or oracles
- Production source behavior or structure
- Verification harness logic
- Drift-checking script logic or thresholds

## Scope

Only introduced drift from the symbolic ticket fixture repair (d14ae733) and prior moves is fixed. Baseline failures pre-existing on origin/main (da633fe0) are documented but not repaired in this bead.