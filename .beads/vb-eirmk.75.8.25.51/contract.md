# Contract: reject unrepresentable timer durations

Bead: `vb-eirmk.75.8.25.51`

## Invariant

A positive finite runtime timeout that cannot be represented both as a bounded millisecond duration and as a host `Instant` is rejected with an existing typed resource/duration error. It must never become an immediate, maximum, or otherwise shortened timer. Operand validation happens before the affected primitive increments/suspends its step or appends a scheduling event.

## Compatibility

Preserve zero and existing negative-as-immediate behavior, ordinary I64/F64 finite and fractional millisecond conversion, untimed Ask, timer event schema, taint/type/error precedence, current authority, and admission/source/checkpoint bytes. Use existing `EngineError`/`CoreError::ResourceLimitExceeded` or an existing accurate duration error. Add no public API/error, persisted format, dependency, or gate change.

## Shared paths

Use one private runtime conversion/check for host timeout and `Instant` representability at WaitUntil, WaitEvent timeout, Ask timeout, live timer construction, evidence-empty timer application where applicable, and cold timer restoration. Cold refusal occurs before installing timer/authority or mutating replay state. Preserve current classified recovery errors.

## Acceptance

- Preserve the real strict/public IPC finite-F64 `1e100` fixture and make it reject before timeout/suspension effects with the exact typed error.
- Preserve ordinary 600000ms Wait/Ask, zero, negative, and untimed Ask controls.
- Verify same IPC service remains healthy after invalid duration.
- Use genuine current-issued strict Fjall cold prefixes and compiler-bound Symbol inputs; do not fabricate Symbol IDs, artifacts, tickets, or checkpoints.
- Verify cold unsupported duration refuses before timer/authority/prefix mutation and ordinary cold recovery remains valid.
- Keep new functions <=25 raw lines, <=5 parameters, files <=300 lines; run affected package tests, source-length, format and Clippy checks.
