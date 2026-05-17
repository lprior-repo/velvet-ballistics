# Regression Diff - vb-0253.1

## Bead-Local Changes
- Added shared capacity predicate and switched both queue/config constructors to it.
- Added Kani capacity harness and one boundary test.

## Regression Classification
- No bead-local compile/test/proof regressions found.
- Workspace `cargo fmt --check` failure predates this bead scope and is classified `DEFERRED_GLOBAL`.
