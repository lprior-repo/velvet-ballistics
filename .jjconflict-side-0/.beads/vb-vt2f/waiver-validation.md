# Waiver Validation — vb-vt2f State 11 formal-verification attempt 2

STATUS: REJECTED

- `WAIVER-TLA-VT2F-001`: FAIL_LOCAL. Waiver was valid only while no runtime/shard/admission lifecycle semantics changed. State 10 implementation changed runtime submit/admission behavior, ask/action routing, shard commands, and strict/journaled accepted-artifact store construction.
- `WAIVER-TLA-VT2F-002`: FAIL_LOCAL. Waiver was valid only while strict admission behavior was not edited. State 10 implemented strict direct submit accepted-artifact rejection and adjusted strict/journaled shard/runtime store construction.
- `WAIVER-VERUS-VT2F-001`: FAIL_LOCAL. Waiver assumed no pure/core/runtime transition logic changes. State 10 changed runtime/shard transition behavior (`fail_action`, ask prompt handling, `RuntimeActionFailed`, ticket/run error mapping, accepted-artifact store construction).
- `WAIVER-LEAN-VT2F-001`: WAIVED. No theorem kernel, algebraic/refinement kernel, Lean/Aeneas/Hax target, or theorem-owned clause was introduced.

Required route: proof replan/writer must either add executable TLA+/Verus obligations for the changed runtime/admission semantics or produce new explicitly approved waivers scoped to the final State 10 production changes. Passing `cargo nextest` and `moon ci` do not resurrect expired proof waivers.
