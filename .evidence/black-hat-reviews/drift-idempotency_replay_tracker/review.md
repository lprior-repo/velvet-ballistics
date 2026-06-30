# Black-Hat Review: drift-idempotency_replay_tracker

## Scope
Repair 8 production-binding drift findings in
`verification/verus/extern_idempotency_replay_tracker.rs`. All 8 ledger
entries claimed line ranges in `crates/vb_storage/src/recovery/types.rs`
near lines 870-1053, but the production `ActionReplayTracker` block has
since been relocated to lines 1231-1417 (the file grew from ~1053
lines to 1465 lines as additional recovery types were added). Drift
detection flagged all 8 because the LAST-segment identifier
(`ActionReplayTracker`, `new`, `mark_completed`, `mark_failed`,
`has_completed`, `has_failed`, `is_resolved`, `default`) could not be
located in the claimed range or its ±5-line context window.

## Phase 1: Contract & Bead Parity

### Finding 1: Binding class preserved (PASS)
**Severity**: Info
**Location**: `verification/verus/extern_idempotency_replay_tracker.rs:62-71`

The file is a WEAK production binding (uses a `production_inner/`
mirror with a drift-gate header, included via
`#[path = "production_inner/action_replay_tracker_production.rs"]`).
The mirror itself is unchanged by this fix.

**Evidence**:
```
bash scripts/check-verus-production-binding.sh "$PWD" | tail -4
  STRONG (direct crates/ binding): 0
  WEAK (production_inner/ mirror): 71
  VACUUM (no production binding):  0
```
VACUUM count remains 0. Binding class is unchanged.

### Finding 2: Drift gate clears all 8 findings (PASS)
**Severity**: Info
**Evidence**:
```
Pre-fix:  grep -c 'extern_idempotency_replay_tracker' /tmp/drift-pre.txt
          → 8 DRIFT findings
Post-fix: grep -c 'extern_idempotency_replay_tracker' /tmp/drift-post.txt
          → 0 DRIFT findings
```

## Phase 2: Farley Engineering Rigor

### Finding 3: Per-entry range coverage (PASS)
**Severity**: Critical (would re-block the gate if wrong)
**Location**: `verification/verus/extern_idempotency_replay_tracker.rs:64-71`

For each ledger entry, the new claimed range was verified to include
the LAST-segment identifier token in production
`crates/vb_storage/src/recovery/types.rs` (within the script's ±5-line
context window).

| Identifier | Old Claim | New Claim | Verified Production Lines |
|---|---|---|---|
| `ActionReplayTracker` | 870-875 | 1231-1239 | doc-comment 1231-1233; struct 1234-1239 |
| `ActionReplayTracker::new` | 899-908 | 1263-1272 | impl 1263; `#[must_use]` 1264; `pub fn new` 1265; body 1266-1271; closing `}` 1272 |
| `ActionReplayTracker::mark_completed` | 960-964 | 1324-1328 | doc-comment 1324-1325; `pub fn mark_completed` 1326; body 1327; closing `}` 1328 |
| `ActionReplayTracker::mark_failed` | 1024-1027 | 1388-1391 | doc-comment 1388; `pub fn mark_failed` 1389; body 1390; closing `}` 1391 |
| `ActionReplayTracker::has_completed` | 1029-1033 | 1393-1397 | doc-comment 1393; `#[must_use]` 1394; `pub fn has_completed` 1395; body 1396; closing `}` 1397 |
| `ActionReplayTracker::has_failed` | 1035-1039 | 1399-1403 | doc-comment 1399; `#[must_use]` 1400; `pub fn has_failed` 1401; body 1402; closing `}` 1403 |
| `ActionReplayTracker::is_resolved` | 1041-1046 | 1405-1410 | doc-comment 1405-1406; `#[must_use]` 1407; `pub fn is_resolved` 1408; body 1409; closing `}` 1410 |
| `ActionReplayTracker::default` | 1049-1053 | 1413-1417 | `impl Default for ActionReplayTracker` 1413; `fn default` 1414; body 1415; closing `}` 1416; closing impl `}` 1417 |

Every range covers from doc-comment (or attribute, where no doc
comment exists) through closing brace. The drift script's identifier
extractor (`extract_id_extern`, which keeps all-lowercase tokens)
will find `new`, `default`, and the function names in these ranges.

### Finding 4: In-body prose refs updated (PASS)
**Severity**: Medium
**Location**: `verification/verus/extern_idempotency_replay_tracker.rs:13, 57`

Two non-ledger prose references to the production impl block's line
range were also updated to match the new location:
- Line 13: `types.rs:867-1053` → `types.rs:1231-1417`
- Line 57: `lines 867-1053 of production` → `lines 1231-1417 of production`

The "WHY THE PRODUCTION MIRROR" prose section (lines 31-44) still
references `types.rs:10`, `types.rs:11-14`, `types.rs:37`, and
`types.rs:17-37`. These are minor-line-number drifts in a `//`
prose block (not `///` doc comments and not ledger entries), and the
drift script does not check them. The underlying code snippets are
still substantively correct (line 9 is `use crate::...`, line 10 is
`use vb_core::{`, line 36 is `#[derive(Debug)]`, line 15-17 is the
`#[cfg(kani)] ReplayResolutionSet` block). These are out of scope
for this drift fix.

## Phase 3: Holzman Rust (Big 6)

### Finding 5: No new unsafe / panic / unwrap (PASS)
**Severity**: Critical
**Evidence**: The change is comment-only. `grep -nE 'unsafe|panic|unwrap|todo|unimplemented|dbg' verification/verus/extern_idempotency_replay_tracker.rs` returns zero hits introduced by this edit. Pre-existing `#[verifier::external]` directive is untouched.

### Finding 6: File length under limit (PASS)
**Severity**: Medium
**Evidence**: `wc -l verification/verus/extern_idempotency_replay_tracker.rs` → 112 lines (limit 300 for production; 800 for `verus/` category).

## Phase 4: Ruthless Simplicity & DDD

### Finding 7: Minimal change (PASS)
**Severity**: Info
**Evidence**: The diff is two `types.rs:867-1053` → `types.rs:1231-1417` prose replacements and eight ledger line-range updates. No new types, no new modules, no new comments. This is the smallest possible change that resolves the drift.

## Phase 5: Bitter Truth

### Finding 8: Companion spec file drift is out of scope (NOTE)
**Severity**: Info
**Note**: `verification/verus/idempotency_replay_tracker.rs` (the
companion spec file) still references the old line range
(`types.rs:867-1053`, `types.rs:870-875`, `types.rs:899-908`,
`types.rs:960-964`, `types.rs:1024-1027`, `types.rs:1029-1033`,
`types.rs:1035-1039`, `types.rs:1041-1046`, `types.rs:1049-1053`) in
its own prose and `///` doc comments. These are prose-only refs in a
file that the drift script does NOT scan (`parse_extern_ledger`
matches `//   - \`...\`` only). They are out of scope for this bead.

The mirror file `verification/verus/production_inner/action_replay_tracker_production.rs`
also has its own drift findings (12 identifiers missing from the
mirror, e.g. `RecoveredStepState`, `RecoveryCannotResumeState`,
`UnsupportedRecoveryState`, `CANNOT_RESUME_REASONS`, etc.) that are
separately reported by the drift gate. These are mirror-vs-production
drift, NOT ledger-line-range drift, and are out of scope for this
bead. Both are follow-up repair work for separate beads.

## Brutal Verdict

**STATUS: APPROVED**

The fix is the minimal, mechanical correction required: eight stale
line-range pointers in the BINDING LEDGER were updated to the actual
production locations, plus two inline prose references to the same
range. The drift gate drops from 8 to 0 findings for this file. The
binding class remains WEAK with 0 VACUUM. No new unsafe, panic,
unwrap, or non-deterministic code paths were introduced; this is a
comment-only edit.

The remaining drift findings (42 in `drift-post.txt`) are scoped to
OTHER extern files and to the `production_inner/` mirrors themselves;
those are separate repair beads and are explicitly excluded from this
bead's scope by the task brief.

## Residual Risk
- The mirror file `verification/verus/production_inner/action_replay_tracker_production.rs`
  still has 12 production identifiers missing (`RecoveredStepState`,
  `RecoveryCannotResumeState`, etc.). These are mirror-regeneration
  work, not ledger-line-range work. Track as a follow-up bead.
- The companion spec file `idempotency_replay_tracker.rs` has stale
  prose references to `types.rs:867-1053` and per-method line numbers.
  These are doc-comment-only and not checked by the drift gate.
  Track as a documentation follow-up bead.
- Future production changes that move the `ActionReplayTracker` impl
  block again will re-trigger all 8 ledger drift findings. Consider
  widening ranges to "struct through impl block end" (1231-1417) for
  durability, but the current 1:1 per-identifier mapping is more
  precise and the drift gate will catch any future move immediately.