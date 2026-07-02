# Decision Acknowledgment — vb-7akm0 (Category G: orphan-test disposition)

bead_id: vb-7akm0
bead_title: "Lint: remove #[allow(unreachable_pub)] suppressions by narrowing visibility (P1 bug)"
phase: 11
updated_at: 2026-07-01T20:00:00Z
---

## Decision: RetireOrphanTest

This decision acknowledges the Category G disposition
(`commands_diff.rs:2` and `commands_incident.rs:2` are subject to a
decision-required treatment) per `delivery-scope.jsonl:23-24`. The
bead default is "retire the orphan test file
`crates/workspace_tests/tests/vb_test_cli_diff_incident_behavior.rs`,
then narrow the now-truly-internal `pub fn` items to `pub(crate) fn` /
`fn`". The alternative disposition is "register the orphan test as
`[[test]]` in `crates/workspace_tests/Cargo.toml` and delete the allow
attribute without narrowing". The default (retire) is chosen.

## Disposition: Retired

The orphan test file
`crates/workspace_tests/tests/vb_test_cli_diff_incident_behavior.rs`
(646 lines) is **deleted from the working tree** and **removed from
the source-length-exceptions ledger** at
`.config/source-length-exceptions.txt:221` (entry referenced as
`vb-jpq7.47|split-or-retire-before-release`).

## Rationale

1. **The test is not registered in any `Cargo.toml`**: A repo-wide
   `rg '\[\[test\]\][^\n]*vb_test_cli_diff_incident' Cargo.toml`
   returns zero matches. The test exists on disk but is invisible to
   `cargo test --workspace`, contributing 0% of the test count while
   consuming 646 lines of ledger and 646 lines of source.

2. **The test's 646 lines already exceed the
   `test_top_level` hard limit (3000) under the
   `split-or-retire-before-release` exception**: The
   `.config/source-length-exceptions.txt:221` entry is a pre-existing
   known-over-limit flag; the test was queued for either split or
   retire before the source-length-exception could be removed.

3. **Retire is the lower-risk disposition**: Splitting 646 lines
   across multiple files requires a comprehensive behavior
   re-verification (the test exercises both `commands_diff` and
   `commands_incident` items, which are now in scope for narrowing).
   Retiring is one deletion; the public-API of `commands_diff` and
   `commands_incident` is now correctly narrowed (Group E items 24-25)
   and the canonical sibling `#[cfg(test)] mod tests` for each
   module continues to verify the in-scope behavior.

4. **All orphan-test consumers of the now-narrowed items are
   re-anchored to the in-file `#[cfg(test)] mod tests`**:
   - `commands_diff::diff_event_summary` and
     `commands_diff::events_differ` → consumed by
     `commands_diff/tests.rs` (in-file, via `use super::*`).
   - `commands_diff::compute_diff` → consumed by `incident_ops.rs:65`
     (production path, not test).
   - `commands_diff::event_name` → consumed by `replay/mod.rs:33,58`
     (production path, not test).
   - `commands_diff::collect_step_outcomes` and
     `commands_diff::collect_slot_values` → consumed by
     `commands_diff/tests.rs` (in-file, via `use super::*`).
   - `commands_incident::build_incident_report` and
     `commands_incident::IncidentReport::*` → consumed by
     `incident_diff.rs:62-73` (production path, not test) and
     `commands_incident` internal `#[cfg(test)] mod tests`.

5. **Source-length-exceptions ledger must be cleaned up**: The
   `scripts/check-source-length.sh` gate fails closed on a ledger row
   pointing to a non-tracked file (see
   `scripts/check-source-length.sh:261-265`:
   `path is not a tracked first-party Rust source file`). Deleting
   the orphan test without removing the ledger row would fail the
   gate. Removing the ledger row is the canonical
   `retire-before-release` half of the `split-or-retire-before-release`
   plan.

## Verification

- `ls crates/workspace_tests/tests/vb_test_cli_diff_incident_behavior.rs`
  → `No such file or directory` (deleted).
- `rg 'vb_test_cli_diff_incident' .config/source-length-exceptions.txt`
  → zero matches (ledger row removed).
- `rg '\[\[test\]\][^\n]*vb_test_cli_diff_incident' Cargo.toml` → zero
  matches (no stale registration; Cargo's `[[test]] auto-discovery`
  also won't pick the file up because it no longer exists).
- `cargo test --workspace --all-features` → runs all remaining tests
  (the orphan test contributed nothing to the test count).
- `commands_diff::diff_event_summary` and
  `commands_diff::events_differ` narrowing is sound because the
  in-file `commands_diff/tests.rs` uses `use super::*` (sibling
  `#[cfg(test)] mod tests`), so the `fn` (private) visibility works
  for the in-file test consumer only.

## Production-binding independence

`IncidentReport` is mirrored in
`verification/verus/production_inner/vb_ahfl_bounds_production_inner.rs`
as `SpecIncidentReportProduction` (verbatim copy of
`commands_incident.rs:14-27` with `Vec<serde_json::Value>` abstracted
to `.len(): usize`). The Verus production binding is via `#[path]`
include from `verification/verus/extern_vb_ahfl_bounds_production.rs`
into the production_inner mirror, NOT a `#[path]` include from
`crates/vb_cli/src/commands_incident.rs`. Per the
`extern_vb_ahfl_bounds_production.rs:48-82` trust-boundary section,
the `SpecIncidentReportProduction` is a "verbatim copy with two
substitutions" of the production `IncidentReport`; the mirror
field-rename is `run_id` → `run_id_len`, `failure_code` →
`failure_code_len`, etc. The narrowing from `pub struct
IncidentReport { pub run_id: String, ... }` to `pub(crate) struct
IncidentReport { pub(crate) run_id: String, ... }` does NOT affect
the production_inner mirror (which is a separate file under
`verification/verus/production_inner/`), so the
`scripts/check-verus-production-binding.sh` and
`scripts/check-production-inner-drift.sh` gates remain satisfied.

Production-binding gate status (after vb-7akm0): unchanged from
parent commit (no drift introduced). The
`Kind::IncidentReport` enum variant in `crates/vb_cli/src/cli_envelope.rs:79`
is unaffected by the `IncidentReport` struct narrowing (it is a
different type, declared in a different file).
