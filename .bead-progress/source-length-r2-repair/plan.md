# Implementation Plan: `moon :source-length` Gate Repair (Round 2)

**Bead candidate:** `vb-source-length-r2`
**Owner:** lewis
**Date:** 2026-06-07
**Status:** Plan — not yet claimed

## Scope and Authority

This plan repairs the `moon :source-length` gate to exit 0 and prevents
future drift. It is the bead-ready successor to research at
`.bead-progress/velvet-ballistics-20260524223121-uuf9dni9/`.

Authority chain (per `AGENTS.md`):
1. `velvet-ballistics-MASTER.md` (line 1812 — `source-length` is a
   canonical CI task)
2. `scripts/source_length_gate.rs`, `scripts/source_length_ledger.rs`,
   `scripts/source_length_scan.rs` (the gate implementation)
3. `.config/source-length-exceptions.txt` (the exception ledger)

The gate counts physical lines per tracked `*.rs` file (limit 300) and
physical lines per hot runtime function (limit 25 logical lines). The
ledger is the only waiver mechanism; rows must use
`<file_path>|<owner>|<split_bead>|<removal_plan>|<reason>` and the file
must currently exceed 300 lines or the row is rejected as **stale**.

## Verified State (Round 2)

Gate currently fails. From `git ls-files '*.rs'` and
`scripts/source_length_gate.rs` we have:

### 7 over-limit files (no valid ledger row, or row missing/stale)

| # | File | Lines | Type | Category |
|---|------|------:|------|----------|
| 1 | `crates/vb_cli/src/cli_postcard/tests.rs` | 751 | test | cold |
| 2 | `crates/vb_cli/src/cli_postcard/types.rs` | 530 | production | other |
| 3 | `crates/vb_cli/src/output.rs` | 303 | production | other; Holzman §3 violation |
| 4 | `crates/vb_core/src/workflow/compiled_slug.rs` | 583 | production (canonical) | other |
| 5 | `crates/vb_core/tests/vb_ajc40_public_decode_regression.rs` | 600 | test | cold |
| 6 | `crates/vb_ipc/src/server/dispatch_tests.rs` | 302 | test | cold |
| 7 | `crates/workspace_tests/tests/vb_a7t6_3_instruction_count_tests.rs` | 317 | test | cold |

### 2 stale ledger rows (file no longer over-limit)

- Line 436: `verification/proptest/properties.rs|lewis|vb-2lu1|…`
  — claimed 369 lines; actual 94 lines.
- Line 493: `crates/vb_ipc/src/server/dispatch.rs|lewis|vb-5iebh|…`
  — claimed 332 lines; actual 66 lines.

The stale-row branch is in `scripts/source_length_ledger.rs:31-39`:
rows where the file is `<= limit` are rejected as `stale exception`.
Both rows are now stale; the gate emits two errors on the line numbers
above and exits non-zero.

### Underlying concern smuggling in `output.rs`

`output.rs` (303 lines) packages four concerns plus a Holzman §3
substring matcher:

1. **Output format flag parsing** (lines 44–66):
   `output_format_from_args`, `named_os_flag`, `parse_emit_output_format`.
2. **IO primitives** (lines 12–42, 113–187):
   `write_structured_stderr`, `write_stderr_bytes`,
   `write_stderr_line_io`, `write_stdout_line*`, `write_stderr_best_effort`.
3. **Postcard envelope encoding** (lines 141–166, 294–300):
   `encode_typed_postcard_frame`, `encode_postcard_envelope_value`,
   `write_typed_postcard_diagnostic_stderr`.
4. **JSON envelope dispatch** (lines 189–281):
   `json_out`, `write_contract_error_json`, `json_error`,
   `legacy_json_error_message`, `write_diagnostic_message_stderr`,
   `write_yaml_diagnostic_stderr`.
5. **Holzman §3 violation** (lines 244–265): `infer_legacy_json_error_code`
   recovers a `CliExitCode` by substring matching on an error message.
   The same code is duplicated in `output_utils.rs:91-112` (concern
   smuggling across `output.rs` and `output_utils.rs`).

The substring matcher must be deleted; the typed `CliExitCode` must be
constructed at the error origin and threaded through `json_error` as a
parameter, eliminating the legacy inference path entirely.

## Per-File Decision and Split Plan

For every file, **split under 300 lines is preferred**. Exception rows
are acceptable only for production files that have been audited and
cannot split cleanly; tests are not eligible for exception rows because
they are not on the hot path and splitting is mechanical.

### 1. `crates/vb_cli/src/cli_postcard/tests.rs` (751 lines, test) — **split**

**Decision:** Convert `cli_postcard/tests.rs` into a directory
`cli_postcard/tests/` with one shared helper module and four
concern-split test modules. `mod.rs` of `cli_postcard` already wires
`tests` via `#[path = "tests.rs"] mod tests;`; switch to
`#[path = "tests/mod.rs"] #[cfg(test)] mod tests;`.

**New files:**

| New path | Lines (est.) | Contains |
|----------|-------------:|----------|
| `cli_postcard/tests/mod.rs` | ~12 | `mod` declarations, common `use` |
| `cli_postcard/tests/support.rs` | ~25 | `encode_test_postcard`, `write_test_header_prefix`, `write_test_bytes` (lines 733–751 of source) |
| `cli_postcard/tests/header_decode.rs` | ~190 | `test_valid_magic`, `test_max_payload`, `test_header_size`, `test_postcard_header_from_bytes`, `test_decode_valid_postcard`, `test_decode_invalid_magic`, `test_decode_payload_too_large`, `test_decode_invalid_header_length`, `test_decode_data_too_short`, `test_encode_postcard`, `test_roundtrip`, `decode_rejects_corrupted_crc_before_exposure`, `decode_rejects_corrupted_digest_before_exposure`, `decode_rejects_old_and_future_versions`, `decode_rejects_wrong_kind`, `decode_rejects_max_plus_one_payload_before_exposure`, `decode_rejects_truncated_header`, `decode_cli_payload_rejects_garbage_bytes_as_typed_envelope` (lines 14–201) |
| `cli_postcard/tests/typed_roundtrip.rs` | ~300 | `typed_diagnostic_payload_round_trips`, `typed_validate_payload_round_trips`, `typed_verify_payload_round_trips`, `typed_explain_payload_round_trips`, `typed_events_payload_round_trips`, `typed_trace_payload_round_trips`, `typed_replay_payload_round_trips`, `typed_diff_payload_round_trips` (lines 203–500) |
| `cli_postcard/tests/classify_envelope.rs` | ~140 | `classify_envelope_routes_validate_report_to_typed_variant`, `classify_envelope_routes_unknown_kind_to_unknown_kind_error`, `classify_envelope_fails_on_missing_kind_field`, `classify_envelope_falls_back_to_generic_for_unmapped_typed_kinds` (lines 501–623) |
| `cli_postcard/tests/from_envelope_kind.rs` | ~110 | `cli_postcard_kind_from_envelope_kind_resolves_known_kinds_and_returns_none_for_unknown`, `from_envelope_kind_impl_for_envelope_kind_covers_all_variants`, `typed_postcard_wire_format_carries_typed_bool_not_string` (lines 624–731) |

**Verification:** `cargo nextest run -p vb_cli --all-features` runs the
same `#[test]` functions, just split across modules; `#[test]`
discovery is path-agnostic. `moon :source-length` exits 0 because no
file in the new tree exceeds 300 lines.

**Hours: 6** (4 concern modules + 1 support + plumbing; 1h re-test +
1h rewire).

---

### 2. `crates/vb_cli/src/cli_postcard/types.rs` (530 lines, production) — **split**

**Decision:** Convert `cli_postcard/types.rs` into a directory
`cli_postcard/types/` with a slim `mod.rs` and six sub-modules by
payload family. `cli_postcard/mod.rs` already re-exports the entire
`types` surface (lines 28–34); the re-export block is unchanged.

**New files:**

| New path | Lines (est.) | Contains |
|----------|-------------:|----------|
| `cli_postcard/types/mod.rs` | ~50 | constants (`CLI_MAGIC`, `MAX_PAYLOAD`, `HEADER_SIZE`, …), `CliPostcardKind` enum + `impl`, `From<EnvelopeKind>`, `EnvelopeSchemaVersion` newtype + `impls` |
| `cli_postcard/types/header.rs` | ~50 | `PostcardHeader` struct + `validate` + `from_bytes` (lines 480–529) |
| `cli_postcard/types/diagnostic.rs` | ~25 | `DiagnosticReport` + `from_code` (lines 159–181) |
| `cli_postcard/types/verify.rs` | ~95 | `VerifyReport`, `VerifyReplaySection`, `VerifyArtifactSection`, `VerifyDurabilitySection`, `verify_kind` (lines 200–246) |
| `cli_postcard/types/explain.rs` | ~80 | `ExplainReport`, `ExplainErrorEntry`, `ExplainArtifactSection`, `explain_kind` (lines 248–296) |
| `cli_postcard/types/events_trace.rs` | ~80 | `EventsReport`, `EventEntry`, `TraceReport`, `TraceEntry`, `events_kind`, `trace_kind` (lines 298–358) |
| `cli_postcard/types/replay_diff.rs` | ~110 | `ReplayReport`, `DiffReport`, `DiffEntry`, `replay_kind`, `diff_kind` (lines 360–408) |
| `cli_postcard/types/payload.rs` | ~70 | `CliPostcardPayload` enum, `GenericPayload`, `impl CliPostcardPayload` (lines 410–478) |

**Verification:** `cargo check -p vb_cli --all-features`; existing
`use crate::cli_postcard::CliPostcardKind` paths keep compiling
because `mod.rs` re-exports the public types. `moon :source-length`
exits 0.

**Hours: 8** (7 new modules + re-export plumbing; 3h careful audit of
`mod.rs` because the surface is large; 1h re-test).

---

### 3. `crates/vb_cli/src/output.rs` (303 lines, production) — **fix Holzman §3 + split**

**Decision:** Both. Delete the substring matcher at lines 244–265
(`infer_legacy_json_error_code`) and its duplicate in
`output_utils.rs:91-112`; thread a typed `CliExitCode` parameter
through `json_error`. Then split the four concerns into separate
files under `output/`.

**Holzman §3 fix — call graph:**

```
json_error(value: &serde_json::Value, format: OutputFormat)               # output.rs:222
  ├─ let code = infer_legacy_json_error_code(&message);                   # HOLZMAN VIOLATION
  └─ write_diagnostic_message_stderr(&message, code, format);             # uses typed code
```

Every existing call site of `json_error(value, format)` knows the
correct `CliExitCode` at the construction site. The fix is to
introduce a sibling API:

```rust
pub(crate) fn json_error_with_code(
    value: &serde_json::Value,
    code: CliExitCode,
    format: OutputFormat,
);
```

Each of the 50+ call sites (in `action_specs.rs`, `run_ops.rs`,
`incident_diff.rs`, `events.rs`, `verify.rs`, `submit.rs`,
`run_compiled.rs`, `commands_ai_context.rs`, `incident_ops.rs`, etc.)
already constructs or imports a typed failure value; the existing
local `CliExitCode::*` binding is reused. The legacy
`json_error(value, format)` becomes a private deprecated helper that
is removed once every call site is migrated.

**Split — new files:**

| New path | Lines (est.) | Contains |
|----------|-------------:|----------|
| `output/mod.rs` | ~12 | `mod` declarations; re-export the public API for existing `use crate::output::…` paths |
| `output/format.rs` | ~30 | `output_format_from_args`, `named_os_flag`, `parse_emit_output_format` (lines 44–66) |
| `output/io.rs` | ~90 | `write_structured_stderr`, `write_stderr_bytes`, `write_stderr_line_io`, `write_stdout_line*`, `write_stderr_line`, `write_stderr_best_effort` (lines 12–42, 113–187) |
| `output/json.rs` | ~120 | `OutputError`, `output_error_exit`, `json_out_exit`, `json_out`, `write_contract_error_json`, `json_error_with_code`, `legacy_json_error_message` (kept for one release), `write_diagnostic_message_stderr`, `write_yaml_diagnostic_stderr`, `write_typed_postcard_diagnostic_stderr`, `encode_typed_postcard_frame`, `encode_postcard_envelope_value` (lines 68–111, 141–166, 189–300) |
| `output/compat.rs` | ~10 | `pub(crate) use crate::file_io::write_failure_message;` re-export (line 302) |

**Verification:** `cargo nextest run -p vb_cli --all-features` plus a
focused `cargo clippy -p vb_cli --all-features -- -D
clippy::string_slice -D clippy::indexing_slicing` confirms the
substring matcher is gone. `moon :lint-src` exits 0; `moon
:source-length` exits 0.

**Hours: 5** (1h Holzman fix + 50+ call-site migrations; 2h
4-module split; 1h rewire; 1h re-test + clippy pass).

---

### 4. `crates/vb_core/src/workflow/compiled_slug.rs` (583 lines, production canonical) — **split**

**Decision:** Split. Production is lines 1–320 (over-limit on its own
at 320), tests are lines 322–583. Both halves must be split because
both are above 300.

**New files:**

| New path | Lines (est.) | Contains |
|----------|-------------:|----------|
| `workflow/compiled_slug.rs` | ~150 | module doc, `MAX_SLUGS_PER_WORKFLOW`, `MAX_SLUG_PATH_SEGMENTS`, `YbBoundedSlug` + impl, `CompiledSlugs`, `YbBoundedSlugs` + impl, `SlugParseError` (lines 1–126 of source) |
| `workflow/compiled_slug/validation.rs` | ~190 | `checked_total_yield_cost`, `validate_compiled_slug_count`, `validate_compiled_slug_summary`, `validate_slug_admission_kernel`, `slug_summary_error`, `max_slug_path_depth`, `validate_compiled_slug_parts`, `validate_compiled_slugs`, `from_bytes_compiled_slugs` (lines 128–320) |
| `workflow/compiled_slug/tests.rs` | ~262 | the existing `#[cfg(test)] mod tests` block, imported via `#[path = "tests.rs"] #[cfg(test)] mod tests;` from `compiled_slug.rs` |

**Pattern precedent:** `vb_core/src/workflow/compiled_slug_kani.rs`,
`compiled_query_kani.rs`, `compiled_total_cost_kani.rs`,
`proptest_workflow.rs` already use the directory + sibling-file
pattern; this plan follows it.

**Verification:** `cargo nextest run -p vb_core --all-features`; the
Kani harnesses (`compiled_slug_kani.rs`) keep importing
`crate::workflow::compiled_slug::YbBoundedSlug` from the slim
module, so no re-export plumbing is required beyond the existing
`pub` visibility. `moon :source-length` exits 0.

**Hours: 4** (1h production split; 1h test split; 1h rewire + clippy;
1h re-test + kani-list).

---

### 5. `crates/vb_core/tests/vb_ajc40_public_decode_regression.rs` (600 lines, test) — **split**

**Decision:** Convert the integration test into a directory
`tests/vb_ajc40_public_decode_regression/` with a support module and
two scenario modules (slugs, queries). The top-level
`tests/vb_ajc40_public_decode_regression.rs` becomes a thin wrapper
because Cargo's test discovery expects a `tests/*.rs` file
**or** a `tests/<name>/main.rs` binary; integration test directories
use `tests/<name>/main.rs`.

**New files:**

| New path | Lines (est.) | Contains |
|----------|-------------:|----------|
| `tests/vb_ajc40_public_decode_regression/main.rs` | ~100 | `use` block, helper builders (`path`, `slug`, `query`, `slug_payload`, `query_payload`, `encode_slugs`, `encode_queries`, `first_slug_path_depth`, `first_query_path_depth`), `mod compiled_slugs;` and `mod compiled_queries;` |
| `tests/vb_ajc40_public_decode_regression/compiled_slugs.rs` | ~300 | slug-specific fixtures and 9 slug regression tests (lines 100–365 of source) |
| `tests/vb_ajc40_public_decode_regression/compiled_queries.rs` | ~250 | query-specific fixtures and 9 query regression tests (lines 366–600 of source) |

**Cargo discovery rule:** `tests/<name>.rs` is one integration test
binary; `tests/<name>/main.rs` is the equivalent directory form. The
test binary name in the runner is `vb_ajc40_public_decode_regression`
either way; CI scripts that grep the binary name keep working.

**Hours: 5** (1h main.rs + helpers; 1h slugs sub; 1h queries sub;
1h `cargo nextest` re-test; 1h confirm binary name unchanged).

---

### 6. `crates/vb_ipc/src/server/dispatch_tests.rs` (302 lines, test) — **split**

**Decision:** Convert `dispatch_tests.rs` into a directory
`server/dispatch_tests/` with a support module and two test-group
modules. `server/mod.rs:27` already has
`mod dispatch_tests;`; switch to `#[path = "dispatch_tests/mod.rs"]
mod dispatch_tests;`.

**New files:**

| New path | Lines (est.) | Contains |
|----------|-------------:|----------|
| `server/dispatch_tests/mod.rs` | ~10 | `mod` declarations |
| `server/dispatch_tests/support.rs` | ~55 | `NEXT_SOCKET_ID`, `temp_socket_path`, `CleanupPath`, `make_runtime` (lines 1–55 of source) |
| `server/dispatch_tests/serve_ipc.rs` | ~95 | `serve_ipc_returns_true_when_server_should_continue`, `serve_ipc_returns_false_on_shutdown`, `serve_ipc_propagates_poll_once_errors`, `serve_ipc_with_resolver_forwards_to_poll_once_with_resolver`, `serve_ipc_with_resolver_returns_false_when_server_should_shutdown`, `serve_ipc_with_resolver_propagates_poll_once_errors` (lines 57–145) |
| `server/dispatch_tests/dispatch_command.rs` | ~150 | `dispatch_command_wrapper_delegates_to_dispatch_command_with_resolver`, `dispatch_command_with_resolver_*`, `dispatch_unknown_command_*` (lines 145–302) |

**Hours: 3** (1h support; 1h two sub-modules; 1h rewire + re-test).

---

### 7. `crates/workspace_tests/tests/vb_a7t6_3_instruction_count_tests.rs` (317 lines, test) — **split**

**Decision:** Convert to a directory with the parser in a sub-module
and tests in the main file. The parser + `ParseError` is a reusable
internal API; the fixtures and tests stay in `main.rs`.

**New files:**

| New path | Lines (est.) | Contains |
|----------|-------------:|----------|
| `tests/vb_a7t6_3_instruction_count_tests/main.rs` | ~210 | `use` block, `PERF_STAT_*` constants, `mod parser;` declaration, all 12 `#[test]` functions (lines 109–317) |
| `tests/vb_a7t6_3_instruction_count_tests/parser.rs` | ~100 | `parse_count_token`, `parse_perf_stat_count`, `ParseError` + `Display` + `Error` impls (lines 31–100) |

**Hours: 2** (1h parser sub-module; 1h rewire + re-test).

---

## Stale Row Cleanup

The gate emits a `stale exception` error for any row whose file
currently has `<= 300` physical lines (see
`scripts/source_length_ledger.rs:31-39`). Two rows fail this check
today:

| Line | Row | Actual | Claimed | Action |
|-----:|-----|-------:|--------:|--------|
| 436 | `verification/proptest/properties.rs\|lewis\|vb-2lu1\|split-or-retire-before-release\|vb-2lu1 add exception for 369-line file.` | 94 | 369 | **delete row** |
| 493 | `crates/vb_ipc/src/server/dispatch.rs\|lewis\|vb-5iebh\|split-or-retire-before-release\|vb-5iebh moon ci baseline: 332-line IPC dispatch (32 over limit); split dispatch branches or trim dead handlers before removing exception.` | 66 | 332 | **delete row** |

The `pre-existing baseline` rows below `verification/proptest/` (e.g.
`crates/vb_benchmark/src/lib.rs`, `crates/vb_boundary_inventory/src/...`)
are not stale today — they reference real over-limit files. They
remain in the ledger.

**Hours: 0.5** (one PR, two line deletions, one moon :source-length
run to confirm zero stale errors).

---

## Ledger Self-Test: Monotonic Non-Increase per Quarter

The ledger currently has 481 `split-or-retire-before-release` rows
that "never close." A self-test must enforce monotonic non-increase
per calendar quarter so the count cannot grow without explicit human
intervention.

**New files:**

| New path | Lines (est.) | Contains |
|----------|-------------:|----------|
| `.config/source-length-quarterly-snapshot.json` | ~6 | `{"quarter": "2026-Q2", "split_or_retire_count": 481, "frozen_at_iso": "2026-06-07T00:00:00Z"}` |
| `scripts/source_length_quarterly_gate.rs` | ~80 | reads ledger, counts `split-or-retire-before-release` rows, reads snapshot, computes current quarter, fails if current_count > snapshot.split_or_retire_count for the same quarter, fails if a new quarter's count exceeds the previous quarter's count, prints the diff |
| `scripts/check-source-length-quarterly.sh` | ~10 | bash wrapper that compiles and runs the gate |

**Wire the new task into Moon:**

```yaml
# .moon/tasks/all.yml (additions)
source-length-quarterly:
  command: 'bash scripts/check-source-length-quarterly.sh'
  inputs:
    - '.config/source-length-exceptions.txt'
    - '.config/source-length-quarterly-snapshot.json'
    - 'scripts/source_length_quarterly_gate.rs'
    - 'scripts/check-source-length-quarterly.sh'
    - '.moon/tasks/all.yml'
  options:
    cache: false
    runInCI: true

source-length-self-test:
  command: 'bash scripts/check-source-length-tests.sh'
  inputs:
    - 'scripts/check-source-length.sh'
    - 'scripts/check-source-length.rs'
    - 'scripts/check-source-length-tests.sh'
    - 'scripts/source_length_gate.rs'
    - 'scripts/source_length_ledger.rs'
    - 'scripts/source_length_scan.rs'
    - 'scripts/source_length_quarterly_gate.rs'  # NEW
    - '.config/source-length-quarterly-snapshot.json'  # NEW
    - '.moon/tasks/all.yml'
  options:
    cache: false
    runInCI: true
```

**Self-test rules:**

1. Read `.config/source-length-exceptions.txt`.
2. Count rows where field 3 is `split-or-retire-before-release`. Call
   this `current_count`.
3. Read `.config/source-length-quarterly-snapshot.json` (a tiny
   `{quarter, count}` file). If the file is missing, the test fails
   closed (`Err`); the operator must write the initial snapshot.
4. Compute `current_quarter = format!("{}-Q{}", year, (month-1)/3 + 1)`
   from UTC date.
5. If `current_quarter == snapshot.quarter` and
   `current_count > snapshot.count`, fail: ledger regressed within
   the quarter.
6. If `current_quarter != snapshot.quarter` and
   `current_count > snapshot.count`, fail: ledger regressed at the
   quarter boundary.
7. On success, the snapshot is **not** auto-updated; updating the
   snapshot is a manual `bd remember` operation. The self-test only
   reads; it never writes.

**Why fail-closed on the snapshot:** the ledger is permanent
waiver; if a single quarter goes over the previous quarter's count,
a new `split-or-retire-before-release` row was added without
retiring another one. The self-test forces the operator to close
out an existing row before adding a new one. Retiring a row
happens when its file is split below 300 lines; the row is then
deleted from the ledger (the file is under limit, so the row would
be rejected as stale anyway).

**Hours: 4** (1h design; 1h impl; 1h `.moon/tasks/all.yml` wiring +
self-test scaffold; 1h run, calibrate snapshot to 481, confirm
exit 0).

---

## Total Estimate

| # | Item | Hours |
|---|------|------:|
| 1 | Split `cli_postcard/tests.rs` (5 files) | 6 |
| 2 | Split `cli_postcard/types.rs` (7 files) | 8 |
| 3 | Delete Holzman §3 substring matcher in `output.rs` + split (4 files) | 5 |
| 4 | Split `compiled_slug.rs` (3 files) | 4 |
| 5 | Split `vb_ajc40_public_decode_regression.rs` (3 files) | 5 |
| 6 | Split `dispatch_tests.rs` (3 files) | 3 |
| 7 | Split `vb_a7t6_3_instruction_count_tests.rs` (2 files) | 2 |
| 8 | Delete 2 stale rows from `.config/source-length-exceptions.txt` | 0.5 |
| 9 | Quarterly ledger self-test (gate + snapshot + Moon wiring) | 4 |
| | **Total** | **37.5** |

Sequencing: items 1–7 (file splits) are independent and can land as
separate beads in parallel; item 8 is a one-line commit; item 9 is a
new gate that depends on items 1–8 having stabilised the ledger
state (so the initial snapshot is meaningful).

## Definition of Done

The bead is **done** when **all** of the following hold, verified
with raw command output captured in
`evidence/source-length-r2-repair/`:

1. `bash scripts/check-source-length.sh` exits 0 (no over-limit files
   with no valid row, no stale rows, no malformed rows, no duplicate
   rows, no invalid paths, no invalid start lines, no
   `cargo-mutants` residue, no monolithic `compile_core_impl.rs` body).
2. `moon :source-length` exits 0.
3. `moon :source-length-self-test` exits 0 (the existing
   self-tests in `scripts/check-source-length-tests.sh` plus the new
   quarterly self-test in `scripts/source_length_quarterly_gate.rs`).
4. `moon run :source-length-quarterly` exits 0 with a snapshot of
   `{"quarter": "2026-Q2", "split_or_retire_count": <N>}` where
   `<N> <= 481` (the round-2 baseline).
5. `infer_legacy_json_error_code` does not appear in any tracked
   `*.rs` file (Holzman §3 violation removed). Verify with
   `git grep -n infer_legacy_json_error_code -- '*.rs'` returning
   empty.
6. `moon :lint-src` exits 0 (clippy -D warnings passes on every new
   file).
7. `moon :test` exits 0 (full nextest run is green; the test runner
   still discovers every `#[test]` function from the split
   directories).
8. `bd close vb-source-length-r2` succeeds, `bd dolt push` succeeds,
   `git push` succeeds; the local worktree matches `origin/main`.

## Risks and Notes

- **Cargo test discovery for split directories:** every test file
  split (items 5 and 7) uses `tests/<name>/main.rs` so the binary
  name in nextest is unchanged. This is verified by `cargo nextest
  list -p vb_core` and `cargo nextest list -p
  velvet-ballistics-workspace-tests` after the split.
- **Public API churn for `cli_postcard::types`:** every existing
  `use crate::cli_postcard::CliPostcardKind` (and the 20+ other
  re-exports in `cli_postcard/mod.rs:28-34`) keeps compiling because
  the new `types/mod.rs` re-exports the same surface. Lint with
  `moon :lint-src` to catch accidental re-export loss.
- **Hot function limit (25 logical lines) is independent of file
  length:** the file splits do not change any function body, so the
  hot ledger (`.config/hot-function-length-exceptions.txt`) is
  untouched. The new compiled_slug validation module is production
  canonical and must stay below 25 logical lines per function;
  verify with `moon :source-length` (which checks both gates).
- **The 481 `split-or-retire-before-release` rows are out of scope**
  for this bead. This bead is "make the gate exit 0 and stop
  regressing"; the 481 rows are tracked work for downstream
  split-or-retire beads. The quarterly self-test prevents
  regression; closing the 481 is a separate, multi-quarter
  programme tracked under `vb-jpq7.47` and similar.
- **Snapshot file location:** `.config/` is already the home of
  `.config/source-length-exceptions.txt`, so the snapshot file
  lives there. Both are tracked; neither is in `.gitignore`.

## Acceptance Command Bundle

The landing evidence (`evidence/source-length-r2-repair/`) must
include raw stdout+stderr for each of:

```bash
bash scripts/check-source-length.sh
moon :source-length
moon :source-length-self-test
moon run :source-length-quarterly
moon :lint-src
moon :test
git grep -n infer_legacy_json_error_code -- '*.rs' || true
```
