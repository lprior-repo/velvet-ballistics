bead_id: vb-qi37.15.3
bead_title: cli: Add trace command
phase: State 3
updated_at: 2026-05-12T00:00:00Z

# Verification Layers

## Boundary
- Verus-owned kernel: `build_trace` and `trace_one` pure functions in `commands_journal.rs`
- TLA+ temporal model: None (read-only journal replay, no temporal behavior)
- Theorem projection: None required; Verus covers pure function determinism
- Runtime shell: `cmd_trace`, `read_journal_events`, output formatting, CLI dispatch
- External systems excluded from formal proof: Fjall journal storage (immutable input source)

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Evidence | Notes |
|-----------------|---------------|-------------------|-------|
| PRE-001 | static-scan | `cargo clippy` parse_run_id validation | Parser unit tests in args.rs |
| PRE-002 | manual-qa | smoke test: trace with valid db path | gauntlet-standard |
| PRE-003 | gauntlet-standard | black-box: trace on known run_id | integration test cli_integration.rs |
| POST-001 | gauntlet-standard | black-box: trace output contains events | cli_integration trace test |
| POST-002 | gauntlet-standard | json/jsonl structure validation | test verifies TraceEntry fields |
| POST-003 | gauntlet-standard | json output parse test | test parses json output |
| POST-004 | gauntlet-standard | jsonl output parse test | test parses each jsonl line |
| POST-005 | gauntlet-standard | text output format test | test checks text line format |
| POST-006 | gauntlet-standard | trace on non-existent run returns empty/success | integration test |
| POST-007 | gauntlet-standard | storage error returns non-zero | error path test |
| INV-001 | verus + proptest | pure function determinism property test | proptest on build_trace |
| INV-002 | static-scan | trace is read-only; no write_journal calls | code review; source scan |
| INV-003 | gauntlet-standard | deterministic output given same events | integration test |
| ERR-001 | static-scan | clippy parse_run_id error handling | parser tests |
| ERR-002 | gauntlet-standard | invalid db path returns error | integration test |
| ERR-004 | gauntlet-standard | journal read error returns error | integration test |

## Verus Scope
- Rust target: `crates/velvet_ballastics/src/commands_journal.rs::build_trace`, `trace_one`
- Spec/Proof function: `build_trace` spec fn; `trace_one` spec fn
- Invariants: determinism, completeness, index correspondence, event variant coverage
- Trusted boundary: `JournalEvent` enum variants are storage-validated
- Shell exclusions: journal I/O, CLI dispatch, output formatting, error reporting
- Evidence command: `cargo test -p velvet_ballastics -- commands_journal`

## TLA+ Scope
- None; TLA+ waiver applies.

## Theorem Scope
- None; Verus covers pure function determinism.

## Waivers
- TLA+ waiver: Reason=no temporal behavior in read-only journal replay. Owner=vb-qi37.15.3 agent. Expiry=permanent unless scope adds temporal behavior.