# Scope

Seven missing `WorkflowParts.input_slots` declarations across three CLI test fixture files: `crates/velvet_ballistics/tests/cli_integration.rs`, `admission_evidence_integration/chunk_001.rs`, and `chunk_004.rs`. One fixture requires a declared slot; six require explicit empty declarations.

# Domain

Test fixture correctness only. No production code, no API invention, no proof planning. The CLI binary already accepts positional I64 payloads; fixtures must declare matching input slots to satisfy the workflow type system.

# Behavior

`cli-input` fixture declares `slot0` as I64 for the external positional payload. The CLI minimal helper, admission `SetConst42->Finish`, direct `Do7`, direct taint expression, and both invalid resource fixtures declare empty `input_slots` arrays since they expose no CLI external ports. All existing assertions, negative resource/admission outcomes, and CLI binary input success paths remain unchanged.

# Errors

Fixtures that omit required slot declarations fail workflow type checking. Fixtures that invent slots for internal-only ports break admission or resource validation. Incorrect slot types cause I64 payload binding failures.

# Acceptance

All existing test assertions pass unchanged. Negative resource and admission behaviors remain exact. CLI binary input succeeds with declared I64 slot. Tests not yet run; this contract precedes implementation.