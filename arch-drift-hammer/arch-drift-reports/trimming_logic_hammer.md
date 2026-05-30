# Architectural Drift Report: `trimming/logic.rs`

**File**: `crates/vb_storage/src/trimming/logic.rs`
**Status**: 🚨 ARCHITECTURAL DRIFT VIOLATION
**Line Count**: 307 (exceeds 300-line limit by 7 lines)
**Assessed**: 2026-05-29

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 307 | 300 | 🔴 EXCEEDED |

**Immediate action required**: This file MUST be refactored below 300 lines.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Hardcoded Magic Numbers

| Location | Primitive | Issue |
|----------|-----------|-------|
| L23, L82, L223 | `key.len() < 17` | Magic byte length check; `JOURNAL_KEY_BYTES` constant exists in `keys.rs` but is not used |
| L27, L86, L226 | `key.get(9..17)` | Hardcoded slice positions for EventSeq extraction |
| L29-32, L88-91, L229-232 | `[u8; 8]` + `try_into()` + `u64::from_be_bytes()` | Raw byte array manipulation repeated 3x |
| L78, L149, L150, L151, L152, L219, L295, L300 | `u64` counters | All statistics tracked as raw `u64` instead of domain types |

### 2.2 Missing Value Objects

**RunEventKey parsing is scattered and primitive**:
```rust
// THIS PATTERN APPEARS 3 TIMES:
let slice = key.get(9..17).ok_or(...)?;
let seq_bytes: [u8; 8] = slice.try_into()?;
let seq_u64 = u64::from_be_bytes(seq_bytes);
```

**Should be extracted to a `RunEventKey` decoder method or value object**.

### 2.3 Policy Fields Are Primitives

`TrimPolicy.retain_last_n_terminal: u64` — retention count should be a bounded `RetainCount(u64)` newtype.

---

## 3. DUPLICATED CODE (DRY VIOLATIONS)

### 3.1 EventSeq Extraction Pattern (3 copies)

| Function | Lines | Code |
|----------|-------|------|
| `latest_durable_snapshot_seq` | 26-33 | full extraction |
| `trim_events_for_run` | 85-91 | full extraction |
| `count_trimmable_events` | 226-232 | full extraction |

**All three do identical byte manipulation to extract `EventSeq` from a key.**

### 3.2 Key Length Guard Pattern (3 copies)

```rust
if key.len() < 17 {
    continue; // or return Err
}
```
Appears at: **L23-24**, **L82-83**, **L223-224**

### 3.3 Run Header Lookup in Loop

`terminal_runs.push((h.run, h.accepted_at_ms))` at **L289** builds a vector, but `h.accepted_at_ms` is raw `u64` timestamp instead of a `Timestamp` value object.

---

## 4. RESPONSIBILITY SMELL

### 4.1 `trim_eligibility_diagnostic` Is a God Method (~70 lines)

This single method:
- Fetches all run headers
- Iterates all runs
- Calls `latest_durable_snapshot_seq` per run
- Calls `check_retention_policy` per run
- Calls `count_trimmable_events` per run
- Accumulates 6 different statistics
- Builds a `TrimDiagnostic` response

**Should be decomposed** into smaller collaborators with clear contracts.

### 4.2 `check_retention_policy` Has Hidden Side Effects

```rust
pub(crate) fn check_retention_policy(&self, run: RunId, policy: &TrimPolicy) -> TrimResult<()> {
    if !self.has_terminal_event(run)? {  // <-- SIDE EFFECT: full event scan
        return Ok(());
    }
    // ...
}
```

`check_retention_policy` calls `has_terminal_event` which **iterates the entire event journal for the run**. This is a hidden O(n) side effect masquerading as a simple policy check.

---

## 5. SCOTT WLASCHIN DDD VIOLATIONS

| Principle | Violation |
|-----------|-----------|
| **No primitive obsession** | 17, 9, 8 as raw magic numbers throughout |
| **Make illegal states unrepresentable** | `key.len() < 17` checks suggest key format not enforced at type level |
| **Single responsibility** | `trim_eligibility_diagnostic` mixes scanning, policy checking, counting, and accumulation |
| **Errors as values** | `unwrap_or(terminal_runs.len())` at L298 is a panic point |

---

## 6. SPECIFIC CODE LOCATIONS REQUIRING ATTENTION

| Lines | Issue | Remediation |
|-------|-------|--------------|
| 23-33 | EventSeq extraction from raw bytes | Extract to `impl [u8] { fn decode_event_seq(&self) -> Option<EventSeq> }` |
| 26-33, 85-91, 226-232 | Identical byte parsing | Single shared helper |
| 23-24, 82-83, 223-224 | Key length guards | `const JOURNAL_KEY_LEN: usize = 17;` used everywhere |
| 298 | `unwrap_or` panic point | Use `position.unwrap_or(...)` with explicit error |
| 268-305 | `check_retention_policy` side effects | Return terminal status from caller, don't scan inside policy check |
| 143-209 | `trim_eligibility_diagnostic` | Split into `scan_eligible_runs`, `count_trimmable_events_for_run`, `build_diagnostic` |
| 78-79, 149-152 | Raw `u64` counters | Wrap in `TrimStats { total_runs, eligible_runs, ... }` |

---

## 7. RECOMMENDED REFACTORING PLAN

### Phase 1: Extract Value Objects & Helpers (no behavior change)
1. Create `impl [u8]` extension for `decode_event_seq_from_key(prefix_offset: usize) -> Option<EventSeq>`
2. Create `const JOURNAL_KEY_LENGTH: usize = 17;` and `const EVENT_SEQ_OFFSET: usize = 9;`
3. Replace all `key.len() < 17` with `key.len() < JOURNAL_KEY_LENGTH`
4. Replace all raw `u64::from_be_bytes` sequences with single helper call

### Phase 2: Reduce Line Count
1. Break `trim_eligibility_diagnostic` into 3 helper methods
2. Break `check_retention_policy` to accept pre-computed terminal status
3. Target: reduce from 307 to ~250 lines

### Phase 3: DDD Polish
1. Create `RetainCount(u64)` wrapper for `retain_last_n_terminal`
2. Create `TrimStats` aggregate for the 4+ u64 counters
3. Make `RunEventKey` a proper decodeable value object

---

## 8. VERIFICATION COMMAND

```bash
cd /home/lewis/src/velvet-ballistics
wc -l crates/vb_storage/src/trimming/logic.rs  # Must be <= 300
```

---

**VERDICT**: 🔴 **ARCHITECTURAL DRIFT CONFIRMED**

This file is 307 lines and exhibits severe primitive obsession, DRY violations, and SRP breaches. The trimming logic responsibilities are smeared across 7 methods with duplicated byte-decoding logic in 3 places. A single 70-line god method handles diagnostic collection.

**Required Actions**:
1. Extract the 3 identical EventSeq-decoding patterns into a single helper
2. Break `trim_eligibility_diagnostic` into smaller collaborators
3. Move `has_terminal_event` side effect out of `check_retention_policy`
4. Reduce to ≤300 lines
5. Create typed wrappers for raw counters and policy fields
