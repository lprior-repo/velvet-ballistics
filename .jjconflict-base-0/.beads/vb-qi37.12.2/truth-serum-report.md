# Truth Serum Report — vb-qi37.12.2

STATUS: APPROVED

## Audit

- Claim checked: resume journal/storage failures are not silently converted to success.
- Evidence: focused resume error propagation test suite passed 12/12 and machine-gate artifact records prior State 11 approval.
- Claim checked: failed `Resumed` append restores `RuntimeState::Resumable`.
- Evidence: focused suite plus `vb_runtime --lib is_resumable` passed.
- Claim checked: no stale ambient source theft remains.
- Evidence: source scan found no registry/thread-local carrier; current enum has explicit `JournalAppendFailedWithSource` plus deterministic unit fallback.
- Claim checked: formal waiver is explicit rather than hidden.
- Evidence: `formal-waivers.jsonl` validates and names the waived TLA obligation, owner, reason, limitation, compensating evidence, and expiry trigger.

## Verdict

No laundered evidence found for the narrowed contract. The old black-hat rejection applied to a superseded source-registry design, not the current source-carrying implementation.
