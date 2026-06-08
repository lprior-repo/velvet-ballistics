# Round 4 Agent A6 — Source-Length Drift (CRITICAL)

**Reviewer:** black-hat-reviewer · **STATUS: REJECTED — SHIP-BLOCKER · 87/100**

`moon :source-length` returns non-zero. The full pipeline is RED on two counts: (1) seven production-or-test source files exceed 300 lines without a valid exception row; (2) two stale exception rows reference files that have already been split.

## Per-File Verdict Table

| # | File | Lines | Production/Test | Verdict |
|---|---|---:|---|---|
| 1 | `crates/vb_cli/src/cli_postcard/tests.rs` | 751 | Test | LOW (test) |
| 2 | `crates/vb_core/src/workflow/compiled_slug.rs` | 583 | **Production canonical** | **CRITICAL** — no row in ledger |
| 3 | `crates/vb_cli/src/cli_postcard/types.rs` | 530 | Production | HIGH — unlisted |
| 4 | `crates/vb_core/tests/vb_ajc40_public_decode_regression.rs` | 600 | Test | MEDIUM |
| 5 | `crates/workspace_tests/tests/vb_a7t6_3_instruction_count_tests.rs` | 317 | Test | LOW |
| 6 | `crates/vb_ipc/src/server/dispatch_tests.rs` | 302 | Test | LOW |
| 7 | `crates/vb_cli/src/output.rs` | 303 | Production | MEDIUM — 4 concerns in one file |

## Worst Offender: compiled_slug.rs

It is the **only** file among the 7 violations that is a **canonical production seam** cited by 6 Flux refinements, 3 fuzz targets, 2 Kani harnesses, and 8 proptest modules — and yet has *no* row in the 497-row exception ledger.

## Holzman Rust Violations in output.rs

`infer_legacy_json_error_code` (lines 244-265) is a `String`-substring matcher (`if message.contains("journal")` etc.) that picks an exit code from arbitrary substring matches. This is exactly the "stringly errors" anti-pattern § 3 forbids.

`from_envelope_kind` (cli_postcard/types.rs:70-103) returns `Option` from string keys. The `GenericPayload` migration-fallback re-introduces an unverified `Vec<u8>` body that can silently re-enable the JSON-in-postcard bridge.

## The Bitter Truth

The exception ledger is the *de facto* gate, and `compiled_slug.rs` is not in it. A file is invisible to the gate if it has no exception row.

The 50+ rows carrying `split-or-retire-before-release` (owned by beads vb-wsamy, vb-lhtnj, vb-5iebh, vb-2lu1, vb-9kwz) have **not produced a split**.

The drift has no natural pressure to refactor because splitting compiled_slug.rs requires touching all 19 downstream proof artifacts.

## Required Repair Actions

1. **CRITICAL**: Add `compiled_slug.rs` to the exception ledger OR split the file (extract `mod tests` → `tests.rs`, then split production into `compiled_slug_types.rs` + `compiled_slug_validation.rs`).
2. **HIGH**: Delete `.config/source-length-exceptions.txt:436` (stale `verification/proptest/properties.rs`) and `:493` (stale `crates/vb_ipc/src/server/dispatch.rs`).
3. **HIGH**: Register `crates/vb_cli/src/cli_postcard/types.rs`, `crates/vb_cli/src/output.rs`, `crates/vb_cli/src/cli_postcard/tests.rs`, `crates/vb_core/tests/vb_ajc40_public_decode_regression.rs`, `crates/workspace_tests/tests/vb_a7t6_3_instruction_count_tests.rs`, and `crates/vb_ipc/src/server/dispatch_tests.rs` in the exception ledger with explicit owner + bead + removal plan, OR split each under 300 lines.
4. **MEDIUM**: Delete `infer_legacy_json_error_code` in `output.rs:244-265`; replace its call sites with direct `CliExitCode` values.
5. **MEDIUM**: Replace stringly-typed `from_envelope_kind` in `cli_postcard/types.rs:70-103` with an exhaustive `try_from_envelope_kind` (or a closed enum bound to the `Kind` family).
6. **STRUCTURAL**: Add a self-test to the source-length gate that asserts the count of `split-or-retire-before-release` rows is monotonically non-increasing per quarter. Without this, the gate is structurally permanent.

## Verdict: SHIP-BLOCKER

The 300-line file cap is a hard CI gate by master-contract mandate. The gate is currently failing on 9 violations. The single highest-impact file (`compiled_slug.rs`) is the only long file in its module that has no row. Two real Holzmann-matrix violations are buried in the drift. The de facto policy is "register and forget", and there is no natural pressure to refactor.
