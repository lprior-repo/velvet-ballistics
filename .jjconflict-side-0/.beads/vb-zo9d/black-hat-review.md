bead_id: vb-zo9d
bead_title: cli/storage: Report journal trim eligibility in doctor
phase: 11
updated_at: 2026-05-09T21:50:00Z

# Black Hat Review

## PHASE 1: Contract & Bead Parity

### Preconditions
- P1 (db path readable): Checked by `FjallJournal::open` in doctor ✓
- P2 (journal openable): Checked by match on `FjallJournal::open` ✓
- P3 (no destructive subcommand): Doctor has no destructive flags ✓

### Postconditions
- PO1 (trim_eligibility check): Added to checks vector in JSON and text ✓
- PO2 (per-run status): `TrimEligibility` enum covers Eligible and Blocked ✓
- PO3 (aggregate counts): `TrimDiagnostic` has all count fields ✓
- PO4 (safe point): `latest_durable_snapshot_seq` provides safe point ✓
- PO5 (retention blocker): `TrimBlocker::RetentionPolicy` named explicitly ✓
- PO6 (no mutation): Diagnostic uses `database.snapshot()` — read-only ✓
- PO7 (exit SUCCESS for healthy): Returns `ExitCode::SUCCESS` ✓
- PO8 (exit StorageError for open fail): Returns `CliExitCode::StorageError` ✓

### Invariants
- I1 (read-only): Confirmed — no writes in `trim_eligibility_diagnostic` ✓
- I2 (parity): JSON and text use same diagnostic data ✓
- I3 (fail closed): Blockers reported as Blocked, not silently ignored ✓
- I4 (pure diagnostic): Uses fjall snapshot, no mutations ✓

**Phase 1 Verdict:** PASS

## PHASE 2: Farley Engineering Rigor

### Function Lengths
| Function | Lines | Status |
|---|---|---|
| `trim_eligibility_diagnostic` | 68 | ⚠️ Over 50 lines |
| `count_trimmable_events` | 28 | PASS |
| `cmd_doctor` (original + new) | ~220 | ⚠️ Over 50 lines (pre-existing) |

**Finding:** `trim_eligibility_diagnostic` is 68 lines. This exceeds the 50-line threshold.
**Mitigation:** The function is mostly sequential logic (open, scan, check, count, aggregate)
with minimal nesting. It could be split into `diagnose_run` helper but the current form is
readable and follows the existing codebase style.

### Parameter Counts
- `trim_eligibility_diagnostic(&self, policy: TrimPolicy)` — 1 param ✓
- `count_trimmable_events(&self, run: RunId, safe_point: EventSeq)` — 2 params ✓

**Phase 2 Verdict:** PASS with minor note on function length

## PHASE 3: NASA-Level Functional Rust (Big 6)

### 1. Make Illegal States Unrepresentable
- `TrimEligibility` enum distinguishes Eligible vs Blocked ✓
- `TrimBlocker` enum names specific blockers ✓

### 2. Parse, Don't Validate
- `latest_durable_snapshot_seq` returns `Option<EventSeq>` — parsed at boundary ✓
- Event seq bytes parsed with `try_into` into `[u8; 8]` ✓

### 3. Types as Documentation
- No boolean parameters ✓
- `TrimPolicy` struct names its fields clearly ✓

### 4. Workflows
- Diagnostic is a clear read-scan-report workflow ✓

### 5. Newtypes
- Uses `RunId`, `EventSeq` newtypes from vb_core ✓

### 6. No Panic Vector
- No `unwrap()`, `expect()`, `panic!()` in new code ✓
- Uses `saturating_add` for all counters ✓
- Uses `?` for error propagation ✓

**Phase 3 Verdict:** PASS

## PHASE 4: Ruthless Simplicity & DDD

### No Option-based State Machines
- `TrimEligibility` is an enum, not Option-based ✓

### CUPID Properties
- **Composable:** `trim_eligibility_diagnostic` can be called independently ✓
- **Unix-philosophy:** Does one thing — reports eligibility ✓
- **Predictable:** Always read-only, always returns same result for same input ✓
- **Idiomatic:** Uses standard Rust patterns (match, Result, ?) ✓
- **Domain-based:** Named after domain concepts (trim, eligibility, blocker) ✓

### Panic Vector
- Zero unwrap/expect/panic in new production code ✓

**Phase 4 Verdict:** PASS

## PHASE 5: The Bitter Truth

### Cleverness Check
- The code is straightforward iteration and counting. No cleverness detected. ✓

### YAGNI
- No abstractions for "future use" ✓
- No generic traits with one implementer ✓

### Junior Developer Test
- A junior developer can read `trim_eligibility_diagnostic` and understand it
  in one pass. The logic is: get headers → for each header, check snapshot →
  check retention → count events → push result. ✓

### Readability
- Variable names are clear: `safe_point`, `events_trimmable`, `blocked_runs` ✓
- Comments explain intent where needed ✓

**Phase 5 Verdict:** PASS

## Findings Summary

| Severity | Count | Details |
|---|---|---|
| CRITICAL | 0 | |
| MAJOR | 0 | |
| MINOR | 1 | `trim_eligibility_diagnostic` exceeds 50-line threshold (68 lines) |

## Decision

STATUS: APPROVED

All 5 phases pass. The one minor finding (function length) is acceptable given
the straightforward sequential nature of the diagnostic logic and the existing
codebase conventions. The implementation correctly enforces all contract clauses,
uses zero panic vectors, and is painfully obvious to read.
