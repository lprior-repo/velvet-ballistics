# vb-ko29.1 TLA+ idempotency evidence

## Result

PASS: TLC completed every bounded IdempotencySafety scenario with no invariant, temporal-property, type, or deadlock error.

## Tool

- Java: `openjdk 26.0.1 2026-04-21`
- TLC: `TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)`
- Jar: `/home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar`

## Bounds

- Standard scenarios: `MaxRuns = 1`, `MaxActions = 1`, `MaxSeq = 3`, `Digests = {0, 1}`, `NullDigest = 0`.
- Overflow scenario: `MaxSeq = 2` to force the reserved sequence-boundary fail-safe sooner.
- No symmetry reduction. Deadlock checking enabled in all cfgs.

## Commands and exit codes

All commands were run from `/home/lewis/src/velvet-ballistics`.

| Scenario | Command | Exit | Raw log | Classification |
|---|---|---:|---|---|
| TypeOK | `java -cp /home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar tlc2.TLC -config verification/tla/IdempotencySafetyTypeOK.cfg verification/tla/IdempotencySafety.tla` | 0 | `.evidence/vb-ko29.1/logs/IdempotencySafetyTypeOK.log` | PASS |
| Full package | `java -cp /home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar tlc2.TLC -config verification/tla/IdempotencySafety.cfg verification/tla/IdempotencySafety.tla` | 0 | `.evidence/vb-ko29.1/logs/IdempotencySafety.log` | PASS |
| Overflow | `java -cp /home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar tlc2.TLC -config verification/tla/IdempotencySafetyOverflow.cfg verification/tla/IdempotencySafety.tla` | 0 | `.evidence/vb-ko29.1/logs/IdempotencySafetyOverflow.log` | PASS |
| Terminal finality | `java -cp /home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar tlc2.TLC -config verification/tla/IdempotencySafetyTerminalFinality.cfg verification/tla/IdempotencySafety.tla` | 0 | `.evidence/vb-ko29.1/logs/IdempotencySafetyTerminalFinality.log` | PASS |
| Duplicate success | `java -cp /home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar tlc2.TLC -config verification/tla/IdempotencySafetyDuplicateSuccess.cfg verification/tla/IdempotencySafety.tla` | 0 | `.evidence/vb-ko29.1/logs/IdempotencySafetyDuplicateSuccess.log` | PASS |
| Duplicate failure | `java -cp /home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar tlc2.TLC -config verification/tla/IdempotencySafetyDuplicateFailure.cfg verification/tla/IdempotencySafety.tla` | 0 | `.evidence/vb-ko29.1/logs/IdempotencySafetyDuplicateFailure.log` | PASS |
| Divergent digest | `java -cp /home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar tlc2.TLC -config verification/tla/IdempotencySafetyDivergentDigest.cfg verification/tla/IdempotencySafety.tla` | 0 | `.evidence/vb-ko29.1/logs/IdempotencySafetyDivergentDigest.log` | PASS |
| Crash/recover duplicate | `java -cp /home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar tlc2.TLC -config verification/tla/IdempotencySafetyCrashRecoverDuplicate.cfg verification/tla/IdempotencySafety.tla` | 0 | `.evidence/vb-ko29.1/logs/IdempotencySafetyCrashRecoverDuplicate.log` | PASS |
| Retry collision | `java -cp /home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar tlc2.TLC -config verification/tla/IdempotencySafetyRetryCollision.cfg verification/tla/IdempotencySafety.tla` | 0 | `.evidence/vb-ko29.1/logs/IdempotencySafetyRetryCollision.log` | PASS |
| Stale tracker | `java -cp /home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar tlc2.TLC -config verification/tla/IdempotencySafetyStaleTracker.cfg verification/tla/IdempotencySafety.tla` | 0 | `.evidence/vb-ko29.1/logs/IdempotencySafetyStaleTracker.log` | PASS |

## Checked claims

- TypeOK declared and checked.
- Normal append transitions require `nextSeq < MaxSeq`; the boundary has explicit `SequenceOverflowFailSafe` transition to terminal `Failed` using the reserved `MaxSeq` slot.
- Exact terminal finality is checked by `TerminalStateFinality` and `TerminalExactStepFinality`.
- Duplicate success, duplicate failure, divergent digest, crash/recover duplicate, retry collision, and stale tracker scenarios check the relevant digest/resolution/recovery invariants.

## Changed files

- `verification/tla/IdempotencySafety.tla`
- `verification/tla/IdempotencySafety.cfg`
- `verification/tla/IdempotencySafetyTypeOK.cfg`
- `verification/tla/IdempotencySafetyOverflow.cfg`
- `verification/tla/IdempotencySafetyTerminalFinality.cfg`
- `verification/tla/IdempotencySafetyDuplicateSuccess.cfg`
- `verification/tla/IdempotencySafetyDuplicateFailure.cfg`
- `verification/tla/IdempotencySafetyDivergentDigest.cfg`
- `verification/tla/IdempotencySafetyCrashRecoverDuplicate.cfg`
- `verification/tla/IdempotencySafetyRetryCollision.cfg`
- `verification/tla/IdempotencySafetyStaleTracker.cfg`
- `.evidence/vb-ko29.1/logs/*.log`
- `.evidence/vb-ko29.1/tla-idempotency-report.md`

## Blockers

None for the bounded TLC package above.
