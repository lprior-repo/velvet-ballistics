# State 11 Red Queen Rerun Report — vb-qi37.16.4

**Bead ID:** vb-qi37.16.4
**Title:** cli/runtime: Implement durable answer command
**Date:** 2026-05-11
**Phase:** State 11 Red Queen Rerun (Post INV-002 Repair)
**Reviewer:** red-queen adversarial QA

---

## STATUS: APPROVED

---

## Context

This is a deterministic Red Queen rerun after the State 6 INV-002 repair. The repair added
`taint: Option<Taint>` to `IpcPayload::AnswerAsk` (protocol), propagated `taint.unwrap_or(Taint::Clean)` in
`handle_answer_ask` (handler), and enforced INV-002 at `lifecycle.rs:326-330` (runtime).

---

## Challenger Evidence

### Focus Challenger 1: AnswerAsk taint None roundtrip

```bash
$ rtk cargo test -p vb_ipc --lib answer_ask_taint_none_defaults_to_clean
cargo test: 1 passed, 403 filtered out (1 suite, 0.00s)
EXIT: 0
```

**VERDICT:** PASS. `taint: None` round-trips through postcard encode/decode and defaults to `Taint::Clean` at handler.

### Focus Challenger 2: AnswerAsk taint Secret roundtrip

```bash
$ rtk cargo test -p vb_ipc --lib answer_ask_taint_secret_roundtrips
cargo test: 1 passed, 403 filtered out (1 suite, 0.00s)
EXIT: 0
```

**VERDICT:** PASS. `taint: Some(Taint::Secret)` round-trips correctly through the IPC protocol.

### Focus Challenger 3: AnswerAsk taint DerivedFromSecret roundtrip

```bash
$ rtk cargo test -p vb_ipc --lib answer_ask_taint_derived_from_secret_roundtrips
cargo test: 1 passed, 403 filtered out (1 suite, 0.00s)
EXIT: 0
```

**VERDICT:** PASS. `taint: Some(Taint::DerivedFromSecret)` round-trips correctly.

### Focus Challenger 4: AnswerAsk taint Clean explicit roundtrip

```bash
$ rtk cargo test -p vb_ipc --lib answer_ask_taint_clean_explicit_roundtrips
cargo test: 1 passed, 403 filtered out (1 suite, 0.00s)
EXIT: 0
```

**VERDICT:** PASS. `taint: Some(Taint::Clean)` round-trips correctly.

### Focus Challenger 5: Runtime INV-002 enforcement (SecretResultNotAllowed)

```bash
$ rtk cargo test -p vb_runtime --lib red_ask_answer_secret_redaction
cargo test: 1 passed, 1348 filtered out (1 suite, 0.00s)
EXIT: 0
```

**VERDICT:** PASS. Runtime correctly returns `RuntimeError::SecretResultNotAllowed` when
`Taint::Secret` answer is submitted and `allows_secret_results=false`.

### Focus Challenger 6: All IPC answer tests

```bash
$ rtk cargo test -p vb_ipc --lib answer
cargo test: 13 passed, 391 filtered out (1 suite, 0.01s)
EXIT: 0
```

**VERDICT:** PASS. 13 tests (9 pre-existing + 4 new taint roundtrip tests) all pass.

### Focus Challenger 7: All runtime ask_answer tests

```bash
$ rtk cargo test -p vb_runtime --lib ask_answer
cargo test: 24 passed, 1325 filtered out (1 suite, 0.03s)
EXIT: 0
```

**VERDICT:** PASS. 24 tests include INV-002 enforcement proof `red_ask_answer_secret_redaction`.

### Focus Challenger 8: CLI invalid run_id (text)

```bash
$ cargo run -p velvet_ballastics --bin velvet-ballastics -- \
    answer nonexistent_run --step 0 --value-file /tmp/val.bin --db /tmp/db
invalid run_id 'nonexistent_run': invalid digit found in string
EXIT: 1
```

**VERDICT:** PASS. Exit 1 (ValidationFailed), descriptive error message.

### Focus Challenger 9: CLI invalid run_id (JSON)

```bash
$ cargo run -p velvet_ballastics --bin velvet-ballastics -- \
    answer nonexistent_run --step 0 --value-file /tmp/val.bin --db /tmp/db --json
{"error":"invalid run_id 'nonexistent_run': invalid digit found in string","success":false}
EXIT: 1
```

**VERDICT:** PASS. Structured JSON error output correct with `success:false`.

### Focus Challenger 10: CLI invalid run_id (JSONL)

```bash
$ cargo run -p velvet_ballastics --bin velvet-ballastics -- \
    answer nonexistent_run --step 0 --value-file /tmp/val.bin --db /tmp/db --jsonl
{"error":"invalid run_id 'nonexistent_run': invalid digit found in string","success":false}
EXIT: 1
```

**VERDICT:** PASS. Structured JSONL error output correct.

### Focus Challenger 11: CLI missing value_file (text)

```bash
$ cargo run -p velvet_ballastics --bin velvet-ballastics -- \
    answer 123 --step 0 --value-file /nonexistent/file.bin --db /tmp/db
error reading value file /nonexistent/file.bin: No such file or directory (os error 2)
EXIT: 1
```

**VERDICT:** PASS. `Error::ValueFileUnreadable` surfaced as exit 1 with OS error 2.

### Focus Challenger 12: CLI missing value_file (JSON)

```bash
$ cargo run -p velvet_ballastics --bin velvet-ballastics -- \
    answer 123 --step 0 --value-file /nonexistent/file.bin --db /tmp/db --json
{"error":"error reading value file /nonexistent/file.bin: No such file or directory (os error 2)","success":false}
EXIT: 1
```

**VERDICT:** PASS. JSON error output correct with `success:false`.

---

## Regression Check: Prior Black-Hat INV-002 Defect

The State 11 Black Hat found that `IpcPayload::AnswerAsk` hardcoded `taint: Taint::Clean`,
bypassing INV-002 structurally. The State 6 repair added `taint: Option<Taint>` field and
propagated it through handler to runtime enforcement.

Evidence the regression is fixed:
- `answer_ask_taint_secret_roundtrips` proves `Taint::Secret` is now carried in the protocol
- `red_ask_answer_secret_redaction` proves runtime enforces INV-002 (rejects `Taint::Secret`
  when `allows_secret_results=false`)
- The invariant enforcement chain is unbroken: `IpcPayload::AnswerAsk { taint: Some(Taint::Secret) }`
  → handler `unwrap_or` → `AskAnswer { taint: Taint::Secret }` → `lifecycle.rs:326-330` check → `Err(RuntimeError::SecretResultNotAllowed)`

---

## Machine Gates

### Format Gate

```bash
$ rtk cargo fmt -- --check
EXIT: 0
```

### Moon Test Gate

```bash
$ moon run :test
velvet-ballastics:test | 9867 tests run: 9867 passed, 0 skipped
Tasks: 4 completed (1 cached)
EXIT: 0
```

### Moon CI Gate

```bash
$ moon ci
Tasks: 19 completed (2 cached)
Time: 2m 17s 552ms
EXIT: 0
```

---

## Validation (done_when ratchet)

```
VALIDATION: Running 7 checks — the ratchet
═══════════════════════════════════════════════════════════════
  PASS: rtk cargo test -p vb_ipc --lib answer_ask_taint_none_defaults_to_clean
  PASS: rtk cargo test -p vb_ipc --lib answer_ask_taint_secret_roundtrips
  PASS: rtk cargo test -p vb_ipc --lib answer_ask_taint_derived_from_secret_roundtrips
  PASS: rtk cargo test -p vb_ipc --lib answer_ask_taint_clean_explicit_roundtrips
  PASS: rtk cargo test -p vb_runtime --lib red_ask_answer_secret_redaction
  PASS: rtk cargo test -p vb_ipc --lib answer
  PASS: rtk cargo test -p vb_runtime --lib ask_answer

Results: 7/7 passed
ALL CHECKS PASS — ratchet holds
```

---

## Survivors

**None.** All 12 challengers passed (exit 0). No behavioral defects found.

---

## Defects Found

**None.** The INV-002 repair is verified end-to-end:
1. `IpcPayload::AnswerAsk` carries `taint: Option<Taint>` through the protocol
2. Handler correctly propagates `taint.unwrap_or(Taint::Clean)` to `AskAnswer`
3. Runtime correctly enforces INV-002: `Taint::Secret` answers rejected when `allows_secret_results=false`
4. All 4 taint variants (None, Clean, DerivedFromSecret, Secret) round-trip correctly
5. CLI answer error modes correctly surface JSON/text errors with appropriate exit codes
6. All 9867 tests pass; moon ci is green

---

## Verdict

**STATUS: APPROVED**

The INV-002 repair holds under deterministic adversarial pressure. The taint chain is complete:
Protocol → Handler → Runtime INV-002 enforcement → `RuntimeError::SecretResultNotAllowed`.
No regression of the prior blackhat defect. All machine gates pass. The crown is defended.