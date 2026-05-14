# Verification Layers — vb-oaom: cli: Add runtime ai context packet command

## Boundary

- **Verified kernel**: `commands_ai_context.rs` — pure parsing, journal traversal, taint-based redaction, packet assembly, and suggested-command derivation.
- **Runtime shell**: The `handle()` function is the CLI entrypoint; it is called from `main.rs` and propagates `ExitCode`. No live runtime shard is accessed.
- **External systems excluded from formal proof**: Fjall journal storage (treated as an oracle; corruption is out of scope for this command's contract).

## Layer Assignment

| Contract Clause | Verification Layer(s) |
|----------------|----------------------|
| PRE-001 (run_id parses as u64) | `proptest` (generated `&str` inputs) + `static-scan` (no unchecked `unwrap` on parse path) |
| PRE-002 (journal opens) | `manual-qa` (open valid path, open invalid path) + `static-scan` |
| PRE-003 (non-empty event trail) | `Fowler scenario` (given unknown run returns structured not-found diagnostic) |
| POST-001 (valid AiContextPacket JSON) | `cargo-nextest` unit tests (snapshot assertions on JSON shape) |
| POST-002 (workflow field populated) | `cargo-nextest` unit tests |
| POST-003 (secret redaction) | `proptest` (property: secret/derived slots always redact) + `kani` (bounded model: no unchecked slot access) |
| POST-004 (suggested commands are real) | `static-scan` (hardcoded command list in source) + `Fowler scenario` |
| POST-005 (action contracts inferred) | `cargo-nextest` unit tests |
| POST-006 (RUN_NOT_FOUND structured error) | `cargo-nextest` unit tests (given unknown run returns structured not-found diagnostic) |
| INV-001 (read-only) | `static-scan` (no mutable journal writes in `handle()`) |
| INV-002 (bounded packet size) | `static-scan` (fixed-length suggested_commands list, bounded journal_event_trail from Fjall) |
| INV-003 (all suggested commands are real) | `static-scan` + `manual-qa` (help text confirms commands exist) |
| INV-004 (UNDECODED for decode failure) | `proptest` (malformed SlotValue bytes) + `cargo-fuzz` (bolero corpus) |
| Error::InvalidRunId | `cargo-nextest` unit tests + `proptest` |
| Error::JournalOpen | `manual-qa` + `static-scan` |
| Error::RunNotFound | `cargo-nextest` unit tests |
| Error::JournalRead | `cargo-nextest` unit tests (mock journal errors) |

## Lean Scope

Not applicable — `ai-context` is a cold CLI command with no pure deterministic kernel amenable to Lean theorem proving. The redaction logic is a pure function (`redacted_slot_value`) but its proof obligation is covered by `proptest` (property-based testing of taint lattice propagation) and `kani` (bounded model checking of slot index access).

## Miri / Cargo-Careful

- `commands_ai_context.rs` contains no `unsafe`, no raw pointer manipulation, and no aliasing. Miri is not required for this cold CLI path.
- If Miri were run: `cargo +nightly miri test -p velvet_ballastics --test '*ai_context*'` — waived with reason: no unsafe code in the module under test; `commands_ai_context.rs` is `#![forbid(unsafe_code)]`.

## Proptest Scope

- `redacted_slot_value` property: for all `SlotIdx`, all `Option<Vec<u8>>`, all `Option<RunSnapshot>`, secret/derived slots always return `[REDACTED]`, clean slots return a non-redacted string.
- `suggested_ai_commands` property: output list length is bounded (≤4), all items start with `velvet-ballastics`.
- `parse_run_id` property: arbitrary decimal strings either parse as `u64` or return an error.

## Fuzzing / Bolero Scope

- Journal event → JSON serialization: feed arbitrary `JournalEvent` binary records through the event-to-JSON path.
- Malformed `SlotValue` bytes: generate arbitrary `Vec<u8>` payloads and verify `redacted_slot_value` never panics and never leaks raw secret content.

## Loom / Shuttle / Lockbud

Not applicable — the `ai-context` command is single-threaded, synchronous, and performs no concurrent access.

## Cargo-Mutants Scope

- Run `cargo mutants -p velvet_ballastics -- --test-threads=1` on the `commands_ai_context` module.
- Kill mutants that remove redaction logic (must be killed by the redaction test).
- Kill mutants that change the `suggested_ai_commands` list length or content.

## Cargo-LLVM-Cov Scope

- Run `cargo llvm-cov -p velvet_ballastics --html` on the `commands_ai_context` module.
- Require >90% line coverage for `commands_ai_context.rs`.

## Static Scan Scope

- `cargo clippy -p velvet_ballastics -- -D warnings` on the module.
- `cargo machete -p velvet_ballastics` to detect unused dependencies.
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg!` allowed.

## Manual QA Scope

- Invoke `velvet-ballastics ai-context <valid_run_id> --db <path> --json` and verify packet fields.
- Invoke `velvet-ballastics ai-context <invalid_run_id> --db <path> --json` and verify `RUN_NOT_FOUND` error.
- Invoke `velvet-ballastics ai-context <valid_run_id> --db <nonexistent_path> --json` and verify storage error.
- Invoke `velvet-ballastics ai-context <run_with_secrets> --db <path> --json` and verify redacted slots are `[REDACTED]`.

## Gauntlet Lanes

- `moon run :verify-fast` — sufficient for this CLI-only change: clippy, nextest unit tests, static scan.
- `moon run :verify-standard` — adds mutation testing on `commands_ai_context.rs`.
- `moon run :ci` — full pipeline required before merge.

## Waivers

- **Miri**: Waived. `commands_ai_context.rs` is `#![forbid(unsafe_code)]` with no pointer aliasing; Miri adds no signal for this module.
- **Lean**: Waived. No pure deterministic kernel suitable for Lean scope; `proptest` + `kani` provide adequate coverage for the redaction and slot-access properties.
- **Loom/Shuttle**: Waived. No concurrency in the `ai-context` cold path.
