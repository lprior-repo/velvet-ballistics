bead_id: vb-qi37.16.2
bead_title: cli/runtime durable resume transition
phase: state-14
updated_at: 2026-05-11T00:00:00Z

STATUS: PASS

## Target

- Workspace: `/home/lewis/src/Velvet-ballistics-vb-qi37-16-2-go`
- Binary: `cargo run -q -p velvet_ballastics --bin velvet-ballastics -- ...`
- Feature under final QA: durable resume transition and CLI resume surface.

## Interface discovery

Command:

```bash
cargo run -q -p velvet_ballastics --bin velvet-ballastics -- --help
```

Outcome: exit 0. Relevant stdout excerpt:

```text
resume     <run_id> --db <path> [--json|--jsonl]     Resume a suspended run
options:
  --json      Output structured JSON
  --jsonl     Output structured JSON Lines (one object per line)
```

Version command:

```bash
cargo run -q -p velvet_ballastics --bin velvet-ballastics -- version
```

Output:

```text
velvet-ballastics 0.1.0
```

Exit: 0.

## Manual QA matrix

| ID | Category | Command | Expected | Actual | Status |
|---|---|---|---|---|---|
| QA-01 | discovery | `cargo run -q -p velvet_ballastics --bin velvet-ballastics -- --help` | Help lists `resume <run_id> --db <path> [--json\|--jsonl]` | Exit 0; help listed resume exactly | PASS |
| QA-02 | discovery | `cargo run -q -p velvet_ballastics --bin velvet-ballastics -- version` | Binary starts and prints version | Exit 0; `velvet-ballastics 0.1.0` | PASS |
| QA-03 | missing input | `cargo run -q -p velvet_ballastics --bin velvet-ballastics -- resume` | Missing run id rejected | Exit 1; stderr `missing argument: run_id` plus help | PASS |
| QA-04 | missing input | `cargo run -q -p velvet_ballastics --bin velvet-ballastics -- resume run-qa-001` | Missing `--db` rejected | Exit 1; stderr `missing argument: --db` plus help | PASS |
| QA-05 | invalid input | `cargo run -q -p velvet_ballastics --bin velvet-ballastics -- resume run-qa-001 --db /tmp/velvet-no-such-db-vb-qi37-16-2 --json` | Invalid nonnumeric run id rejected | Exit 1; stderr `invalid run_id 'run-qa-001': invalid digit found in string` | PASS |
| QA-06 | fail-closed valid id | `cargo run -q -p velvet_ballastics --bin velvet-ballastics -- resume 1001 --db /tmp/velvet-no-such-db-vb-qi37-16-2 --json` | Unknown run fails closed with structured JSON | Exit 5; stderr `{"error":"run 1001 not found","success":false}` | PASS |
| QA-07 | runtime happy path | `cargo test -q -p vb_runtime resume_keeps_awaiting_action_resumable_after_resume -- --nocapture` | Resume from resumable run reports resumed and preserves resumable post-drive state | Exit 0; 1 passed | PASS |
| QA-08 | storage replay | `cargo test -q -p vb_storage --test replay_resume -- --nocapture` | Replay/resume storage tests pass | Exit 0; 3 passed | PASS |

## Verbatim execution evidence

### QA-03 missing run id

```text
$ cargo run -q -p velvet_ballastics --bin velvet-ballastics -- resume
--- stdout ---
<empty>
--- stderr ---
missing argument: run_id

velvet-ballastics - compiled workflow runtime
...
--- exit: 1 ---
```

### QA-04 missing db

```text
$ cargo run -q -p velvet_ballastics --bin velvet-ballastics -- resume run-qa-001
--- stdout ---
<empty>
--- stderr ---
missing argument: --db

velvet-ballastics - compiled workflow runtime
...
--- exit: 1 ---
```

### QA-05 invalid run id

```text
$ cargo run -q -p velvet_ballastics --bin velvet-ballastics -- resume run-qa-001 --db /tmp/velvet-no-such-db-vb-qi37-16-2 --json
--- stdout ---
<empty>
--- stderr ---
invalid run_id 'run-qa-001': invalid digit found in string
--- exit: 1 ---
```

### QA-06 valid numeric id missing from journal/db

```text
$ cargo run -q -p velvet_ballastics --bin velvet-ballastics -- resume 1001 --db /tmp/velvet-no-such-db-vb-qi37-16-2 --json
--- stdout ---
<empty>
--- stderr ---
{"error":"run 1001 not found","success":false}
--- exit: 5 ---
```

### QA-07 runtime durable resume regression

```text
$ cargo test -q -p vb_runtime resume_keeps_awaiting_action_resumable_after_resume -- --nocapture
running 1 test
.
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1367 filtered out; finished in 0.00s
--- exit: 0 ---
```

### QA-08 storage replay resume tests

```text
$ cargo test -q -p vb_storage --test replay_resume -- --nocapture
running 3 tests
...
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
--- exit: 0 ---
```

## Findings

- No CRITICAL or MAJOR failures found in final manual QA.
- Resume CLI discovery, missing-input rejection, invalid run-id rejection, structured fail-closed unknown-run behavior, runtime resume behavior, and storage replay coverage all passed with real command evidence.

## Summary

- Total final manual QA checks: 8
- Passed: 8
- Failed: 0
- Severity breakdown: 0 CRITICAL, 0 MAJOR, 0 MINOR.
