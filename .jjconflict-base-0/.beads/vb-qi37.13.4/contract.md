bead_id: vb-qi37.13.4
bead_title: cli: Structured output contract tests
phase: State 3
updated_at: 2026-05-11T00:00:00Z

# Contract Specification

## Preconditions
- PRE-001: The `vb` binary is available to black-box integration tests.
- PRE-002: Tests invoke commands non-interactively with explicit arguments and temp files only.

## Postconditions
- POST-001: Happy-path CLI commands exit 0 and write primary payloads to stdout.
- POST-002: Bad-input CLI commands exit non-zero and write diagnostics to stderr.
- POST-003: Structured output tests assert stable machine-readable fields for supported `--json`/`--jsonl` modes and expose missing master `--emit text|yaml|postcard` coverage as a contract gap.

## Invariants
- INV-001: CLI output must not contain Rust panic text, stack traces, or interactive prompts.
- INV-002: Responses remain bounded by test-enforced maximum byte limits.

## Error Taxonomy
- ERR-001: Parse/validation errors map to non-zero documented CLI exit codes.
- ERR-002: Unsupported/unknown modes return typed stderr diagnostics.

## Contract Signatures
- black_box_cli(args: &[OsStr]) -> Result<Output, CliContractError>

## Verus-Owned Clauses
- None for this bead: the scope is black-box process-level CLI contract testing, not pure Rust core logic.

## TLA+-Owned Clauses
- None: no temporal state machine is introduced by test-only CLI contract coverage.

## Non-goals
- Implementing missing emitters or diagnostic envelopes belongs to owning feature beads, not this test-contract bead unless tests expose a red gate.
