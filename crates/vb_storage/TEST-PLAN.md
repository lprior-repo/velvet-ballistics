# vb_storage TEST-PLAN.md

## Crate: vb_storage

### VERDICT CONTEXT
- **Status**: REJECTED
- **Clippy**: 288 errors (2 warnings currently visible after partial fixes)
- **Coverage Issues**: artifacts.rs 0%, error.rs 33.85%, process_lock.rs 44.19%
- **Integration Purity**: `use crate::` imports in integration tests

---

## Section 1 — Behavior Inventory

### artifacts.rs (0% coverage — LETHAL)

| Subject | Action | Outcome | Condition |
|---------|--------|---------|-----------|
| FjallJournal | `list_artifacts` | Returns `Vec<WorkflowDigest>` when keyspace non-empty | valid prefix exists |
| FjallJournal | `list_artifacts` | Returns empty `Vec` when no artifacts stored | keyspace empty |
| FjallJournal | `list_artifacts` | Returns `Err(UnexpectedEof)` when key slice invalid | corrupted key |
| FjallJournal | `remove_artifact` | Returns `Ok(())` when artifact existed and removed | digest present |
| FjallJournal | `remove_artifact` | Returns `Err(ArtifactNotFound)` when digest absent | digest not in store |
| FjallJournal | `artifact_exists` | Returns `Ok(true)` when artifact stored | digest present |
| FjallJournal | `artifact_exists` | Returns `Ok(false)` when artifact absent | digest absent |

### process_lock.rs (44.19% coverage — LETHAL)

| Subject | Action | Outcome | Condition |
|---------|--------|---------|-----------|
| ProcessLock | `acquire` | Returns `Ok(Self)` with file descriptor | lock file creatable |
| ProcessLock | `acquire` | Returns `Err(ProcessLockHeld)` with `holder_pid` | another process holds |
| ProcessLock | `acquire` | Returns `Err(ProcessLockIo)` on I/O error | permission denied / disk full |
| ProcessLock | `acquire` | Writes PID to lock file on success | best-effort, non-critical |
| read_holder_pid | (internal) | Returns `Some(u32)` when PID parseable | valid u32 in file |
| read_holder_pid | (internal) | Returns `None` when file empty | empty lock file |
| read_holder_pid | (internal) | Returns `None` when parse fails | malformed PID |

### error.rs (33.85% coverage — LETHAL)

Every `JournalError` variant must have a scenario:

| Variant | Trigger Scenario |
|---------|-----------------|
| `Fjall` | Invalid path / corruption |
| `Encode` | Postcard serialize failure |
| `KeyCapacity` | Key > 64 bytes |
| `DuplicateEvent` | Same run+seq appended twice |
| `WriteLockPoisoned` | Lock holder panicked |
| `QueueCapacity` | Queue::new(0) |
| `QueueFull` | Producer outpaces consumer |
| `QueueShutdown` | Queue::shutdown called |
| `WrongRun` | Replay returns mismatched run |
| `SequenceGap` | Replay finds non-contiguous seq |
| `SequenceOverflow` | EventSeq u64 exhausted |
| `BadMagic` | Wrong magic bytes |
| `UnsupportedSchemaVersion` | version > CURRENT |
| `MigrationRequired` | version < CURRENT but supported |
| `UnknownRecordKind` | kind not in family |
| `RecordKindFamilyMismatch` | kind/family mismatch |
| `HeaderLengthMismatch` | Header length != expected |
| `PayloadTooLarge` | payload > max |
| `HeaderChecksumMismatch` | CRC mismatch |
| `PayloadDigestMismatch` | BLAKE3 mismatch |
| `UnexpectedEof` | Truncated record |
| `PostcardDecodeFailed` | Malformed postcard |
| `ArtifactMalformed` | Invalid artifact structure |
| `ArtifactChecksumMismatch` | Digest mismatch |
| `InvalidGateCount` | gate_count != 15 |
| `MissingRequiredProofFlag` | Required flag false |
| `ArtifactNotFound` | Digest absent |
| `AdmissionRequired` | Raw admission without accepted artifacts |
| `ArtifactInvalid` | Artifact validation failed |
| `InputTooLarge` | Input > bounded limit |
| `InputSchemaMismatch` | Input doesn't match artifact schema |
| `CapabilityDenied` | Runtime capability insufficient |
| `SecretUnavailable` | Required secret missing |
| `RunAlreadyExists` | RunId already active |
| `ActiveRunCapacityExceeded` | Too many active runs |
| `FrameAllocationFailed` | Memory allocation failed |
| `AdmissionJournalFailed` | Journal append failed |
| `StrictDurabilityFailed` | Durability barrier unmet |
| `ClockUnavailable` | Timestamp source unavailable |
| `ProcessLockHeld` | Another process holds lock |
| `ProcessLockIo` | Lock file I/O error |
| `Trim` | Trim operation failed |

### journal.rs (impl Drop LETHAL at line 329)

| Subject | Action | Outcome | Condition |
|---------|--------|---------|-----------|
| FjallJournal | `Drop::drop` | Silently ignores persist error | persist call fails |

---

## Section 2 — Trophy Allocation

```
File             | Size  | Layer       | Target% | Justification
-----------------|-------|-------------|---------|----------------------------------
artifacts.rs     | 45 ln | unit        | 95%     | Pure calculation, no I/O in tests
error.rs         | 494 ln| unit        | 65%     | Enum variants, exhaustive match
process_lock.rs  | 104 ln| unit        | 80%     | Simple I/O, proptest friendly
journal.rs       | 2401ln | integration | 70%     | Complex state machine
batch.rs         | 66 K  | integration | 60%     | Write batch lifecycle
recovery/*.rs    | varied| integration | 75%     | Recovery state machine
admission.rs     | 28 K  | integration | 70%     | Admission flow
security_tests.rs| 36 K  | integration | 60%     | Adversarial paths
trimming.rs      | 47 K  | integration | 55%     | Trim policy state machine
codec.rs         | 103 K | unit        | 80%     | Pure encoding/decoding
```

**Target**: ~60% integration, ~30% unit, ~5% e2e, ~5% static analysis

---

## Section 3 — BDD Scenarios

### artifacts.rs

```gherkin
### Behavior: list_artifacts returns empty Vec when no artifacts stored
Given: a FjallJournal with empty compiled_ir keyspace
When: list_artifacts() is called
Then: the result is Ok(empty Vec)

### Behavior: list_artifacts returns digests when artifacts exist
Given: a FjallJournal with 3 stored compiled IR artifacts
When: list_artifacts() is called
Then: the result is Ok(Vec) containing exactly 3 digests

### Behavior: list_artifacts returns UnexpectedEof on corrupted key
Given: a FjallJournal with a key shorter than 1+DIGEST_BYTES
When: list_artifacts() iterates over the malformed key
Then: the result is Err(JournalError::UnexpectedEof)

### Behavior: remove_artifact returns Ok when artifact existed
Given: a FjallJournal with a known artifact digest
When: remove_artifact(digest) is called
Then: the result is Ok(()) and artifact_exists(digest) returns false

### Behavior: remove_artifact returns ArtifactNotFound when digest absent
Given: a FjallJournal with no artifact for digest D
When: remove_artifact(D) is called
Then: the result is Err(JournalError::ArtifactNotFound { digest: D })

### Behavior: artifact_exists returns true when stored
Given: a FjallJournal with artifact digest D stored
When: artifact_exists(D) is called
Then: the result is Ok(true)

### Behavior: artifact_exists returns false when absent
Given: a FjallJournal with no artifact for digest D
When: artifact_exists(D) is called
Then: the result is Ok(false)
```

### process_lock.rs

```gherkin
### Behavior: acquire returns Ok on successful lock
Given: a valid db_path directory with write permission
When: ProcessLock::acquire(db_path) is called
Then: the result is Ok(ProcessLock) and flock is held

### Behavior: acquire returns ProcessLockHeld when contested
Given: ProcessLock A holds the lock on db_path
When: ProcessLock::acquire(db_path) is called from process B
Then: the result is Err(JournalError::ProcessLockHeld { holder_pid: Some(A's pid) })

### Behavior: acquire returns ProcessLockIo on permission denied
Given: db_path directory with no write permission
When: ProcessLock::acquire(db_path) is called
Then: the result is Err(JournalError::ProcessLockIo { .. })

### Behavior: read_holder_pid returns Some when PID valid
Given: a lock file containing "12345\n"
When: read_holder_pid(file) is called
Then: the result is Some(12345)

### Behavior: read_holder_pid returns None when file empty
Given: a lock file that is empty
When: read_holder_pid(file) is called
Then: the result is None

### Behavior: read_holder_pid returns None when parse fails
Given: a lock file containing "not-a-number\n"
When: read_holder_pid(file) is called
Then: the result is None
```

### error.rs (Every variant)

```gherkin
### Behavior: JournalError::Fjall wraps fjall::Error
Given: a fjall::Error variant
When: JournalError::from(err) is called
Then: the result is JournalError::Fjall(err)

### Behavior: JournalError::DuplicateEvent contains run and seq
Given: run = RunId::new(1), seq = EventSeq::new(5)
When: JournalError::DuplicateEvent { run, seq } is constructed
Then: the error message contains "run 1" and "seq 5"

### Behavior: JournalError::ProcessLockHeld includes holder_pid
Given: holder_pid = Some(12345)
When: JournalError::ProcessLockHeld is constructed
Then: the error message contains "pid: Some(12345)"

### Behavior: JournalError::ProcessLockHeld path is preserved
Given: path = PathBuf::from("/tmp/.process.lock")
When: JournalError::ProcessLockHeld is constructed
Then: the path field equals the input path

### Behavior: JournalError::ArtifactNotFound includes digest
Given: digest = WorkflowDigest::from_bytes([0xAB; 32])
When: JournalError::ArtifactNotFound { digest } is constructed
Then: the error message contains the digest bytes

### Behavior: JournalError::PayloadTooLarge shows len and max
Given: len = 1000, max = 500
When: JournalError::PayloadTooLarge { len, max } is constructed
Then: the error message contains "1000 > 500"

### Behavior: JournalError::diagnostic_code returns correct code for each variant
Given: a JournalError variant
When: diagnostic_code() is called
Then: the returned DiagnosticCode matches the constant for that variant
```

---

## Section 4 — Proptest Invariants

### error.rs invariants

```rust
// JournalError diagnostic_code is consistent with variant
proptest! {
    #[test]
    fn journal_error_diagnostic_code_is_consistent(error: JournalError) {
        let code = error.diagnostic_code();
        prop_assert!(code.as_u32() != 0, "diagnostic code must be non-zero");
    }
}

// VerificationWarning diagnostic_code is consistent
proptest! {
    #[test]
    fn verification_warning_code_is_schema_mismatch(code: u16) {
        let warning = VerificationWarning::SchemaVersionMismatch {
            found: code,
            current: code.saturating_add(1),
        };
        prop_assert_eq!(
            warning.diagnostic_code(),
            VERIFICATION_WARNING_SCHEMA_MISMATCH_CODE
        );
    }
}
```

### process_lock.rs invariants

```rust
// read_holder_pid is None for empty file and Some for valid u32
proptest! {
    #[test]
    fn read_holder_pid_roundtrip(pid: u32) {
        // Create temp file with PID string
        let path = ...;
        std::fs::write(&path, format!("{pid}\n")).unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let result = read_holder_pid(&file);
        prop_assert_eq!(result, Some(pid));
    }
}

// read_holder_pid is None for non-u32 content
proptest! {
    #[test]
    fn read_holder_pid_invalid_content(content: String) {
        // content must not parse as u32
        let path = ...;
        std::fs::write(&path, &content).unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let result = read_holder_pid(&file);
        prop_assert_eq!(result, None);
    }
}
```

### artifacts.rs invariants

```rust
// list_artifacts is idempotent (calling twice returns same set)
proptest! {
    #[test]
    fn list_artifacts_idempotent(journal: FjallJournal, digests: Vec<WorkflowDigest>) {
        // Store artifacts
        for digest in &digests {
            journal.put_compiled_ir(...).unwrap();
        }
        let first = journal.list_artifacts().unwrap();
        let second = journal.list_artifacts().unwrap();
        prop_assert_eq!(first, second);
    }
}

// remove_artifact twice returns ArtifactNotFound on second call
proptest! {
    #[test]
    fn remove_artifact_twice_fails(digest: WorkflowDigest) {
        journal.put_compiled_ir(...).unwrap();
        journal.remove_artifact(digest).unwrap();
        let result = journal.remove_artifact(digest);
        prop_assert!(matches!(result, Err(JournalError::ArtifactNotFound { .. })));
    }
}
```

---

## Section 5 — Fuzz Targets

### artifacts.rs

```rust
// Fuzz target: list_artifacts with corrupted keyspace
#[derive(arbitrary::Arbitrary)]
struct CorruptedKeyspaceInput {
    key_bytes: Vec<u8>,
}

extern "C" fn fuzz_list_artifacts(data: &[u8]) -> usize {
    // Seed a journal with arbitrary key/value pairs
    // Call list_artifacts and verify it doesn't panic
    // Returns 1 if UnexpectedEof returned correctly, 0 otherwise
}

// Fuzz target: remove_artifact with arbitrary digest
extern "C" fn fuzz_remove_artifact(data: &[u8]) -> usize {
    // Use first 32 bytes as digest, rest as potential artifact
    // Verify ArtifactNotFound or proper removal
}
```

### error.rs

```rust
// Fuzz target: JournalError::from(postcard_error)
extern "C" fn fuzz_decode_error(data: &[u8]) -> usize {
    // Try to decode as postcard Error
    // Verify all variants produce valid JournalError
}

// Fuzz target: ArtifactInvalidSource roundtrip
extern "C" fn fuzz_artifact_invalid_source(data: &[u8]) -> usize {
    // Decode/Encode ArtifactInvalidSource
    // Verify roundtrip equality
}
```

---

## Section 6 — Kani Harnesses

### process_lock.rs

```rust
// Kani harness: ProcessLock::acquire does not panic
#[kani::proof]
fn process_lock_acquire_no_panic() {
    // Prove: acquire returns Err or Ok, never panics
    let path = kani::any::<PathBuf>();
    let result = ProcessLock::acquire(&path);
    // No panic possible - all paths return Result
}

// Kani harness: read_holder_pid bounds
#[kani::proof]
fn read_holder_pid_bounds() {
    // Prove: return value is None or Some(u32) — always valid
    let file = kani::any::<File>();
    let result = read_holder_pid(&file);
    match result {
        Some(pid) => assert!(pid > 0),
        None => {}, // valid
    }
}
```

### error.rs

```rust
// Kani harness: diagnostic_code never returns zero
#[kani::proof]
fn diagnostic_code_nonzero() {
    let error: JournalError = kani::any();
    let code = error.diagnostic_code();
    assert!(code.as_u32() != 0);
}

// Kani harness: all JournalError variants have display impl
#[kani::proof]
fn all_variants_have_display() {
    let error: JournalError = kani::any();
    let _ = format!("{}", error); // Must not panic
}
```

---

## Section 7 — Mutation Testing Checkpoints

### Target: ≥90% kill rate

| Module | Mutation Target | Kill Mechanism |
|--------|----------------|----------------|
| artifacts.rs | `list_artifacts` — empty Vec case | Integration test verifies `Vec::is_empty` |
| artifacts.rs | `remove_artifact` — not-found case | Unit test expects `ArtifactNotFound` |
| artifacts.rs | `artifact_exists` — false case | Unit test expects `Ok(false)` |
| error.rs | Every variant | `assert!(matches!(...))` for each |
| error.rs | `diagnostic_code` | Proptest invariant `code != 0` |
| process_lock.rs | `read_holder_pid` — None on empty | Unit test with empty file |
| process_lock.rs | `read_holder_pid` — None on parse fail | Unit test with "abc\n" |

---

## Section 8 — Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| list_artifacts empty | empty keyspace | Ok(Vec::new()) | unit |
| list_artifacts 3 items | 3 stored artifacts | Ok(Vec![d1,d2,d3]) | unit |
| list_artifacts corrupted key | malformed key bytes | Err(UnexpectedEof) | unit |
| remove_artifact exists | valid stored digest | Ok(()) | unit |
| remove_artifact absent | unknown digest | Err(ArtifactNotFound) | unit |
| artifact_exists true | stored digest | Ok(true) | unit |
| artifact_exists false | absent digest | Ok(false) | unit |
| acquire success | writable path | Ok(ProcessLock) | unit |
| acquire held | existing lock | Err(ProcessLockHeld) | integration |
| acquire io error | no permission | Err(ProcessLockIo) | unit |
| read_holder_pid valid | "12345\n" | Some(12345) | unit |
| read_holder_pid empty | "" | None | unit |
| read_holder_pid invalid | "abc\n" | None | unit |
| JournalError::Fjall | postcard error | Fjall variant | unit |
| JournalError::DuplicateEvent | run+seq | display contains both | unit |
| JournalError::ProcessLockHeld | path+errno+pid | display contains pid | unit |
| JournalError::ArtifactNotFound | digest | display contains digest | unit |

---

## Section 9 — LETHAL Fixes Required

### LETHAL-1: journal.rs:329 — impl Drop silently discards persist error

**Current Code:**
```rust
impl Drop for FjallJournal {
    fn drop(&mut self) {
        if let Err(e) = self.database.persist(fjall::PersistMode::SyncAll) {
            let _ = e;  // LETHAL: silent discard
        }
    }
}
```

**Required Fix:** Replace with `expect` or logging. Since Drop cannot return Result:
```rust
impl Drop for FjallJournal {
    fn drop(&mut self) {
        if let Err(e) = self.database.persist(fjall::PersistMode::SyncAll) {
            // In debug builds, panic to catch persist failures.
            // In release, log and continue (process is terminating anyway).
            #[cfg(debug_assertions)]
            panic!("FjallJournal persist failed in Drop: {}", e);
            #[cfg(not(debug_assertions))]
            eprintln!("WARN: FjallJournal persist failed in Drop: {}", e);
        }
    }
}
```

**Test:** Add integration test that verifies Drop behavior — cannot directly test Drop in unit, but verify persist path works in normal operation.

### LETHAL-2: process_lock.rs:57,59,100,102 — silent I/O discards

**Current Code (lines 56-59):**
```rust
#[allow(clippy::let_underscore_must_use)]
let _ = file.set_len(0);
#[allow(clippy::let_underscore_must_use)]
let _ = write!(file, "{pid}");
```

**Current Code (lines 99-102):**
```rust
#[allow(clippy::let_underscore_must_use)]
let _ = file.rewind();
#[allow(clippy::let_underscore_must_use)]
let _ = file.read_to_string(&mut buf);
```

**Required Fix:** Replace `let _ =` with proper error handling or explicit best-effort pattern:
```rust
// At line 57: set_len is best-effort — log on failure
if let Err(e) = file.set_len(0) {
    eprintln!("WARN: failed to truncate lock file: {}", e);
}

// At line 59: write is best-effort — log on failure
if let Err(e) = write!(file, "{pid}") {
    eprintln!("WARN: failed to write PID to lock file: {}", e);
}

// At line 100: rewind is best-effort — no-op on failure for read_holder_pid
let _ = file.rewind(); // Best-effort: subsequent read will handle

// At line 102: read_to_string is best-effort — empty buf on failure
if file.read_to_string(&mut buf).is_err() {
    buf.clear(); // Treat as empty on I/O error
}
```

**Test:** Add unit test for `read_holder_pid` that verifies `None` is returned when read fails (can mock via tempfile with read permission removed).

### LETHAL-3: integration test `use crate::` imports

**File: tests/accepted_artifact_red_phase.rs line 226**
```rust
use vb_storage::records::CompiledIrRecord;  // EXTERNAL — OK
```

**File: tests/vb_h6ix_integration.rs lines 13-16**
```rust
use vb_storage::recovery::{  // Should use external crate
    ActionReplayTracker, RecoveryError, extract_terminal, recover_full_journal, replay_events,
};
```

**Required Fix:** Integration tests must use the public crate API, not `crate::`:
```rust
// In tests/vb_h6ix_integration.rs:
use vb_core::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::recovery::{ActionReplayTracker, RecoveryError, extract_terminal, recover_full_journal, replay_events};
use vb_storage::{EventSeq, FjallConfig, FjallJournal, JournalEvent};
```

---

## Section 10 — Coverage Gaps

### artifacts.rs: 0% → 95%

| Test | Function | Assertion |
|------|----------|-----------|
| `artifacts_list_empty` | `list_artifacts` | `assert!(digests.is_empty())` |
| `artifacts_list_with_items` | `list_artifacts` | `assert_eq!(digests.len(), 3)` |
| `artifacts_list_corrupted_key` | `list_artifacts` | `assert!(matches!(result, Err(UnexpectedEof)))` |
| `artifacts_remove_exists` | `remove_artifact` | `assert!(result.is_ok())` |
| `artifacts_remove_not_found` | `remove_artifact` | `assert!(matches!(result, Err(ArtifactNotFound)))` |
| `artifacts_exists_true` | `artifact_exists` | `assert!(result.unwrap())` |
| `artifacts_exists_false` | `artifact_exists` | `assert!(!result.unwrap())` |

### error.rs: 33.85% → 65%

Missing test coverage for these variants:
- `KeyCapacity`
- `WriteLockPoisoned`
- `QueueCapacity`
- `QueueShutdown`
- `WrongRun`
- `SequenceOverflow`
- `RecordKindFamilyMismatch`
- `HeaderLengthMismatch`
- `PayloadDigestMismatch`
- `PostcardDecodeFailed`
- `ArtifactMalformed`
- `ArtifactChecksumMismatch`
- `InvalidGateCount`
- `MissingRequiredProofFlag`
- `AdmissionRequired`
- `ArtifactInvalid`
- `InputTooLarge`
- `InputSchemaMismatch`
- `CapabilityDenied`
- `SecretUnavailable`
- `RunAlreadyExists`
- `ActiveRunCapacityExceeded`
- `FrameAllocationFailed`
- `AdmissionJournalFailed`
- `StrictDurabilityFailed`
- `ClockUnavailable`
- `ProcessLockHeld`
- `ProcessLockIo`
- `Trim`

### process_lock.rs: 44.19% → 80%

Missing test coverage:
- `acquire` with permission denied path
- `acquire` with non-directory path
- `read_holder_pid` with valid PID
- `read_holder_pid` with empty file
- `read_holder_pid` with invalid content

---

## Exit Criteria

- [ ] TEST-PLAN.md written to `/home/lewis/src/Velvet-ballistics/crates/vb_storage/TEST-PLAN.md`
- [ ] Every artifact.rs behavior has a BDD scenario
- [ ] Every process_lock.rs behavior has a BDD scenario
- [ ] Every error.rs variant has a test scenario
- [ ] LETHAL-1 fix specified (journal.rs Drop)
- [ ] LETHAL-2 fix specified (process_lock.rs silent discards)
- [ ] LETHAL-3 fix specified (integration test imports)
- [ ] Proptest invariants defined for error.rs and process_lock.rs
- [ ] Fuzz targets specified for artifacts.rs
- [ ] Kani harnesses specified for process_lock.rs
- [ ] Mutation kill rate target ≥90% stated
- [ ] Coverage matrix complete
