# Trusted Base Plan - vb-dybj

This plan records expected assumptions, reductions, and trusted surfaces. It is not an approval ledger.

| Trusted ref | Related obligation(s) | Trusted kind | Reason | Scope | Impact | Behavior-affecting? | Planned control |
|---|---|---|---|---|---|---:|---|
| TB-VB-DYBJ-001 | PO-VB-DYBJ-001, PO-VB-DYBJ-004, PO-VB-DYBJ-007 | verifier harness boundary | Verus cannot directly verify all serde/Postcard internals or existing crate compilation without wrappers. | Wrapper specs around production-bound functions only. | Could become vacuous if production functions are not bound. | Require source refs and reject admit/axiom for behavior claims. |
| TB-VB-DYBJ-002 | PO-VB-DYBJ-002, PO-VB-DYBJ-010, PO-VB-DYBJ-013 | bounded model reduction | Kani requires finite unwind/object bounds and harness setup. | Bounded inputs for u64, header lengths, trailing suffixes. | Too-small bounds could miss failures. | Record exact Kani bounds and use arbitrary/generator inputs, not hardcoded-only shapes. |
| TB-VB-DYBJ-003 | PO-VB-DYBJ-005 | tooling limitation | Flux support for external crate annotations may require wrapper artifact. | Digest shape refinement only. | Could overtrust wrapper if disconnected. | Require production source mapping or blocked_tooling evidence from reviewer/formal verifier. |
| TB-VB-DYBJ-004 | PO-VB-DYBJ-007, PO-VB-DYBJ-008 | dependency serialization boundary | Postcard enum serialization algorithm is external dependency behavior. | Selected `RecordKind` enum bytes only. | Could confuse enum bytes with `id()` bytes. | Tests must freeze bytes and name `postcard_enum` vs `envelope_id_u16_le`. |
| TB-VB-DYBJ-005 | PO-VB-DYBJ-012, PO-VB-DYBJ-015 | fuzz smoke bound | Fuzz smoke is time/run bounded and not exhaustive. | 60 seconds / 10000 runs planned smoke. | Deep malformed cases may remain unexplored. | Preserve corpus seeds and allow deeper reruns if formal verifier escalates. |
| TB-VB-DYBJ-006 | PO-VB-DYBJ-016 | TLA+ model reduction | Fixture lifecycle model bounds fixture set and models migration name as presence flag. | Four representative fixture categories. | Does not prove all possible future fixtures. | Tie model constants to contract-selected surfaces and require non-vacuity invariant. |
| TB-VB-DYBJ-007 | PO-VB-DYBJ-018 | source-scan substitution | Planned command names a script that may not exist. | Forbidden-token scan over touched files. | Equivalent substitution may weaken coverage. | Formal verifier must record exact substitute command and token list if script absent. |

No behavior-affecting waiver is planned. Any `assume`, `admit`, `trusted`, `ignore`, disabled check, or model-bound expansion in downstream proof artifacts must become a trusted-base ledger row before closure.
