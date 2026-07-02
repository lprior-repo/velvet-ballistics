# Landing Report — vb-n5k6v

## Bead: Tests: wire orphaned edge_case_tests (P1 bug)

### Summary

Land the State 11 holzman-rust implementation that closes the
`vb-n5k6v` finding: the orphaned `edge_case_tests.rs` file in
`crates/vb_storage/src/` was sitting on disk (637 lines, 26 dormant
`#[test]` fns) but had no `#[path = "..."] mod` declaration in
`crates/vb_storage/src/lib.rs` to register it with the lib-test build.
The fix is a 3-line `#[cfg(test)] #[path = "edge_case_tests.rs"] mod
edge_case_tests;` insertion at `crates/vb_storage/src/lib.rs:183-186`
(matching the canonical 16-sibling `#[path = "..."]` pattern at
`lib.rs:118-181`), plus a 4-line `#[cfg(test)]` mirror of the existing
`persist_strict` test-only `consume_persist_failure_for_test` flag at
`crates/vb_storage/src/journal/append.rs:36-39` to make
`persist_strict_recovers_after_simulated_failure`
(`edge_case_tests.rs:69`) pass deterministically.

The single-touch `append_strict` `#[cfg(test)]` fix at
`journal/append.rs:36-39` is `#[cfg(test)]`-only and stripped from
release builds; it mirrors the `persist_strict` test-only
flag-consumption pattern at `journal/append.rs:86-89` byte-for-byte.
The `consume_persist_failure_for_test` helper at `journal/core.rs:232-234`
is the canonical test-only seam (`pub(crate)`, `#[cfg(test)]`,
returns the atomic-swap-consumed flag value). No new types, no new
error variants, no new helpers introduced.

### Single Commit on the State-11 line

| Hash | Message |
|------|---------|
| `84a5eb7d303a` | `vb-n5k6v: rust-contract artifacts (orphaned edge_case_tests wiring, P1 test-only repair)` |

- jj change id: `womqwkksqltu`
- jj change commit: `84a5eb7d303a`
- Author: `femdation-controller` (cheap25 batch, landing-skill pass)
- Parent commit: `rsvywymk 1d6c017f` (`AGENTS.md: capture coord-checkout contamination traps seen in round10 forward-port`)
- Bookmark: not yet promoted — the femdation master controller performs
  the `bookmark move main --to @` and `jj git push --bookmark main`
  in its serialized landing pass. The change currently sits on `@`
  (`womqwkks 84a5eb7d`).

### Files Changed (2 files, 8 insertions, 0 deletions)

```
crates/vb_storage/src/lib.rs            | 4 ++++
crates/vb_storage/src/journal/append.rs | 4 ++++
2 files changed, 8 insertions(+), 0 deletions(-)
```

### Per-File Code Diff Synopsis

| File | Lines | Change | Status |
|------|-------|--------|--------|
| `crates/vb_storage/src/lib.rs` | 183-186 | 3-line `#[cfg(test)] #[path = "edge_case_tests.rs"] mod edge_case_tests;` declaration + 1 blank separator | Production wire (the file already exists; this just registers it for the lib-test build) |
| `crates/vb_storage/src/journal/append.rs` | 36-39 | 4-line `#[cfg(test)] if self.consume_persist_failure_for_test() { return Err(JournalError::StrictDurabilityFailed); }` mirroring `persist_strict:86-89` | Test-only (`#[cfg(test)]`-stripped in release builds) |

The 4-line `lib.rs` insertion (3 declaration + 1 blank separator)
matches the 16-sibling canonical pattern at `lib.rs:118-181` —
every sibling declaration is followed by a blank line; without it
`mod edge_case_tests;` would be immediately followed by
`pub mod queue;` with no separator, breaking the visual pattern.

### Quality Gates (re-executed in the isolated workspace)

All gates were re-executed against the State 11 commit `womqwkks
84a5eb7d` from the isolated workspace at
`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-n5k6v` and
captured into `.beads/vb-n5k6v/evidence/`:

| # | Command | Result | Evidence |
|---|---------|--------|----------|
| 1 | `PROPTEST_CASES=1 cargo test -p vb_storage --lib` (pre-wire baseline) | **1530 passed, 0 failed (1 suite, 0.95s)** | `evidence/pre-wire-test-count.txt` |
| 2 | `PROPTEST_CASES=1 cargo test -p vb_storage --lib` (post-wire) | **1556 passed, 0 failed (1 suite, 1.36s)** | `evidence/post-wire-test-count.txt` |
| 3 | `cargo test -p vb_storage --lib edge_case` | **26 passed, 0 failed (1 suite, 0.07s)** | `evidence/cargo-test-edge-case.txt` |
| 4 | `cargo test -p vb_storage --lib close_propagates_persist_errors` (regression) | 1 passed, 1555 filtered out (1 suite, 0.01s) | `evidence/close-propagates-test.txt` |
| 5 | `cargo test -p vb_storage --lib persist_strict` (regression) | 5 passed, 1551 filtered out (1 suite, 0.01s) | `evidence/persist-strict-tests.txt` |
| 6 | `cargo test -p vb_storage --lib append_strict` (regression) | 25 passed, 1531 filtered out (1 suite, 0.03s) | `evidence/append-strict-tests.txt` |
| 7 | `cargo check --workspace --all-targets --all-features` | Finished `dev` profile (139 crates compiled, 9.04s) | `evidence/cargo-check-workspace.txt` |
| 8 | `cargo check -p vb_storage --tests` | exit 0, "cargo build (0 crates compiled) Finished `dev` profile" | `dispatch/state-12-formal-verifier/command-logs/cargo_check_vb_storage_tests.log` |
| 9 | `cargo clippy -p vb_storage --lib -- -D warnings` (source target, strict) | exit 0, "No issues found" | `dispatch/state-12-formal-verifier/command-logs/cargo_clippy_vb_storage_lib_strict.log` |

**Total: 1556 tests pass; 26 dormant tests surfaced; 0 regressions.**

Delta: +26 tests (1530 → 1556), exactly matching the CC-WIRE-005
contract clause and the 26 dormant test inventory in CC-WIRE-004.

### Pre-existing FAIL_GLOBAL (carrier-forwarded, not blockers)

Three pre-existing `FAIL_GLOBAL` classifications were honestly reported
in `final-evidence-decision.md` and are zero-impact for vb-n5k6v:

1. **Test clippy strict gate** (`cargo clippy -p vb_storage --tests -- -D warnings`):
   240 errors, 236 predate the bead on parent commit `rsvywymk 1d6c017f`;
   +4 newly-exposed E0453 in `crates/vb_storage/src/edge_case_tests.rs:4,6,7,8`
   from the file's pre-existing `#![allow(...)]` block (lines 1-9, file
   content byte-identical pre/post wire). Per AGENTS.md: "Tests must
   compile and run, but test clippy is not strict." **Zero impact on
   vb-n5k6v closure.**
2. **`cargo fmt --check` drift**: pre-existing format drift in
   `edge_case_tests.rs:627,632` and other files (`vb_core/src/lib.rs:26`,
   `vb_runtime/frame_pool/tests.rs`, `vb_core/src/time.rs`). The 4 lines
   added by this bead are fmt-clean (match the 16-sibling pattern).
   **Zero impact on vb-n5k6v closure.**
3. **Workspace `cargo test --workspace --no-run` failure**: pre-existing
   E0624 errors in `vb_compile/tests/*` calling `WorkflowSource::new`
   from `tests/common/mod.rs`. Not in vb-n5k6v blast radius; pre-existing
   on parent commit `rsvywymk 1d6c017f`. The `vb_storage` workspace
   build (`cargo check --workspace --all-targets --all-features`) is
   clean (139 crates compiled, 9.04s). **Zero impact on vb-n5k6v
   closure.**

All three classifications are **honestly FAIL_GLOBAL but zero impact
on vb-n5k6v closure**. They are reported per the formal-verifier skill
rule "Existing unrelated global failures: classify honestly; do not
turn them into proof success" and do not block landing.

### Bead Closure (from coord checkout `/home/lewis/src/velvet-ballistics`)

```
$ bd close vb-n5k6v --reason "edge_case_tests.rs wired as cfg(test) mod in lib.rs:182; 26 dormant tests now run; test count delta 1530 → 1556; no Cargo.toml change; no production-logic change."
✓ Closed vb-n5k6v — Tests: wire orphaned edge_case_tests or delete stale file: edge_case_tests.rs wired as cfg(test) mod in lib.rs:182; 26 dormant tests now run; test count delta 1530 → 1556; no Cargo.toml change; no production-logic change.

$ bd dolt push
Pushing to Dolt remote...
Error: failed to push to origin/main: Error 1105 (HY000): To https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics
 ! [rejected]            main -> main (non-fast-forward)
hint: Updates were rejected because the tip of your current branch is behind
hint: its remote counterpart. Integrate the remote changes (e.g.
hint: 'dolt pull ...') before pushing again.

$ bd dolt pull
Pulling from Dolt remote...
Pull complete.

$ bd dolt push
Pushing to Dolt remote...
Push complete.
```

The non-fast-forward rejection was a sibling-bead race on the Dolt
remote (other cheap25-batch beads are landing in parallel); `bd dolt
pull` reconciled the local Dolt branch and the retry pushed clean.
Per the landing-skill backoff protocol, push was retried after
`bd dolt pull` re-synced state. No data was lost.

### State-of-the-World After Landing

- `bd show vb-n5k6v`: `● P1 · CLOSED`, owned by Lewis, close-reason
  recorded, `closed_at: 2026-07-02T06:07:52Z`.
- `bash scripts/check-beads-server-mode.sh` → "beads server-mode check
  passed" (pro-active verification; the active backend is server mode
  only and `.beads/embeddeddolt/` is not present).
- `bd dolt push` (post-close) → "Push complete."
- Source checkout `/home/lewis/src/velvet-ballistics` is clean
  (HEAD detached at the current cheap25 main; no `bd close`/`bd`/
  `scratch` operations were performed from this checkout other than
  `bd close`, `bd dolt push`, and `bd dolt pull`).
- The jj change `womqwkks 84a5eb7d` remains pointed at the State 11
  commit; the femdation controller performs the `bookmark move main
  --to @` and `jj git push --bookmark main` in its serialized landing
  pass.

### Ledger Surface Touched This Landing

- `agent-invocation-ledger.jsonl` — sequence 8 (state 15, landing-skill)
  and sequence 9 (state 16, cleanup-skill). Both new entries' `entry_hash`
  computed as SHA-256 of canonical JSON body (sort_keys + compact
  separators) and the `previous_entry_hash` chain links state-14 →
  state-15 → state-16 unbroken.
- `routing-ledger.jsonl` — 2 new rows: state 15 (`landing` sublane) and
  state 16 (`cleanup` sublane). The routing-ledger `entry_hash` values
  mirror the corresponding `agent-invocation-ledger` `entry_hash`
  values (the routing-ledger is a state-transition mirror, not an
  independent hash chain).

No other ledger files were modified by the landing subagent; verification
rows 1-3 (states 12/13/14) are owned by the prior stages and remain
immutable.

### Production Contract Pin (Provenance)

The runtime contract being pinned by the test-only fix is the
production branch in `crates/vb_storage/src/journal/append.rs:35-39`:

```rust
pub fn append_strict(&self, event: &JournalEvent) -> Result<(), JournalError> {
    #[cfg(test)]
    if self.consume_persist_failure_for_test() {
        return Err(JournalError::StrictDurabilityFailed);
    }
    // ...validate first so an invalid event is rejected before any allocation...
}
```

The flag-consumption helper at
`crates/vb_storage/src/journal/core.rs:232-234` is `pub(crate)` and
`#[cfg(test)]` only; the `fail_next_persist_for_test` field is
`pub(crate)` and `#[cfg(test)]` only; the `StrictDurabilityFailed`
error variant is the existing variant in `JournalError`. No new
public API, no new error variant, no new helper introduced.

The 26 changed test functions are all in
`crates/vb_storage/src/edge_case_tests.rs` (file content
byte-identical pre/post wire; 637 lines); the wire is a build-graph
fix and does not modify the test code itself.

End of landing report.
