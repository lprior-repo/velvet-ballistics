# Final Manual QA — vb-qi37.16.4

STATUS: PASS

## Scope

- State: 14 final hands-on QA after State 13 architectural drift approval.
- Workspace: `/home/lewis/src/Velvet-ballistics-vb-qi37-16-4-go`.
- Forbidden checkout: `/home/lewis/src/Velvet-ballistics` was not used.
- Feature under test: durable `answer` command plus runtime/IPC ask-answer behavior.

## Commands and Evidence

### 1. CLI surface discovery

Command:

```bash
timeout 180 cargo run -q -p velvet_ballistics --bin velvet-ballistics -- --help; code=$?; printf '\nEXIT:%s\n' "$code"
```

Outcome: PASS. The command listed `answer <run_id> --step <N> --value-file <file> --db <path> [--json|--jsonl]` and exited 0.

Verbatim excerpt:

```text
  answer     <run_id> --step <N> --value-file <file> --db <path> [--json|--jsonl]  Answer a suspended step

EXIT:0
```

### 2. Answer command missing/invalid argument behavior

Command:

```bash
timeout 180 cargo run -q -p velvet_ballistics --bin velvet-ballistics -- answer; code=$?; printf '\nEXIT:%s\n' "$code"
```

Outcome: PASS. Missing `run_id` is rejected without panic.

Verbatim excerpt:

```text
missing argument: run_id
EXIT:1
```

Command:

```bash
timeout 180 cargo run -q -p velvet_ballistics --bin velvet-ballistics -- answer not-a-run --step 0 --value-file /no/such/value.bin --db /tmp/vb-qa-db --json; code=$?; printf '\nEXIT:%s\n' "$code"
```

Outcome: PASS. Invalid run ID returns structured JSON failure without panic.

Verbatim output:

```text
{"error":"invalid run_id 'not-a-run': invalid digit found in string","success":false}

EXIT:1
```

### 3. Answer command IPC connection failure path

Command:

```bash
tmp=$(mktemp -d); python3 - <<'PY' "$tmp/value.bin"
from pathlib import Path
import sys
Path(sys.argv[1]).write_bytes(b'qa-answer-bytes')
PY
timeout 180 cargo run -q -p velvet_ballistics --bin velvet-ballistics -- answer 42 --step 0 --value-file "$tmp/value.bin" --db "$tmp/run-db" --json; code=$?; printf '\nEXIT:%s\n' "$code"; rm -rf "$tmp"
```

Outcome: PASS. The CLI reads an answer file, derives the socket path from `--db`, attempts IPC, and reports a structured connection failure without panic.

Verbatim output:

```text
{"error":"error connecting to IPC server at /tmp/tmp.aTacUMqbWR/run-db.sock: connect failed: No such file or directory (os error 2)","success":false}

EXIT:6
```

### 4. IPC answer tests

Command:

```bash
timeout 300 cargo test -q -p vb_ipc answer -- --nocapture; code=$?; printf '\nEXIT:%s\n' "$code"
```

Outcome: PASS.

Verbatim output:

```text
running 13 tests
.............
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 391 filtered out; finished in 0.00s


EXIT:0
```

### 5. CLI answer tests

Command:

```bash
timeout 300 cargo test -q -p velvet_ballistics answer -- --nocapture; code=$?; printf '\nEXIT:%s\n' "$code"
```

Outcome: PASS.

Verbatim excerpt:

```text
running 1 test
.
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 93 filtered out; finished in 0.00s

running 1 test
.
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 93 filtered out; finished in 0.00s

running 1 test
.
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 70 filtered out; finished in 0.00s

EXIT:0
```

### 6. Runtime ask-answer tests

Command:

```bash
timeout 300 cargo test -q -p vb_runtime ask_answer -- --nocapture; code=$?; printf '\nEXIT:%s\n' "$code"
```

Outcome: PASS.

Verbatim output:

```text
running 24 tests
........................
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 1325 filtered out; finished in 0.01s


running 1 test
.
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s


EXIT:0
```

## Decision

Final manual QA passes. CLI discovery and negative paths behaved correctly, and targeted CLI/IPC/runtime answer test suites passed after State 13 made no code changes.
