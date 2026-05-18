# Architectural Drift Review — vb-qi37.16.4

STATUS: APPROVED

## Scope

- State: 13 architectural drift + Scott DDD review only.
- Workspace: `/home/lewis/src/Velvet-ballistics-vb-qi37-16-4-go`.
- Forbidden checkout not read/written/touched: `/home/lewis/src/Velvet-ballistics`.
- No source code changes made; downstream Go-skill states were not run.
- Canonical skill files read before review:
  - `/home/lewis/.claude/skills/architectural-drift/SKILL.md` lines 8-15 require line counts, DDD/primitive-obsession review, explicit state transitions, and parse-don't-validate checks.
  - `/home/lewis/.agents/skills/architectural-drift/SKILL.md` lines 8-15 contain the same rules; no conflict, and the `.agents` copy is authoritative if conflict appears.

## Inputs Reviewed

- `.beads/vb-qi37.16.4/delivery-scope.jsonl`
- `.beads/vb-qi37.16.4/implementation.md`
- `.beads/vb-qi37.16.4/formal-verification-report.md`
- `.beads/vb-qi37.16.4/verification-ledger.jsonl`

## Rust Files Reviewed

Delivery-scope expected files plus implementation-recorded answer IPC files were reviewed:

| File | Lines | Drift result |
|---|---:|---|
| `crates/velvet_ballastics/src/args.rs` | 1697 | Residual oversized file; answer command uses typed `Command::Answer` path. |
| `crates/velvet_ballastics/src/storage.rs` | 295 | Within 300-line limit. |
| `crates/velvet_ballastics/tests/cli_integration.rs` | 2552 | Residual oversized test file. |
| `crates/vb_runtime/src/shard/lifecycle.rs` | 2751 | Residual oversized file; reviewed `handle_ask_answer`. |
| `crates/vb_runtime/src/shard/types.rs` | 298 | Within 300-line limit; `AskTicket`/`AskAnswer` typed boundary present. |
| `crates/vb_runtime/src/journal.rs` | 1191 | Residual oversized file; `RuntimeJournalEvent::AskAnswered`/`SlotWritten` typed events present. |
| `crates/vb_runtime/src/trace.rs` | 1039 | Residual oversized file; typed trace event model retained. |
| `crates/velvet_ballastics/src/main.rs` | 4281 | Residual oversized CLI file; reviewed `cmd_answer`. |
| `crates/vb_ipc/src/server/handlers.rs` | 2122 | Residual oversized IPC handler file; reviewed `handle_answer_ask`. |

## Scott DDD / Drift Findings

- Primitive obsession: bead-local answer flow uses domain newtypes at the boundary after parsing/conversion: `RunId`, `StepIdx`, `SlotIdx`, `AskTicket`, `AskAnswer`, `Taint`, and `RuntimeJournalEvent`. The CLI accepts string/path wire inputs, then parses into typed runtime/IPCs boundaries.
- Parse-don't-validate: `cmd_answer` parses `run_id` before use; IPC `handle_answer_ask` decodes `IpcPayload::AnswerAsk`, converts ticket through `step_from_ticket`, bounds-checks answer bytes before `u32::try_from`, and decodes `SlotValue` before constructing `AskAnswer`.
- Explicit state transitions: `Shard::handle_ask_answer` is the explicit transition for external answers. It checks secret-result policy, checks encoded payload size against the resource contract, clears matching pending ask timer, writes the answer slot with taint, marks the ask step running then succeeded, sets the program counter to the resume step, appends `SlotWritten` before `AskAnswered`, emits trace, appends `StepSucceeded`, then drives the run.
- Illegal states unrepresentable: the answer path is carried by `AskTicket` and `AskAnswer` rather than loose parameter lists; taint and encoded length are carried with the value. IPC wire primitives are converted before runtime entry.
- Forbidden constructs in reviewed bead-local implementation slices: no `unsafe`, `.unwrap()`, `.expect()`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, lossy `as` casts, unchecked indexing, or unchecked arithmetic observed in `cmd_answer`, `handle_answer_ask`, or `handle_ask_answer` reviewed sections.

## Residual Drift Not Hidden

The touched/near-touched workspace still contains large pre-existing Rust files over the architectural-drift skill's 300-line target: `args.rs`, `cli_integration.rs`, `lifecycle.rs`, `journal.rs`, `trace.rs`, `main.rs`, and `vb_ipc/src/server/handlers.rs`. Splitting those files would be a broad structural refactor outside this State 13 review-only scope and would require rerunning from the appropriate Go-skill gate. No bead-local answer-command drift requiring immediate code change was found.

## Decision

Approved for State 13 architectural drift review. No code changes were necessary for the vb-qi37.16.4 answer-command implementation, so no downstream states were run.
