bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 3
updated_at: 2026-05-14T22:16:30Z
attempt: 1-of-7

# Domain Model Review

## Domain terms

- Public exit code: process status observable by shells/operators. Must be `0..=8`.
- Diagnostic envelope: machine-readable error payload with stable code/message and optional path/span/repair.
- Structured output: deterministic CLI envelope carrying `schema_version`, `kind`, and payload/diagnostic data.
- Postcard route: bounded binary encode/decode path used for structured operator/UI payloads.

## Illegal states currently representable

- `CliExitCode::DomainError = 9` makes an out-of-contract public exit status representable.
- Verus mirror `SpecCliExitCode::DomainError` and `spec_exit_code_in_range_0_to_9` encode the wrong contract.
- Fuzz registry lacks `vb_ui_model_postcard_decode`, so integrated postcard proof coverage may be absent even if lower-level postcard functions exist.

## Required model correction

- Remove public code `9` or remap domain-specific failures into an existing public error class.
- Update proof/test models so the type-level/public enum cannot silently reintroduce a tenth public exit code.
- Treat postcard decode evidence as part of the structured-output model, not as a disconnected standalone sample.

## DDD judgment

The exit code enum is the boundary type. The illegal state is in the boundary representation itself, so State 10 must repair production type/tests after State 5/6 and State 7/9 define and approve the target behavior.
