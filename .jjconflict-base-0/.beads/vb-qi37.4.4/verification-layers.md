# Verification Layers

## Boundary
- Changed API: `RuntimeError`, `diagnostic_code`, `runtime_code`, `Display`, `Error::source`, API/CLI/IPC envelopes.
- Verus waiver: `WAIVER-VERUS-DIAG-TOTALITY`; owner `Lewis`; expires before parent `vb-qi37.4` release closure; limitation: current diagnostic totality is embodied by Rust enum exhaustive matches rather than a pure spec module; compensating evidence: exhaustive unit tests over each variant and compile-time match exhaustiveness.

## Layer Assignment
- PRE-001 -> `ERR-ADM-CAUSES-001`.
- PRE-002 -> `ERR-ADM-PERSIST-001`.
- POST-001 -> `ERR-ADM-001` + `ERR-ADM-CAUSES-001`.
- POST-002 -> `API-ADM-001` + `TLA-ERR-001`.
- INV-001 -> `ERR-ADM-001` + `WAIVER-VERUS-DIAG-TOTALITY`.
- Error taxonomy -> `ERR-ADM-CAUSES-001`, `ERR-ADM-PERSIST-001`, `ERR-IDEMP-001`.
- Release gate -> `REL-GATE-004`.

## Waivers
- `WAIVER-VERUS-DIAG-TOTALITY`: Verus proof deferred until a pure diagnostic-code mapping module exists. Owner: Lewis. Expiry/follow-up: before `vb-qi37.4` release closure. Compensating evidence: exhaustive tests asserting exact variant/code/runtime-code/source behavior plus Rust exhaustive match compilation.
