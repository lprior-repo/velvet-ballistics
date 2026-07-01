# Error Taxonomy — vb-7akm0 Lint-Suppression Audit

| Field | Value |
|---|
| bead_id | vb-7akm0 |
| state | 3 (rust-contract) |
| skill | rust-contract |
| captured_at | 2026-07-01T16:04Z |
| upstream_artifacts | domain-model.md, type-contracts.md, workflow-model.md |

## 0. Scope

This file enumerates every error variant the lint-suppression audit domain may produce, classifies each by recoverability, and binds each variant to a stable diagnostic code. Errors come from three sources:

1. **Smart-constructor errors** — type-contract violations at parse time (railway failures).
2. **Workflow engine errors** — illegal state transitions or guard failures (e.g., applying a treatment to a Category G row without a recorded decision).
3. **Lint-anchor errors** — moon ci `lint-src` or `cargo test` failures wrapped into the audit vocabulary.

No error variant below may silently swallow a `core::result::Result::unwrap`-shaped panic or `expect`-shaped failure. The audit core stays pure and never panics on user-supplied input.

## 1. Root Error Type

```rust
/// Top-level error type for the lint-suppression audit domain.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LintAuditError {
    // ── Smart-constructor failures (parse-time, railway) ──
    #[error("repo path parse failed: {0}")]
    RepoPathParse(#[from] RepoPathError),
    #[error("pub item ref construction failed: {0}")]
    PubItemRef(#[from] PubItemRefError),
    #[error("suppression construction failed: {0}")]
    Suppression(#[from] SuppressionError),

    // ── Workflow engine failures ──
    #[error("workflow transition refused: {0}")]
    TransitionRefused(String),
    #[error("category G decision missing for {0}")]
    DecisionMissing(RepoPath),
    #[error("category G decision conflicts with default recommendation")]
    DecisionConflictsDefault { file_path: RepoPath, decision: String },
    #[error("audit trail invariant violated: {0}")]
    AuditTrailInvariant(String),

    // ── Lint-anchor failures (moon ci) ──
    #[error("lint-src reported new unreachable_pub label: {label} at {file_path}:{line}")]
    NewUnreachablePubLabel {
        file_path: RepoPath,
        line: u32,
        label: SmolStr,
        raw_log_path: String,
    },
    #[error("lint-src non-zero exit ({exit_status}) without a new unreachable_pub label")]
    LintSrcNonZeroExit {
        exit_status: i32,
        raw_log_path: String,
    },
    #[error("cargo test --workspace non-zero exit ({exit_status}); consumer {test_path} may have broken")]
    CargoTestNonZeroExit {
        test_path: RepoPath,
        exit_status: i32,
        raw_log_path: String,
    },
    #[error("test regression: previously-passing test {test_path} now fails; expected consumer {expected_consumer}")]
    TestRegression {
        test_path: RepoPath,
        expected_consumer: SmolStr,
        raw_log_path: String,
    },

    // ── Tooling failures ──
    #[error("required tool `{tool}` is not installed")]
    ToolingMissing { tool: String },
    #[error("required tool `{tool}` version {required} does not match observed {observed}")]
    ToolVersionMismatch { tool: String, required: String, observed: String },
}
```

## 2. Diagnostic Codes

Every `LintAuditError` variant maps to a stable diagnostic code so that observability layers (logs, metrics, alerts) can group errors by code without parsing strings. The codes are reserved in the 0xB000–0xB0FF range (distinct from the substrate range 0xA000–0xA0FF used by vb-disri-style closures).

| Variant | Code | Symbolic name |
|---|---|---|
| `RepoPathParse` | 0xB001 | `LINT_AUDIT_REPO_PATH_PARSE` |
| `PubItemRef` | 0xB002 | `LINT_AUDIT_PUB_ITEM_REF` |
| `Suppression` | 0xB003 | `LINT_AUDIT_SUPPRESSION` |
| `TransitionRefused` | 0xB010 | `LINT_AUDIT_TRANSITION_REFUSED` |
| `DecisionMissing` | 0xB011 | `LINT_AUDIT_DECISION_MISSING` |
| `DecisionConflictsDefault` | 0xB012 | `LINT_AUDIT_DECISION_CONFLICTS` |
| `AuditTrailInvariant` | 0xB013 | `LINT_AUDIT_TRAIL_INVARIANT` |
| `NewUnreachablePubLabel` | 0xB020 | `LINT_AUDIT_NEW_UNREACHABLE_PUB` |
| `LintSrcNonZeroExit` | 0xB021 | `LINT_AUDIT_LINT_SRC_NONZERO` |
| `CargoTestNonZeroExit` | 0xB022 | `LINT_AUDIT_CARGO_TEST_NONZERO` |
| `TestRegression` | 0xB023 | `LINT_AUDIT_TEST_REGRESSION` |
| `ToolingMissing` | 0xB030 | `LINT_AUDIT_TOOLING_MISSING` |
| `ToolVersionMismatch` | 0xB031 | `LINT_AUDIT_TOOL_VERSION_MISMATCH` |

## 3. Classification by Recoverability

| Error family | Recoverable? | Recommended response |
|---|---|---|
| Smart-constructor (`RepoPathParse`, `PubItemRef`, `Suppression`) | Yes — fix the input row and re-parse. | Log at `WARN`; do NOT count as a bead failure. |
| Workflow engine (`TransitionRefused`, `DecisionMissing`, `DecisionConflictsDefault`, `AuditTrailInvariant`) | Partially. | `DecisionMissing` is recoverable by recording a decision. `TransitionRefused` indicates an ordering bug; log at `ERROR`. |
| Lint-anchor (`NewUnreachablePubLabel`, `LintSrcNonZeroExit`, `CargoTestNonZeroExit`, `TestRegression`) | Mostly yes. | `NewUnreachablePubLabel` is the normal failure signal of `RunLintSrc` after a partial change; fix the file and re-run. `TestRegression` requires reverting the narrowing OR fixing the consumer; consult the `crate-internal direct path` invariant from `domain-model.md §1`. |
| Tooling (`ToolingMissing`, `ToolVersionMismatch`) | No (in-session). | Block the audit with `BLOCKED_TOOLING`; do NOT advance the bead. |

## 4. Retry Semantics

| Error family | Idempotent retry? | Backoff strategy |
|---|---|---|
| Smart-constructor | No (deterministic — retry with same input gives same error). | None. |
| `DecisionMissing` | Yes (after decision is recorded). | None. |
| `DecisionConflictsDefault` | Yes (after decision is revised to match recommendation, or with explicit override). | None. |
| `NewUnreachablePubLabel` | No. | Fix the offending file and re-run lint-src; otherwise re-run is wasteful. |
| `LintSrcNonZeroExit` (non-label) | No. | Inspect raw log; fix and re-run. |
| `CargoTestNonZeroExit` | No. | Fix the consumer or revert the narrowing. |
| `TestRegression` | No. | Per `domain-model.md §3.3 Test-Suite Re-verify policy`: `cargo test --workspace` MUST exit 0 after each category. |
| `ToolingMissing`, `ToolVersionMismatch` | Yes (after installing / pinning the tool). | None. |

## 5. Mapping to Production Error Sites

| Audit error | Production site (per suppression) |
|---|---|
| `NewUnreachablePubLabel { file_path: "crates/vb_validate/src/..." }` | `moon run :lint-src` exits 1 with stderr containing `error: `pub` item `...` is never used` or `unreachable_pub` (Categories A/B/C/D/E/F/G). |
| `NewUnreachablePubLabel { file_path: "crates/vb_cli/src/commands_diff.rs" }` | `moon run :lint-src` exits 1 after retiring the orphan test (Category G — the orphan test was the only consumer). |
| `TestRegression { test_path: "crates/vb_validate/src/gate_tests.rs" }` | `cargo test -p vb_validate --lib` exits 1 because `use crate::gate_07_stack::validate_gate_07_expression_stack_depth;` no longer resolves (the function is now `fn` but the path is crate-internal — should still work, but verify). |
| `TestRegression { test_path: "crates/vb_validate/src/secret_leak/tests.rs" }` | Same shape as above for `validate_resource_limits`. |
| `TestRegression { test_path: "crates/vb_cli/tests/lifecycle_integration.rs" }` | Should NOT happen — `create_run_header` remains `pub`. |
| `ToolingMissing { tool: "moon" }` | Surfaced when `bash scripts/check-beads-server-mode.sh` or `moon run :lint-src` cannot find `moon` in `$PATH`. |

## 6. Forbidden Patterns

The audit vocabulary forbids the following patterns in implementation code (lifted from AGENTS.md):

| Pattern | Reason |
|---|---|
| `panic!` on a `LintAuditError` | Violates zero-unwrap discipline; degrade gracefully. |
| `unwrap()` or `expect()` on a `LintAuditError` or any inner variant | Violates zero-unwrap rule; surface the error. |
| `LintAuditError::Other(String)` or equivalent catch-all | Defeats the diagnostic-code mapping; every variant must be enumerable. |
| `stringify!(err).contains("foo")` to inspect an error variant | Use `matches!(err, LintAuditError::Variant { .. })` instead. |
| Adding a new `pub` item to vb_validate or vb_cli during this bead | Violates the No-New-Pub policy (`domain-model.md §3.3`); the bead is narrowing only. |

## 7. Sub-Crate Mapping to `vb_core::errors`

The audit-error vocabulary is layered ON TOP OF the existing `vb_core::errors::CoreError` vocabulary, not replacing it. When a lint-anchor failure produces a core error (e.g., a panic in `vb_validate` that surfaces as `CoreError::InternalInvariantViolation`), the audit wrapper preserves the core error as `source` via `#[from]`:

```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LintAuditError {
    #[error("core invariant violated while clearing suppression {file_path}:{line}: {source}")]
    CoreInvariantWrapped {
        file_path: RepoPath,
        line: u32,
        #[source]
        source: vb_core::errors::CoreError,
    },
    // ... other variants
}
```

This wrapping is what allows the audit layer to participate in `vb_core::diagnostic::HasSymbolicCode` without inventing a parallel code registry.

## 8. Open Error-Vocabulary Questions

1. Whether `TestRegression` should carry the full `stderr_excerpt` or just the first line. Recommendation: full excerpt, for forensics; defer to State 5.
2. Whether `NewUnreachablePubLabel` should be split into `NewLabelAtExistingFile` and `NewLabelAtNewFile` (e.g., when a Category B narrowing surfaces a hidden label in a different file). Recommendation: yes, for triage; defer to next iteration.
3. Whether the diagnostic-code range 0xB000–0xB0FF collides with any reserved range in `vb_core::diagnostic::CODE_REGISTRY`. Owner: proof-writer (State 5). Verification: registry dump.

End of error-taxonomy.md.