# Codebase Map — vb-pg2wq

- bead_id: vb-pg2wq
- bead_title: Tests: make duplicate-event test assert one exact contract (P1 bug)
- captured_at: 2026-07-01T15:50:00Z
- scout: explore subagent (direct child of femdation)
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq
- source_checkout: /home/lewis/src/velvet-ballistics (read-only observation; no edits performed)
- source_head: 2c8ea33c9 (origin/main)
- jj_parent: rsvywymk (AGENTS.md round10 forward-port)
- upstream_main: 2c8ea33c9
- prior_merges: cheap25-tier1 sweep (7 P1 closures), shared-25-beads holzman-rust + black-hat batch
- related_prior_findings: vb-82snf (EPIC — fuzz recovery assertions, related P1 sibling)

## Bead Scope (from bd show vb-pg2wq)

- Parent epic: e06.
- Finding focus (verbatim from bead description and
  `bd show vb-pg2wq`'s "Section 0: Clarifications" / "Section 9: Context"):
  > "Duplicate-event test accepts DuplicateEvent, any error, or Ok,
  > making it unable to catch regressions."
- Audit citation in bead text:
  > "A test for duplicate-event handling has fuzzy or weak assertions
  > (e.g., assert_ne!, asserts on len only). Replace with an exact
  > contract assertion that pins the exact behavior on duplicate detection."
- Required EARS obligations (read from bead):
  - "THE SYSTEM SHALL remediate this audited Fuzz and tests finding
    without weakening existing safety, durability, lint, proof, or
    evidence gates."
  - "THE SYSTEM SHALL preserve the master contract prohibition on
    unsafe, unwrap, expect, panic, unchecked indexing, unchecked
    casts, unchecked arithmetic, ignored fallible results, runtime
    YAML, runtime JSON, and runtime HTTP."
  - Trigger: "WHEN the implementation changes production behavior —
    Shall: THE SYSTEM SHALL add or repair behavior tests before
    claiming completion."
  - Trigger: "WHEN a verifier or test exposes an implementation flaw —
    Shall: THE SYSTEM SHALL fix the implementation rather than
    weakening the proof, harness, or assertion." (No production
    fix is in scope per the bead wording — only test repair.)
- Anti-hallucination guard paths (must_read_first=true per
  `bd show vb-pg2wq` Section 7.5):
  - `crates/workspace_tests`
  - `crates/vb_storage/src/tests.rs`
  - `fuzz`
  - `velvet-ballistics-MASTER.md`

## Primary Targets (weak duplicate-event tests to repair)

All four files use the identical weak-assertion pattern:

  `let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }));`
  `prop_assert!(is_dup);`

`..` discards both `run` and `seq` fields of the typed error.
A regression that returns `DuplicateEvent { run: 0, seq: 0 }` (or
any other field mismatch) would still pass; a regression that
mutates the variant to a sibling (e.g., `BatchAborted`, `QueueFull`,
`KeyCapacity`) would fail — but only by accident, not by pinning the
typed contract. The exact contract is the `run`/`seq` tuple the
production code emitted (see "Production contract" below).

### Target 1 — `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs`

- File: `/home/lewis/src/velvet-ballistics/crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs`
- Weak assertion: lines 69-79 (`fn ps001_duplicate_rejected`)
  - lines 77-78: `let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. })); prop_assert!(is_dup);`
- Proptest inputs: `run in 1u64..1000u64, seq in 0u64..100u64`
- Filesystem line count: 80 (proptest module only)
- Shared helpers in module (used by all PS_00x proptests):
  - `make_event(run: u64, seq: u64) -> JournalEvent`
  - `temp_journal() -> (tempfile::TempDir, FjallJournal)`
- Imports touched: `vb_storage::error::JournalError`,
  `vb_storage::batch::JournalWriteBatch`,
  `vb_storage::journal::FjallJournal`,
  `vb_storage::events::JournalEvent`,
  `vb_core::{RunId, WorkflowDigest}`,
  `vb_storage::EventSeq`.

### Target 2 — `crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs`

- File: `/home/lewis/src/velvet-ballistics/crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs`
- Weak assertion: lines 55-65 (`fn ps003_dup_fields`)
  - lines 63-64: `let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. })); prop_assert!(is_dup);`
- The function name (`ps003_dup_fields`) is currently a lie: it
  does NOT assert any field of `DuplicateEvent`.
- Proptest inputs: `run in 1u64..1000u64, seq in 0u64..100u64`
- Filesystem line count: 83.

### Target 3 — `crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs`

- File: `/home/lewis/src/velvet-ballistics/crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs`
- Weak assertion: lines 27-36 (`fn ps008_dup_before_queue`)
  - line 35: `let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. })); prop_assert!(is_dup);`
- Filesystem line count: 71.

### Target 4 — `crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs`

- File: `/home/lewis/src/velvet-ballistics/crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs`
- Weak assertion: lines 27-37 (`fn ps009_dup_rejected`)
  - lines 35-36: `let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. })); prop_assert!(is_dup);`
- Filesystem line count: 96.
- Note: This is the only PS_009 file under
  `crates/vb_storage/tests/`; the corresponding fuzz target lives
  at `fuzz/fuzz_targets/vb_vzcuf_PS_009.rs` and uses different
  assertion logic — that fuzz file is OUT OF SCOPE for this bead.

### Target 5 — `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs`

- File: `/home/lewis/src/velvet-ballistics/crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs`
- Two weak assertions (the bead says "a test" singular, but this
  file carries two more `matches!(..., DuplicateEvent { .. })` patterns
  that are equally weak):
  - lines 38-54 (`fn ps004_no_persist`):
    - lines 47-48: `let duplicate_event = matches!(append_result, Err(JournalError::DuplicateEvent { .. })); prop_assert!(duplicate_event);`
  - lines 84-98 (`fn ps004_empty_commit_after_rej`):
    - lines 93-94: `let duplicate_event = matches!(append_result, Err(JournalError::DuplicateEvent { .. })); prop_assert!(duplicate_event);`
- Filesystem line count: 99.

### Aggregate count

- Total weak duplicate-event `..` matches across `crates/vb_storage/tests/`:
  6 occurrences in 5 functions across 4 files (ps001×1, ps003×1,
  ps004×2, ps008×1, ps009×1).
- All 6 use the same broken pattern. Reproduce command:
  `rtk rg -n "JournalError::DuplicateEvent \{ \.\. \}" crates/vb_storage/tests/`
  → 6 hits, all paths under `crates/vb_storage/tests/`.

## Production Contract (the source of truth the tests must pin)

### Type — `JournalError::DuplicateEvent`

- File: `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/error/mod.rs`
- Variant declaration: lines 30-31
  ```
  #[error("duplicate journal event for run {run:?} seq {seq:?}")]
  DuplicateEvent { run: RunId, seq: EventSeq },
  ```
- Distinguish from sibling variant on line 32-33:
  ```
  #[error("duplicate journal event staged in the same batch for run {run:?} seq {seq:?}")]
  DuplicateStagedKey { run: RunId, seq: EventSeq },
  ```
  (This is `DuplicateStagedKey`, a different variant. Tests in
  different contexts may legitimately expect either; the cross-batch
  scenario used in all 5 weak tests above targets `DuplicateEvent`,
  not `DuplicateStagedKey`. Both share identical payload fields.)

- Diagnostic code path:
  `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/error/codes.rs:104`
  maps `DuplicateEvent { .. }` → `DUPLICATE_EVENT_CODE` (string name
  `"DUPLICATE_EVENT"` at `codes.rs:197`). Downstream agents may use
  this if they wish to ALSO pin the diagnostic code (out of scope per
  the bead, but a fine-grained enhancement).

### Behavior — `JournalWriteBatch::append_event`

- File: `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/batch/append_event.rs`
- Lines 42-67 are the contract surface:
  ```
  pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError> {
      let key = run_event_key(event.run_id(), event.seq())?;
      if !event.is_valid() { return Err(JournalError::InvalidEvent); }
      if self.staged_event_keys.contains(&key) {
          return Err(JournalError::DuplicateStagedKey { run: event.run_id(), seq: event.seq() });
      }
      if self.journal.events.contains_key(key)? {
          self.aborted = true;
          return Err(JournalError::DuplicateEvent {
              run: event.run_id(),
              seq: event.seq(),
          });
      }
      ...
  }
  ```
- Cross-batch scenario (the one the 5 weak tests exercise):
  commit `event` in batch A, then construct batch B and call
  `append_event(&event)`. Production must:
  1. Return `Err(JournalError::DuplicateEvent { run: e.run, seq: e.seq })`
     where `e` is the exact `&JournalEvent` passed in.
  2. Set `batch.aborted = true` (no public field; observable as
     `JournalWriteBatch::is_aborted() == true`).
  3. NOT mutate the staged inner batch.
  4. NOT persist anything new (replay remains the original).
- The exact tuple contract is `(e.run_id(), e.seq())` — not a copy
  with shifted fields, not a synthesized `RunId::new(0)`.

### Reading the contract for tests

- File: `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/tests.rs`
- Existing exemplar already in the codebase, lines 1344-1367
  (`fn duplicate_event_returns_exact_run_and_seq`). This is the
  canonical pattern downstream implementers should mirror. Verbatim:
  ```
  fn duplicate_event_returns_exact_run_and_seq() {
      ...
      let event = JournalEvent::RunAccepted {
          run: RunId::new(42),
          seq: EventSeq::new(7),
          workflow: WorkflowDigest::from_bytes([3; 32]),
      };
      journal.append_journaled(&event).expect("...");
      let result = journal.append_journaled(&event);
      let Err(JournalError::DuplicateEvent { run, seq }) = result else {
          panic!("expected DuplicateEvent, got {:?}", result);
      };
      assert_eq!(run, RunId::new(42));
      assert_eq!(seq, EventSeq::new(7));
  }
  ```
- The proptest analog (what the 5 weak tests should look like) is
  shown at `tests.rs:4888-4892` on the `flush_batch` path:
  ```
  assert!(matches!(
      queue.flush_batch(&journal),
      Err(JournalError::DuplicateEvent { run: found, seq })
          if found == run && seq == EventSeq::new(0)
  ));
  ```
- Note: the 5 weak tests target `JournalWriteBatch::append_event`
  (the batch API), not `journal.append_journaled` (the direct API).
  The exact-assertion pattern is the same — substitute
  `matches!(append_result, Err(JournalError::DuplicateEvent { run: r, seq: s }) if r == RunId::new(run) && s == EventSeq::new(seq))`
  for the weak `matches!(..., DuplicateEvent { .. })`.

## Adjacent (NOT in scope, but referenced for context)

These tests already pin duplicate-event behavior with strong
assertions. They are NOT to be modified by this bead; they exist
as a model for downstream agents:

- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/batch/t_append_event.rs:20-43`
  `fn batch_append_event_rejects_duplicate_event` — already covers
  cross-batch duplicate but uses `..` (weak). Even the reference test
  in `src/batch/` is weak. (Adjacent finding — out of scope, but
  demonstrates the pattern is endemic; future bead candidate.)
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/batch/t_byte_accounting_part2.rs:74-82`
  `fn duplicate_event_fields_are_accurate` — formats the variant to
  check `format!("{err}").contains("42")`. Indirectly pins `run: 42`,
  but does NOT pin `seq`. Adjacent; out of scope.
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/batch/t_byte_accounting_part2.rs:84-106`
  `fn rejected_duplicate_event_not_staged_in_batch` — uses weak
  `..` match. Out of scope.
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/batch/t_byte_accounting_part3.rs:5-20`
  `fn duplicate_detection_fires_before_count_check` — `..` weak. OOS.
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/batch/t_byte_accounting_part3.rs:55-70`
  `fn duplicate_and_queue_full_conflict_duplicate_wins` — `..` weak. OOS.
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/batch/t_byte_accounting_part4.rs:5-20`
  `fn cross_batch_duplicate_is_rejected_with_duplicate_event` — `..` weak. OOS.
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/batch/t_byte_accounting_part4.rs:22-36`
  `fn duplicate_event_aborts_batch` — `..` weak. OOS.
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/batch/t_byte_accounting_part4.rs:76-104`
  `fn e2e_aborted_batch_commit_returns_typed_batch_aborted_error` — `..` weak. OOS.
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/batch/t_byte_accounting_part4.rs:106-129`
  `fn append_strict_batch_atomicity_rolls_back_on_duplicate` — `..` weak. OOS.

These adjacent items confirm the systematic nature of the bug;
the bead is intentionally scoped to the proptest files (the
"Fuzz and tests finding" naming in the bead hints at the proptest
lanes), and they share the exact same shape. Future bead
candidates may want to harden the `src/batch/` tests too.

## Production-side public APIs the tests depend on (already imported)

- `vb_storage::batch::JournalWriteBatch::new(&FjallJournal) -> Self`
- `vb_storage::batch::JournalWriteBatch::append_event(&mut self, &JournalEvent) -> Result<(), JournalError>`
  (file: `crates/vb_storage/src/batch/append_event.rs:42`)
- `vb_storage::batch::JournalWriteBatch::commit(&mut self) -> Result<(), JournalError>`
  (file: `crates/vb_storage/src/batch/commit.rs`)
- `vb_storage::batch::JournalWriteBatch::len(&self) -> usize`
- `vb_storage::batch::JournalWriteBatch::is_aborted(&self) -> bool`
- `vb_storage::journal::FjallJournal::events_for_run(run: RunId) -> Result<Vec<JournalEvent>, JournalError>`
- `vb_storage::error::JournalError` (enum with `DuplicateEvent { run, seq }` variant at `error/mod.rs:30`).

These public APIs are durable; the test fix must use them, not
internals. No production change is in scope; the audit finding is
about test assertion strength only.

## Unknowns / Open Questions

- UNKNOWN: Was the bead originally scoped to ALL 5 weak test
  functions, or only one ("a test for duplicate-event handling has
  fuzzy... assertions")? The audit description is singular
  ("a test", singular) but 6 hits across 5 functions in 4 files
  exist. Recommend: downstream contract/test-planner treat all
  6 hits as in-scope, since each one is independently regression-resistant
  against a field mutation, and the bead EARS obligation is
  "preserve... lint, proof, or evidence gates" which implies
  fixing the full class. If implementation wishes to land a
  smaller scope, it must record the remainder as a follow-up.
- The bead title says "duplicate-event test" singular but the
  source pattern across 5 functions is identical. Suggest the
  planner-agent resolve the singular/plural intent with the
  femdation controller before contracting implementation.
- The bead `bd show vb-pg2wq` Section 0 cites
  "Parent epic: e06" without specifying e06's parent path;
  no e06 bead id exists in the live `bd list` (UNKNOWN).

## Risks / Tags

- risk:temporal — none. proptests are deterministic on
  `tempfile::tempdir()` + `JournalWriteBatch::new`.
- risk:concurrency — none; tests are single-threaded.
- risk:unsafe-ub — none; `forbid(unsafe_code)` and no pointer
  arithmetic in any of the target tests.
- risk:persistence — covered indirectly through
  `tempfile::tempdir()`; Fjall journal tmpdir is fsynced at
  commit.
- risk:auth-security — none.
- risk:parser-codec — none; `JournalEvent::RunAccepted` is the
  only variant used in the 5 weak tests.
- risk:dependency — none; no Cargo.toml changes required.
- risk:performance — none; tests are microsecond-scale.
- risk:public-api — none; uses already-public batch API.
- risk:migration — none.
- risk:user-visible-behavior — none; tests are internal CI.
- risk:proof-binding — none; Kani harness
  `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59` models
  `DuplicateEvent { run, seq }` (with `r == run && s == seq`
  guards). The new test assertion (pinning exact `run`/`seq`)
  will STRENGTHEN the test↔Kani binding without changing either.
  No proof-agent action needed for this bead, but flag for
  proof-planner that the bind survives the test hardening.
- risk:audit-regression-resistance — TRUE for all 5 weak tests.
  This is exactly the bead's finding.

## Excluded Paths (deliberately not in scope)

- `crates/vb_storage/src/batch/t_append_event.rs` and the three
  `t_byte_accounting_partN.rs` files: their `matches!(_, DuplicateEvent { .. })`
  uses are weak, but they live inside the source tree (not the
  proptest fuzz/test lanes) and the bead title points at the
  proptest files (audit naming "Fuzz and tests finding"). Mark
  these as `OUT_OF_SCOPE — ADJACENT FINDING` and queue as
  follow-up if needed.
- `crates/workspace_tests/tests/journal_side_index_contracts.rs:495-531`
  uses `matches!(dup_result, Err(JournalError::DuplicateEvent { .. }))` (line 525)
  but already pins `batch.is_aborted()` and `batch.len() == 0` and
  asserts via separate `prop_assert_eq!` calls. The single match
  uses `..` but the surrounding props narrow the contract. Borderline.
  Recommend OUT_OF_SCOPE; the audit focus was "duplicate-event test
  accepts DuplicateEvent, any error, or Ok, making it unable to
  catch regressions" — the `journal_side_index_contracts` test
  also asserts `is_aborted`, `len == 0`, and exact event-count
  after commit (line 508-510), so it is not the regression-blind
  pattern. Leaving it alone is consistent with bead scope.
- `crates/vb_storage/src/tests.rs:837-851`
  `fn duplicate_event_append_is_rejected` — uses `..` (weak)
  but asserts only `DuplicateEvent` variant. Not in the
  "Fuzz and tests finding" lane; OOS.
- `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs`
  — variant-level fixture; different concern. OOS.
- `crates/workspace_tests/tests/vb_ko29_5_public_idempotency.rs:90-118`
  — already pins `run: got, seq` and asserts `got == run`. Strong
  test; do not touch. Listed as evidence the strong pattern
  exists across the workspace.
- All `crate = vb_storage::tests::proptest_journal_*` other than
  the PS_00x files in targets 1-5 above: OOS.
- `verification/**`, `fuzz/**`, `benches/**`, `xtask/**`,
  `crates/workspace_tests/**` (except as noted above): OOS.

## Downstream Owner Recommendations

- rust-contract → write the exact-match `requires`/`ensures` for
  `JournalWriteBatch::append_event`'s `DuplicateEvent` branch (uses
  values `run = event.run_id()`, `seq = event.seq()`). Bind the
  contract to production code at
  `crates/vb_storage/src/batch/append_event.rs:55-67` via
  `#[path = ...]` mirror (STRONG binding per master contract).
- proof-planner → Kani harness
  `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs` already models
  `DuplicateEvent { run: r, seq: s }` with `r == run && s == seq`
  guard. Test fix strengthens the runtime→proof alignment
  without requiring any new harness. Mark as
  "no new Kani required".
- test-writer / test-planner → for each of the 5 weak functions,
  replace `let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }));`
  with
  `assert!(matches!(result, Err(JournalError::DuplicateEvent { run: r, seq: s }) if r.get() == run && s.get() == seq));`
  (or proptest equivalent), preserving the existing
  `b1.append_event(&event).expect("first"); b1.commit().expect("commit");`
  setup. Preserve every other assertion in each function;
  do not delete `prop_assert_eq!(b2.len(), 0)` or
  `prop_assert!(b2.is_aborted())` if present.
- holzman-rust → verify the test-fix doesn't introduce any
  forbidden Rust construct. The new assertion only adds
  field-binding in a `matches!` guard; safe.
- black-hat/truth-serum → review focus: ensure that
  the strengthened assertions bind to values actually returned
  by production (not synthetic `RunId::new(0)` or
  `EventSeq::new(0)`), and that the proptest strategy still
  varies `run` and `seq` so the test remains regression-resistant
  across the 1u64..1000 × 0u64..100 input space.

## Open Discovery Items (NOT blocked, but flagged)

- None. Discovery is sufficient for downstream agents to start
  contracting/test-planning. The 5 weak test targets are exhaustively
  enumerated above; production contract is fully traced; the
  exact-match idiom is exemplified in 2 pre-existing tests.

## Reproduction Commands (for downstream run-and-evidence capture)

To exercise each weak test:

```
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001 ps001_duplicate_rejected --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003 ps003_dup_fields        --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_no_persist          --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_empty_commit_after_rej --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_008 ps008_dup_before_queue     --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_009 ps009_dup_rejected         --no-fail-fast
```

(The above commands are recorded for downstream agents; the
explore scout did NOT execute them — execution is the proof/execution
agent's responsibility. No runtime evidence was collected in
this scout pass.)
