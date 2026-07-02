# Baseline Report: vb-zioy

**Bead:** vb-zioy — fix: enforce body.len() == 1 in collect body lowering (vb-xi2f.23)
**Source checkout:** /home/lewis/src/velvet-ballistics
**Baseline commit:** main (stashed working changes)

## Git Status
- Branch: main
- Modified files stashed for clean baseline
- 1 stashed change: crates/vb_compile/tests/v1_primitive_lowering.rs (45 lines added — multi-step body rejection test)

## Compilation State
- `cargo check -p vb_compile`: PASS
- `cargo test -p vb_compile --no-run`: PASS (tests compile)
- Workspace lints active: unsafe_code=forbid, unwrap_used=deny, expect_used=deny, panic=deny, todo=deny, unimplemented=deny, dbg_macro=deny, indexing_slicing=deny, string_slice=deny, get_unwrap=deny, arithmetic_side_effects=deny, as_conversions=deny

## Scope Context
Parent issue vb-xi2f.23 (CLOSED): "lower nested collect body steps"
This bead: enforce body.len() == 1 in collect body lowering specifically.

The working tree already contained a test for multi-step body rejection in scoped primitives (repeat, for_each, collect, reduce). This bead focuses on ensuring collect specifically enforces the single-step body invariant.
