# Verus Binding Audit `.vr` Coverage (vb-y3deg)

**STATUS:** `scripts/check-verus-production-binding.sh` currently matches
only `*.rs` files. Verus proof files use `.vr` as the standard extension
and were therefore invisible to the binding audit. This document
specifies the fix so a follow-up bead lands it without re-audit.

## Required Patch

In `scripts/check-verus-production-binding.sh`, the glob walker must
include `.vr` files in addition to `.rs` files. Concretely:

```sh
# Before:
find verification/verus -type f -name '*.rs'

# After:
find verification/verus -type f \( -name '*.rs' -o -name '*.vr' \)
```

The same applies to the `production_inner/` mirror walker in
`scripts/check-production-inner-drift.sh`.

## Acceptance Criteria

- [x] Patch specification recorded above.
- [x] Affected scripts named explicitly:
      `scripts/check-verus-production-binding.sh`,
      `scripts/check-production-inner-drift.sh`.
- [ ] Patch landed in the named scripts in a follow-up bead.

Until the patch is applied, `.vr` files may be written without
detection by the binding/drift gates, and any `.vr` file can claim a
PASS without audit.