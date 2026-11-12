# IPC and CLI Defects

## P0: Live IPC server buffers before validating magic

Evidence:

- Subagent inspection found the live server holds per-client read buffers as heap `Vec`s, appends socket bytes, and only later validates frame headers.
- The standalone frame decoder does validate magic and payload bounds, but the live socket path allocates/grows before that decoder runs.

Master violated:

- Section 21: validate magic before allocation; validate payload length before reading payload.
- Section 44 point 16 and point 21.

Impact: An adversarial client can force read-buffer allocation/growth before magic rejection.

Suggested bead: `P0 fix IPC server to validate magic before read-buffer allocation growth`

## P1: IPC command set drifted from exact 11-command master list

Evidence:

- Subagent inspection found `crates/vb_ipc/src/commands.rs` defines the required 11 plus extra `ListRuns`, `GetMetrics`, `GetWorkflowGraph`, `GetTaintReport`, and `VerifyWorkflow`.

Master violated:

- Section 21 required IPC commands.
- Section 31 command handler surface.
- Section 44 point 16.

Impact: Protocol surface is larger than the authoritative v1 contract unless the master is updated.

Suggested bead: `P1 reconcile IPC v1 command set with master 11-command contract`

## P1: CLI `action inspect` takes numeric action id, not action name

Evidence:

- Subagent inspection found CLI parser stores `action_id: u16` and help says `<action_id>`.

Master violated:

- Section 33: `velvet-ballistics action inspect <action-name> --emit yaml`.

Impact: Operator contract mismatch.

Suggested bead: `P1 align CLI action inspect with action-name contract`

## P1: CLI command surface exceeds Section 33 without reconciliation

Evidence:

- Subagent inspection found parser/help accepting extra commands such as `verify`, `explain`, `trace`, `retry`, `resume`, `answer`, `diff`, `submit`, `simulate`, `cancel`, and top-level `status`.

Master violated:

- Section 33 if treated as exact command surface.
- Meta-review warned later master sections also extend CLI, so this is a doc/implementation reconciliation defect rather than automatically a code defect.

Impact: Agents cannot tell which CLI section is normative without a merged command contract.

Suggested bead: `P1 reconcile CLI command surface across master sections 33 69 70 75`

## P1: CLI `--emit postcard` wraps JSON UTF-8 instead of typed payloads

Evidence:

- Subagent inspection found `CliPostcardContentType::JsonUtf8`, JSON bytes serialized into a Postcard wrapper, and decoder returning `serde_json::Value`.

Master violated:

- Section 33: `--emit postcard` is canonical binary machine-output flag where supported.

Impact: Binary output is JSON-in-Postcard, not a typed domain payload contract.

Suggested bead: `P1 replace CLI postcard JSON wrapper with typed postcard output envelopes`
