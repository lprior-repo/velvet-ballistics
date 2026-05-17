# Defects — vb-qi37.12.2

STATUS: REJECTED

1. CRITICAL — `ResumeError::JournalAppendFailed` still has no error-bound source. The new `ResumeSourceRegistry` stores sources in a same-thread pending queue and binds them only when `source_runtime_error()` is called (`crates/vb_runtime/src/shard/types.rs:28-36`, `47-64`, `439-460`). A fresh unit `JournalAppendFailed` can steal an unobserved pending source; moves/clones change the pointer key.
2. HIGH — Regression tests drain the pending source before checking stale-source negatives (`crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs:313-384`). They do not test the killer case: sourced error returned but unobserved, then unrelated fresh/converted `JournalAppendFailed` observed first.
3. HIGH — Semver-compatible public API shape still downgrades contract R5. `ResumeError::JournalAppendFailed` remains a unit variant (`crates/vb_runtime/src/shard/types.rs:413-435`), and conversion relies on the same TLS accessor (`crates/vb_runtime/src/error/conversions.rs:21-38`). Passing semver does not prove source causality.
4. MEDIUM — Evidence marks State 11 gates PASS (`.beads/vb-qi37.12.2/machine-gate-report.md:13-18`) but lacks adversarial proof that unrelated same-thread values cannot consume recorded sources.

Mandatory route: replace TLS queue with real error-bound source preservation or narrow the contract, add unobserved-source theft regressions, refresh evidence, rerun State 11, then rerun State 12 black-hat.
