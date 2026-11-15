# Contract: reject legacy answer CLI selectors

## Intent

The public `answer` CLI accepts only the current slot selector: `--slot <slot-id> --value <file>`. The old `--step` / `--value-file` pair must fail during argument validation before reading a value file or sending an IPC request. This supersedes the `.48` compatibility clause by explicit user direction; the historical `.48` contract remains unchanged.

## Invariants

- A valid `--slot` / `--value` answer follows the existing pending-Ask slot lookup, actual ticket authority, typed Postcard decode, taint/size checks, error ordering, output formats, and journal behavior.
- Either legacy flag is rejected as an unknown flag for `answer`; mixed, duplicate, missing, and malformed current selectors retain exact refusal behavior.
- Rejected legacy CLI requests do not read the answer file, contact IPC, enqueue work, or change run state, ticket, journal, trace, counters, or queue depth.
- The crate-private CLI selector retains only the Slot case and its checked wire-ticket offset. The `IpcPayload::AnswerAsk` layout, server decoding of both existing ticket ranges, and all persisted formats remain unchanged.
- `--step` remains accepted by its unrelated `run`, `retry`, and `trace` commands. Do not remove shared flags outside the `answer` command.
- Agent-context metadata and help advertise only the slot/value pair. The generated agent-context snapshot changes only for the answer summary and flags.

## Acceptance

- Exact parser tests reject `--step` and `--value-file` as `UnknownFlag { command: "answer", flag }` while preserving canonical slot success and all unrelated parser tests.
- The public CLI rejects the removed pair before IPC against a genuine pending Ask and preserves full before/after run, ticket, journal, trace, counter, and queue observations.
- The same Ask remains answerable through its actual slot. Existing wrong/stale slot refusals, output-format assertions, and wire ticket boundary tests remain intact.
- Agent-context metadata and snapshots match the canonical flags with no unrelated snapshot changes.
- No schema, dependency, IPC wire-format, persisted-format, admission, authority, or gate change.
