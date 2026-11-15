# Contract: forward existing Section 17 runtime codes

## Scope

Repair runtime-code reporting for the existing shared input-kind mismatch producer and for wrapped `CoreError` values. Keep the public enums and method signatures, journal formats, diagnostics, symbolic identities, display strings, and payloads unchanged. Add no dependency, runtime-code label, diagnostic code, or persisted field.

## Domain facts

- `RuntimeInputSlotKindMismatch` is emitted by `prepare_input_slots` for actual seed and queued input validation. At the runtime boundary its Section 17 runtime code is `INPUT_TYPE_MISMATCH`, already defined by `CoreError::INPUT_TYPE_MISMATCH_RUNTIME_CODE`.
- Runtime string codes and numeric/symbolic diagnostic identities are distinct. The mismatch remains numeric diagnostic `0x2005` with its current symbolic registry identity and display/payload.
- A wrapped `CoreError` retains the existing classification supplied by `CoreError::runtime_code()`: forward `Some(code)` unchanged, including `QUEUE_FULL`; use runtime `STORAGE_ERROR` only when that method returns `None`.
- `RollbackFailed` continues to report its primary error's runtime code.

## Observable behavior

- Seed and queued input kind mismatches return the existing mismatch error with unchanged slot, expected kind, actual kind, display, numeric diagnostic, symbolic identity, and no partial submission effects (no queued command, new run/frame, submission counter increment, or journal write).
- `RuntimeError::runtime_code()` returns the Section 17 label for that mismatch.
- `RuntimeError::Core` forwards every existing non-`None` CoreError runtime code; an unmapped CoreError retains the existing `STORAGE_ERROR` fallback.
- Diagnostic-code and symbolic-code results are unchanged for every tested case.

## Boundaries and hazards

- Reuse the existing shared producer and existing CoreError mapping; do not add variants or infer labels for other currently unmapped runtime errors.
- Tests exercise actual input ingress and wrapped error behavior, and assert both string runtime codes and unchanged numeric/symbolic identities and state effects.
- Main risks are accidentally replacing numeric diagnostics with string-code logic, changing fallback behavior, or allowing a mismatched input to mutate queued/run state before refusal.
