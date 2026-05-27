# Hazard Analysis — vb-om21

| Hazard | Risk | Contract mitigation | Proof seed refs |
|---|---|---|---|
| Prefix crossing | Tail scan reads lexicographically adjacent run's event and inflates tail. | Terminate/exclude on `!key.starts_with(run_prefix)`. | `ps-vb-om21-prefix-bound` |
| Stale metadata truncation | Declared tail below final durable key causes recovery to ignore committed events. | `TailMismatch` fail closed before replay. | `ps-vb-om21-tail-mismatch` |
| Missing journal ambiguity | Empty query and recovery-required absence collapse to broad/no-data behavior. | Distinguish zero-tail query from `MissingJournal` recovery failure. | `ps-vb-om21-missing-journal` |
| Numeric overflow | Final committed key at `u64::MAX` makes next tail overflow. | Use checked arithmetic and typed overflow error. | `ps-vb-om21-tail-overflow` |
| Key parse panic | Short/malformed storage key causes slicing/indexing panic. | Validate key length before extracting bytes; no unchecked indexing/slicing. | `ps-vb-om21-key-parse` |
| Payload/key divergence | Key seq says one value while decoded event seq differs. | Existing replay validation (`SequenceGap`, `WrongRun`) remains mandatory after tail scan. | `ps-vb-om21-replay-parity` |
| Unbounded scan/resource blowup | Tail computation collects every journal event or scans unrelated ranges. | O(1) max fold over prefix-bounded iterator; no event Vec for tail query. | `ps-vb-om21-bounded-scan` |
| Contract drift from existing errors | Acceptance names typed errors absent in code; implementation maps to generic strings. | Add/bridge structured `TailMismatch` and `MissingJournal` semantics. | `ps-vb-om21-typed-errors` |

## Residual Illegal-State Risks

- Current code has no inspected tail metadata record/API; implementation must choose where metadata enters without using boolean flags or `Option<u64>` lifecycle ambiguity.
- Current `RecoveryError` lacks `TailMismatch` and `MissingJournal`; broad `NoRecoveryData` remains a drift risk until typed variants are added or semantically wrapped.
- Current `events_for_run_from` collects replay events; a tail query should not reuse it if doing so makes tail scan resource usage proportional to payload decode and replay collection.
