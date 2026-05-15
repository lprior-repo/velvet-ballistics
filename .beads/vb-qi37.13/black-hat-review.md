# State 12 Black-Hat Rerun — vb-qi37.13

STATUS: REJECTED
ROUTE: State 10 implementation repair, then State 11 formal-verifier rerun, then State 12 black-hat rerun.

## Startup sources read

- `/home/lewis/.claude/skills/black-hat-reviewer/SKILL.md` lines 12-16 require exact contract/bead parity and immediate rejection on parity failure; lines 18-21 require functional-core/I/O rigor; lines 23-33 require parse-don't-validate and panic-vector enforcement.
- `/home/lewis/.agents/skills/black-hat-reviewer/SKILL.md` is identical for applied rules and wins on conflict.

## Scope guard

- Review performed only against `/home/lewis/src/vb-qi37-13-r2`.
- Did not use `/home/lewis/src/vb-qi37-13` or `/home/lewis/src/Velvet-ballistics`.

## Findings, ordered by severity

### LETHAL-001 — Structured diagnostics repair is incomplete; supported `--json` routes still emit raw text

- Contract breach: `.beads/vb-qi37.13/contract.md` lines 15 and 23-25 require typed validation diagnostics and machine-readable structured diagnostics with stable `schema_version`, `kind`, `code`, `exit_code`, and `message` where context exists.
- Source breach: `crates/velvet_ballastics/src/main.rs` lines 715-720 handle invalid UTF-8 in `verify --json` with raw `errln!`, bypassing `write_failure_message` / `DiagnosticReport`. Lines 218-224 handle invalid run IDs with raw `errln!` and no `OutputFormat`, so `inspect/events/replay/trace/retry/resume --json` can still leak plain text.
- Direct evidence from `/home/lewis/src/vb-qi37-13-r2`:
  - `./target/debug/velvet-ballastics verify <invalid-utf8-file> --json` -> `code=1`, `stdout=` empty, `stderr=file is not valid UTF-8: invalid utf-8 sequence of 1 bytes from index 0`.
  - `./target/debug/velvet-ballastics inspect not-a-run --db <tmp>/db --json` -> `code=1`, `stdout=` empty, `stderr=invalid run_id 'not-a-run': invalid digit found in string`.
- User risk: operators asking for JSON still receive unparseable stderr on ordinary supported command failures. State 10 fixed the sampled routes, not the contract class.

### MAJOR-001 — State 11 evidence overclaims “structured diagnostics across parse and non-parse routes”

- Ledger/report breach: `.beads/vb-qi37.13/formal-verification-report.md` lines 61-66 and `.beads/vb-qi37.13/machine-gate-report.md` lines 28-31 claim structured diagnostics and stdout/stderr separation across parse and non-parse routes.
- Actual coverage gap: `crates/velvet_ballastics/tests/vb_qi37_13_structured_reconciliation.rs` covers 11 cases, but it does not cover invalid UTF-8 in `verify --json` or invalid run-id diagnostics in database-backed commands. Those untested routes still fail the envelope contract.
- This is false assurance: the State 11 approval says zero blockers while public structured routes remain raw text.

## Positive evidence that is not enough

- Path guard passed in `/home/lewis/src/vb-qi37-13-r2`.
- `cargo test -p velvet_ballastics --test vb_qi37_13_structured_reconciliation --all-features`: PASS, 11/11.
- `cargo test -p velvet_ballastics cli_postcard --all-features`: PASS, 17/17; the prior `cli_postcard::decode_postcard` CRC/digest/version/kind defect appears repaired in source lines 175-231.
- `cargo test -p vb_ui_model --all-features postcard`: PASS, 12/12.

## Verdict

REJECTED. Contract parity still fails for structured diagnostics. Do not proceed to State 13.

## Mandated fixes

1. Thread `OutputFormat` into every public error path that currently cannot know structured mode, starting with `parse_run_id` callers and `cmd_verify` invalid UTF-8.
2. Make every `--json` / `--jsonl` failure route emit `DiagnosticReport` JSON/JSONL on stderr with stdout empty and stable public exit code `0..=8`.
3. Add black-box tests for invalid UTF-8 `verify --json` / `--jsonl` and invalid run-id database commands under `--json` / `--jsonl`.
4. Rerun State 11 and update evidence so it no longer claims class-wide coverage from a sampled matrix.
