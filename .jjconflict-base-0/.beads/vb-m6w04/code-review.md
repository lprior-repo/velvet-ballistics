# Code Review: vb-m6w04

**Verdict: APPROVED**

**Frozen SHA:** 531f72e6fa6dcc20562938097e110ac4c57faaee
**Baseline SHA:** 11e4146f9af840f92e14a1ac62104140808ea30e
**Reviewer:** LOCAL RTX3090 Qwen3.8-27B via OMP (independent, did not author source)
**Date:** 2026-10-21

## Changes Reviewed

1. `crates/vb_compile/tests/v1_primitive_lowering.rs` — Test oracle corrections
2. `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs` — Formatting only

## Issue A: Together Primitive Constant Indices

**Claim verified: stale oracle.**

The together test case has two branches with distinct constant values (`value: "1"` and `value: "2"`). These must map to different entries in the constant table. The old test oracle incorrectly expected both SetConst nodes to reference const index 0:

```rust
// OLD (incorrect): both expected index 0
assert_set_const_node(nodes, &[], 2, Some(1), None, 0, 1)?;
assert_set_const_node(nodes, &[], 4, Some(3), None, 0, 2)?;

// NEW (correct): first index 0, second index 1
assert_set_const_node(nodes, &[], 2, Some(1), None, 0, 1)?;
assert_set_const_node(nodes, &[], 4, Some(3), None, 1, 2)?;
```

The `assert_set_const_node` helper (line 2000) validates both the constant index AND the actual value at that index. Expecting both nodes to have index 0 would mean the same value for both branches, which contradicts the test source (values "1" and "2"). The fix is correct.

## Issue B: Top-Level `inputs` Declaration

**Claim verified: stale oracle.**

The master specification (velvet-ballistics-MASTER.md line 298) explicitly lists `inputs` as a valid optional top-level field:

```yaml
Optional top-level fields:
inputs:   # input schema declarations
vars:     # static non-secret constants
secrets:  # named secret requirements
result:   # final output mapping
examples: # executable test fixtures
```

The removed test expected `inputs:` to produce `UnsupportedTopLevelDeclaration("inputs")`. The compiler correctly accepts it.

**Verification of proper acceptance tests elsewhere:** The `UnsupportedTopLevelDeclaration` error variant is still used for `vars`, `secrets`, and `examples` (see `crates/vb_compile/src/mod_compile_validation/part_01.rs` lines 67-73). These are still tested by remaining taxonomy cases. No invalid-input error handling was silently removed.

## Formatting Changes (part_05_digest.rs)

**Verified: semantics-neutral.**

The changes are purely rustfmt line-wrapping and spacing adjustments:
- `u32::try_from(value.len()).map_err(...)` split across lines
- Error message strings reformatted

All semantic elements preserved:
- Fail-closed typed errors (`digest_error`)
- Collision framing (tagged writes with TAG_STRING)
- Normalized defaults (length prefixing, little-endian)
- Overflow handling (try_from with u32 range errors)

## Test Results

| Test Suite | Count | Result |
|------------|-------|--------|
| v1_primitive_lowering | 47 | All pass |
| vb_compile lib | 496 | All pass |
| digest_framing_collision | 9 | All pass |
| rustfmt (touched files) | 2 files | Clean |

## Evidence Artifacts

- `.beads/vb-m6w04/evidence/primitive.*` — Primitive lowering tests
- `.beads/vb-m6w04/evidence/lib.*` — Lib tests
- `.beads/vb-m6w04/evidence/collision.*` — Digest collision tests
- `.beads/vb-m6w04/evidence/fmt2.*` — Rustfmt check on touched files

## Unverified Limits

- Kani formal verification not run (parent owns final CI)
- Fuzz testing not run
- Full moon ci pipeline not run
- Broader repo rustfmt issues (outside this bead's two-file delta) not addressed

## Conclusion

Both test oracle corrections align the tests with the documented master specification. The compiler behavior was already correct; the tests were stale. The formatting changes are semantics-neutral. No production code was modified. Approved for coordinator publication.