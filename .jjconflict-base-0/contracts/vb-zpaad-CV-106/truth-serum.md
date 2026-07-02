# Truth-Serum Audit — vb-zpaad (CV-106)

**Bead:** vb-zpaad (bug-hunt CV-106 follow-up).
**Pipeline caveat:** self-authored by orchestrator (no subagent tool
exposed).

## Audit Posture

Dual-persona audit: an **Accuser** that assumes the orchestrator is
hallucinating and looks for evidence of fakery, and a **Defender**
that cross-references every claim against raw outputs.

## Audit Verdict

**PASS.** All claims in the contract, proof plan, proof review, test
plan, test review, and black-hat review are cross-verifiable against
the raw command outputs under `.evidence/vb-zpaad/`. No summary is
substituted for raw output. No exit code is omitted.

## Claim-by-Claim Cross-Verification

### Claim 1: `Span::try_new(start, end) -> Result<Span, SpanError>`

**Verification:** `rtk rg "pub const fn try_new" crates/vb_core/src/span.rs`
returns exactly one match in
`crates/vb_core/src/span.rs:43:5`. **Confirmed.**

### Claim 2: `SpanError` enum is `#[non_exhaustive]`

**Verification:** `rtk read crates/vb_core/src/span.rs` shows
`#[non_exhaustive] pub enum SpanError` at line 62. **Confirmed.**

### Claim 3: `From<SpanError> for CoreError` impl exists

**Verification:** `rtk rg "impl From<SpanError> for CoreError" crates/`
returns exactly one match in `crates/vb_core/src/errors.rs:770`.
**Confirmed.**

### Claim 4: `CoreError::InvalidSpan` variant is added

**Verification:** `rtk rg "InvalidSpan \{$" crates/vb_core/src/errors.rs`
shows the variant definition at line 521 and the diagnostic_code
match arm at line 720. **Confirmed.**

### Claim 5: `CoreError::INVALID_SPAN_CODE = 0x130E` is registered

**Verification:** `rtk rg "INVALID_SPAN_CODE: DiagnosticCode = DiagnosticCode::new\(0x130E\)" crates/`
returns exactly one match. **Confirmed.** Code 0x130E was
double-checked to be unused in the diagnostic registry and the
existing accessor range (0x1311-0x1315). **Confirmed.**

### Claim 6: Kani harness file is wired into the kani module

**Verification:** `rtk rg "pub mod kani_span_try_new" crates/vb_core/src/kani/mod.rs`
returns exactly one match. **Confirmed.**

### Claim 7: proptest file is in the correct location

**Verification:** `rtk ls crates/vb_core/tests/ | grep proptest_span_try_new`
returns the filename. **Confirmed.**

### Claim 8: All four new Kani harnesses verify

**Verification:** `.evidence/vb-zpaad/kani/*.log` contains the raw
output of four `cargo kani ... --harness <name>` invocations. Each
log ends with `VERIFICATION:- SUCCESSFUL` and
`Complete - 1 successfully verified harnesses, 0 failures, 1 total.`
**Confirmed.**

### Claim 9: All seven new proptest cases pass

**Verification:**
`.evidence/vb-zpaad/tests/proptest_span_try_new.log` shows
`running 7 tests ....... test result: ok. 7 passed; 0 failed`.
**Confirmed.**

### Claim 10: All 32 inline span tests pass

**Verification:**
`.evidence/vb-zpaad/tests/inline_span_tests.log` shows
`running 32 tests ................................ test result: ok. 32 passed; 0 failed`.
**Confirmed.**

### Claim 11: The full workspace nextest run passes

**Verification:**
`.evidence/vb-zpaad/tests/workspace_nextest.log` shows
`13842 tests run: 13842 passed, 39 skipped`. **Confirmed.**

### Claim 12: Clippy with the full deny set passes

**Verification:** `.evidence/vb-zpaad/lint/clippy.log` exists; the
command that produced it exited 0 (verified in the shell). The log
is empty of warnings. **Confirmed.**

### Claim 13: The pre-existing `kani_from_str_rejects_unsupported`
              harness is broken on `main` (not by this bead)

**Verification:** `rtk git stash` → re-run harness → same failure.
The list has 33 entries, the unwind bound is 30. This is a
pre-existing harness bug, documented in `black-hat-review.md`
Axis 6.1. **Confirmed.**

### Claim 14: The fmt mismatch in `vb_runtime/...` is pre-existing

**Verification:** `rtk git stash` → `cargo fmt --all --check` → same
mismatch in `crates/vb_runtime/src/shard/types.rs` and
`crates/vb_runtime/src/error/equality.rs`. **Confirmed.**

### Claim 15: The commit message will follow the format
              `bead vb-zpaad: CV-106 <short description>`

**Verification:** Pending — this audit runs before the commit. The
defender commits to the format. The accuser will re-audit after
the commit. **Provisional.**

## Hallucination Check

| Hallucination risk                                | Mitigation                                              |
|---------------------------------------------------|---------------------------------------------------------|
| "The kani harness passes" without raw output      | Each log is captured under `.evidence/vb-zpaad/kani/`.  |
| "All tests pass" without test count               | Log shows `13842 tests run: 13842 passed, 39 skipped`.  |
| "Clippy clean" without lint output                | Log captured; exit 0 verified.                          |
| "Diagnostic code 0x130E is unused" without check  | Cross-checked against production registry and the existing kani harness's unsupported list. |
| "`Span::try_new` is `const`" without checking     | Function signature is `pub const fn try_new`.           |
| "SpanError is `#[non_exhaustive]`" without check  | Attribute is present at line 62.                        |
| "From<SpanError> for CoreError compiles" without running tests | Workspace test run (13,842 tests) covers the conversion. |
| "Span::new is unchanged" without test             | Regression test `try_new_preserves_existing_new_semantics` and proptest `new_is_unchanged` and Kani `kani_span_new_unchanged` all confirm. |

## Self-Approval Risk

The pipeline as designed uses independent subagents to author
contract, proof plan, proof artifacts, test plan, and reviews, then
a black-hat reviewer cross-checks. In this self-authored run, all
of those artifacts are produced by the orchestrator. The accuser
flagged this risk at the start of the bead; the user explicitly
approved the self-authoring posture.

To compensate, this audit:
1. Cross-references every claim against raw output.
2. Re-runs the gates (`cargo test`, `cargo nextest`, `cargo clippy`,
   `cargo kani`) and records exit codes.
3. Documents the pre-existing harness failure
   (`kani_from_str_rejects_unsupported`) rather than hiding it.

## Self-Authoring Marker

This truth-serum audit is self-authored by the orchestrator, not by
a `truth-serum` subagent, because the runtime does not expose a
subagent tool. The content is the audit the `truth-serum` skill
would have produced given the artifacts and the raw evidence.
