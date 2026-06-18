# Error Taxonomy — Residue Quarantine CI Gate

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
state: 3 (rust-contract)
skill: rust-contract
attempt: 1-of-7
updated_at: 2026-06-17T22:00:00.000000+00:00

## 1. Error vs Failure

The gate distinguishes between **failure** (the gate's normal fail-closed
behavior when active residue is found) and **error** (a contract violation
that prevents the gate from completing its normal happy path).

| Class | Meaning | Exit code | Stderr prefix |
|-------|---------|-----------|---------------|
| Failure | The gate ran successfully and found active residue. | 1 | `RUNTIME-FMT:` |
| Error | The gate could not complete its happy path. | 2 | `GateError:` |

The moon pipeline treats both classes as failures (the pipeline aborts
either way), but the State 13 black-hat-reviewer and the State 14
evidence-packaging must distinguish them: a failure is the gate
working as designed; an error is a gate bug or a master amendment that
the State 11 implementation has not yet caught up with.

## 2. `GateError` Enumeration

The rust-contract contract binds the following exhaustive error
enumeration. Each variant has a defined exit code and a defined
stderr template.

```
enum GateError {
    PatternFileMissing(String),       // (1) master amendment stale
    GlobUnreadable {                  // (2) filesystem failure
        path: String,
        os_error: String,
    },
    AllowlistParseFailure {           // (3) allowlist file malformed
        line: u32,
        reason: String,
    },
    ScriptInvocationFailure(String),  // (4) catch-all for unhandled panics
    NewResidueDetected,               // (5) sentinel for the happy-path failure
}
```

### 2.1 Variant 1: `PatternFileMissing(String)`

- **Cause:** The master document (`velvet-ballistics-MASTER.md`) does
  not exist, is unreadable, or no longer contains the expected closed
  sets of forbidden imports, hot crates, or cold markers. The
  `ResiduePolicy::from_master` parser failed.
- **Argument:** The forbidden-import name that was being looked up
  when the parser failed. (E.g. `"serde_json"` if the master no
  longer mentions `serde_json` in §2 or §12.)
- **Exit code:** 2 (contract violation).
- **Stderr template:** `GateError:PatternFileMissing: <name>`
- **Mitigation:** The master document is the source of truth. A
  PatternFileMissing error means the master has changed and the
  State 11 implementation must be updated. The State 11 holzman-rust
  agent owns the repair: re-read the master, regenerate the policy.

### 2.2 Variant 2: `GlobUnreadable { path, os_error }`

- **Cause:** A quarantined crate's path is unreadable (e.g., the
  `crates/<name>/src/` directory does not exist or has been replaced
  with a file). The `SourceTreeWalker::walk` function failed.
- **Arguments:** `path` is the relative path that was unreadable
  (e.g., `"crates/vb_core/src"`); `os_error` is the formatted
  `std::io::Error`.
- **Exit code:** 2 (contract violation).
- **Stderr template:** `GateError:GlobUnreadable: <path>: <os_error>`
- **Mitigation:** A GlobUnreadable error means the repository layout
  has changed. The State 11 holzman-rust agent must re-derive the
  quarantined-crate set from the master document. The contract
  binds the closed set of hot crate names; adding a new crate
  requires a master amendment.

### 2.3 Variant 3: `AllowlistParseFailure { line, reason }`

- **Cause:** The `scripts/forbid-runtime-fmt.allow` file is
  unreadable, contains a malformed line, or contains a duplicate
  allowlist key. The `AllowlistRef::load` parser failed.
- **Arguments:** `line` is the 1-indexed line number where the
  parse failed; `reason` is a human-readable description of the
  parse error (e.g., `"expected 7 pipe-separated fields, got 5"`,
  `"unknown forbidden name 'serde_jsonx'"`, `"duplicate key
  crates/vb_core/src/foo.rs|42|serde_json"`).
- **Exit code:** 2 (contract violation).
- **Stderr template:** `GateError:AllowlistParseFailure: line <line>: <reason>`
- **Mitigation:** The allowlist file is human-edited; the State 9
  test-writer and the State 11 holzman-rust agent are responsible
  for keeping it well-formed. A AllowlistParseFailure error
  surfaces immediately in the moon pipeline and must be fixed
  before the bead can be landed.

### 2.4 Variant 4: `ScriptInvocationFailure(String)`

- **Cause:** A catch-all for unhandled panics, uncaught `Result::Err`
  values, or unexpected control flow that does not map to the other
  variants. The bash `trap` catches the panic and translates it to
  this error.
- **Argument:** A human-readable description of the failure (e.g.,
  the panic message).
- **Exit code:** 2 (contract violation).
- **Stderr template:** `GateError:ScriptInvocationFailure: <reason>`
- **Mitigation:** A ScriptInvocationFailure error means the gate
  has a bug. The State 13 black-hat-reviewer must report it; the
  State 11 holzman-rust agent must fix it.

### 2.5 Variant 5: `NewResidueDetected`

- **Cause:** The gate ran successfully and found at least one
  forbidden import that is not in the allowlist. This is the
  gate's normal fail-closed behavior.
- **Argument:** None (the actual residue matches are emitted as
  separate `RUNTIME-FMT:` lines, not as part of the error).
- **Exit code:** 1 (gate failure, not contract violation).
- **Stderr template:** None. The stderr is the per-match lines
  followed by the summary line.
- **Mitigation:** The developer must remove the forbidden import
  or add an allowlist entry. Adding an allowlist entry requires
  the entry to satisfy the allowlist format and to cite `owner=`,
  `reviewed_by=`, `test=`, and `reason=` fields. The State 14
  evidence-packaging agent reviews the allowlist for drift.

## 3. Exit Code Mapping

| Decision / Error | Exit code | Meaning |
|------------------|-----------|---------|
| `GateDecision::Pass` | 0 | No active residue. The gate is clean. |
| `GateDecision::Fail(active)` | 1 | Active residue found. The gate failed. |
| `GateError::NewResidueDetected` | 1 | (Sentinel for the same condition.) |
| `GateError::PatternFileMissing` | 2 | Master amendment stale. |
| `GateError::GlobUnreadable` | 2 | Filesystem failure. |
| `GateError::AllowlistParseFailure` | 2 | Allowlist malformed. |
| `GateError::ScriptInvocationFailure` | 2 | Unhandled panic / bug. |
| Bash pre-flight failure (wrong cwd) | 64 | InvalidInvocation (matches sibling gate convention). |

The exit code is the **only** signal `moon ci` reads. The stderr is
informational. The bash wrapper MUST use the exit code table above
exactly; a `chmod +x` bug or a `set -e` bug that causes a non-mapped
exit code is a contract violation.

## 4. Stderr Templates

### 4.1 Active residue line

Format:
```
<file>:<line_no>: RUNTIME-FMT: <forbidden_name>: <snippet>
```

Example:
```
crates/vb_core/src/action.rs:42: RUNTIME-FMT: serde_json: use serde_json;
```

The format is a contract. Changes to the format are breaking changes
to `test-forbid-runtime-fmt.sh`.

### 4.2 Allowlisted residue line

Format:
```
<file>:<line_no>: allowlisted: <reason>: <snippet>
```

Example:
```
crates/vb_core/tests/proptest_serde_roundtrip.rs:17: allowlisted: dev-dep test only: let _v: serde_json::Value = serde_json::from_str("null").unwrap_or_default();
```

### 4.3 Final summary line

Format:
```
summary: active=<N> allowlisted=<M> files_scanned=<K> hot_paths=<H> cold_paths=<C>
```

Example:
```
summary: active=0 allowlisted=2 files_scanned=98 hot_paths=82 cold_paths=16
```

The summary line is emitted exactly once per gate execution, on
stdout when the gate passes and on stderr when the gate fails.

### 4.4 Error template

Format (per variant, see §2):
```
GateError:<VariantName>: <args>
```

Example:
```
GateError:AllowlistParseFailure: line 7: expected 7 pipe-separated fields, got 5
```

### 4.5 Stderr / stdout split

| Output | Channel | When |
|--------|---------|------|
| Active residue line | stderr | always, when emitted |
| Allowlisted residue line | stderr | always, when emitted |
| Summary line | stdout (pass) / stderr (fail) | exactly once per execution |
| Error template | stderr | exactly once per error |
| Empty | stdout | if the gate fails before any line is emitted (e.g., `PatternFileMissing`) |

The split is a contract. Tests assert against it.

## 5. Error Recovery

The gate has no error-recovery sub-workflow. Every error terminates
the gate and exits non-zero. The moon pipeline aborts.

The following errors are *recoverable* from the developer's
perspective (the developer fixes the underlying problem and reruns
the gate):

- `NewResidueDetected` (exit 1) — remove the forbidden import or add
  an allowlist entry.
- `AllowlistParseFailure` (exit 2) — fix the allowlist file format.

The following errors are *non-recoverable* from the developer's
perspective and require a master amendment + a code update:

- `PatternFileMissing` (exit 2) — the master document has changed;
  the State 11 implementation must be updated.
- `GlobUnreadable` (exit 2) — the repository layout has changed;
  the State 11 implementation must be updated.

`ScriptInvocationFailure` (exit 2) is a bug; it is not recoverable
in any way other than fixing the bug.

## 6. Error Visibility

| Error | Where it appears in `moon ci` output |
|-------|--------------------------------------|
| `NewResidueDetected` | `task:forbid-runtime-fmt: stderr` line "RUNTIME-FMT: ..."; exit code 1 |
| `PatternFileMissing` | `task:forbid-runtime-fmt: stderr` line "GateError:PatternFileMissing: ..."; exit code 2 |
| `GlobUnreadable` | `task:forbid-runtime-fmt: stderr` line "GateError:GlobUnreadable: ..."; exit code 2 |
| `AllowlistParseFailure` | `task:forbid-runtime-fmt: stderr` line "GateError:AllowlistParseFailure: ..."; exit code 2 |
| `ScriptInvocationFailure` | `task:forbid-runtime-fmt: stderr` line "GateError:ScriptInvocationFailure: ..."; exit code 2 |

The developer sees the stderr line at the top of the failed moon
task. The State 14 evidence-packaging captures the full stderr into
`assurance-bundle.md`.

## 7. Out-of-Scope Error Classes

The following error classes are explicitly NOT in this gate's error
taxonomy:

- **Runtime panic during cargo build:** the gate does not run
  `cargo build`. The scanner is compiled once by the bash wrapper
  and run as a standalone binary. A `rustc` failure is caught by
  the bash `set -e` and translated to exit 2 with a bash error
  message, not a `GateError:*` line.
- **Moon task not found:** the gate does not call `moon run`
  recursively. The moon pipeline handles missing-task errors.
- **Network failure:** the gate makes no network calls.
- **File system permission failure (read-only file):** the gate's
  `is_cold_path` classifier silently skips a cold path with a read
  error, but a hot path with a read error is `GateError::GlobUnreadable`
  (which is a different variant from a missing-path error).
- **Bash variable unset:** caught by `set -u` in the wrapper; the
  bash error message is not a `GateError:*` line.

## 8. Test-Facing Error Contract

The three contract tests in `test-forbid-runtime-fmt.sh` use the
following error-class assertions:

| Test name | Expected exit code | Expected stderr prefix |
|-----------|--------------------|-------------------------|
| `test_quarantine_gate_blocks_json_import` | 1 | `RUNTIME-FMT: serde_json:` |
| `test_quarantine_gate_blocks_unbounded_channel` | 1 | `RUNTIME-FMT: tokio::sync::mpsc::unbounded:` |
| `test_moon_ci_quarantine_dependency_correctly_ordered` | 0 | (no error expected; success assertion) |

The tests assert against the *prefix*, not the full line. This
allows the snippet to vary (different fixture contents) while the
gate's behavior is unchanged.
