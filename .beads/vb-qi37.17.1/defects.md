# Defects — vb-qi37.17.1: cli: Add incident command

## DEFECT-001 (Medium)

**Location**: `crates/vb_cli/src/app_impl.rs` lines 3191, 3207
**Description**: `cmd_incident` returns `CliExitCode::RuntimeFailed` for JSON serialization failures. Contract (POST-004 and contract "Detailed fix" section lines 186–193, 200–207) specifies `CliExitCode::StorageError`.
**Evidence**:
```rust
// Line 3191 (Json format serialization error handler)
return CliExitCode::RuntimeFailed.into();

// Line 3207 (Jsonl format serialization error handler)
return CliExitCode::RuntimeFailed.into();
```
Contract template fix:
```rust
return CliExitCode::StorageError.into();
```
**Fix**: Replace `RuntimeFailed` with `StorageError` on lines 3191 and 3207.
**Route**: holzman-rust (State 10)

---

## DEFECT-002 (Medium)

**Location**: `crates/vb_cli/tests/vb_qi37_17_1_incident_command.rs` lines 163–182 (T-016)
**Description**: T-016 verifies JSON report fields for a successful (non-failed) run but does NOT assert the exit code. Contract POST-004 mandates: "Exit code is `CliExitCode::Success` when `failure_found` is true; `CliExitCode::StorageError` otherwise."
**Evidence**:
```rust
// Current assertions (lines 180–181):
assert_eq!(json["failure_code"].as_str(), Some(""));
assert_eq!(json["failed_at_step"], serde_json::Value::Null);
// Missing: exit code assertion
```
**Fix**: Add to T-016:
```rust
assert_eq!(output.status.code(), Some(5),
    "non-failed run should return StorageError (exit code 5)");
```
**Route**: test-writer (State 9)

---

## DEFECT-003 (Medium)

**Location**: `crates/vb_cli/tests/vb_qi37_17_1_incident_command.rs` line 144 (T-015)
**Description**: T-015 tests the structured error output for a non-existent run but does NOT assert that the output contains no stack trace text. Contract POST-003 requires: "no stack traces or raw error details." INV-002 requires: "never includes std::backtrace::Backtrace, debug formatting of JournalError, or any stack-trace-producing display." Test plan section 3 explicitly states the grep requirement.
**Evidence**:
```rust
// Current T-015 assertions (lines 149–154):
let json: serde_json::Value = serde_json::from_str(&stderr).expect("valid JSON on stderr");
assert_eq!(json["code"], "ValidationFailed");
assert_eq!(json["kind"], "DiagnosticReport");
assert!(json["message"].as_str().unwrap_or("").contains("no events"));
// Missing: stack trace absence assertion
```
**Fix**: Add after line 154:
```rust
let stderr_str = String::from_utf8_lossy(&output.stderr);
assert!(!stderr_str.to_lowercase().contains("backtrace"),
    "error output must not contain stack traces");
assert!(!stderr_str.contains("at crates/"),
    "error output must not contain source location traces");
```
**Route**: test-writer (State 9)

---

## DEFECT-004 (Low)

**Location**: `contract.md` line 100 ("56 E0061 compile errors") and line 13 ("~37 call sites")
**Description**: Contract states 56 compile errors; implementation report states 57. The discrepancy is 1 error — likely a miscount during contract authoring. Implementation report lists 30 (recover_full_journal) + 22 (replay_events) + 1 (replay_journal) = 53 explicit call sites, plus merge conflict resolution.
**Fix**: Update contract.md INV-005 and surrounding text to reference 57 compile errors consistently.
**Route**: contract (State 3)
