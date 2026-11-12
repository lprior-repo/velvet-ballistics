# VB-NDX8B: Repair test-integrity checker — fail-closed per-file diff + hash-bound justifications

## Problem
`scripts/check-test-integrity.rs` (second draft, preserved at parent) has holes:

1. **Lossy header decode** — `hunk_test_path()` runs `String::from_utf8_lossy` over the whole
   diff hunk; non-UTF-8 relevant bytes are silently replaced instead of failing closed.
2. **Unchecked indexing** — `parts[0]`/`parts[1]` on the `diff --git a/... b/...` split.
3. **Silent error drops** — `scan_diff_bytes` does `String::from_utf8(hunk).ok()` then
   `Err(_) => continue`, silently skipping malformed relevant hunks; `validate_file_utf8`
   reads with `fs::read_to_string(...).ok()` (dropping the real UTF-8 error) and then compares
   `bytes.len() != text.len()`, a tautology on already-validated text.
4. **Rename-out bypass** — whole-repo `--find-renames` diff collapses a 100% rename to a
   header with no body hunks, so renamed-out tests can be missed.
5. **No binding justifications** — the advertised "equal-or-stronger replacement coverage or
   bead-linked justification" has no machine-checkable mechanism; findings can only be
   blanket-suppressed (which this repair must NOT do).
6. **Self-tests assert exit code only** — an unrelated error (e.g. setup failure surfacing as
   exit 1) can fake a pass; setup failures are swallowed by `.ok()`/`unwrap_or(2)`.

## Contract

### Diff selection (fail-closed)
- Changed files: `git diff --name-status -z --no-renames BASE --`, parsed as raw
  `[status]\0[path]\0` records (handles spaces/quoted paths; renames surface as `D old` + `A new`).
- Per-path diff: `git diff --no-renames --no-textconv --no-ext-diff --text --unified=0 BASE -- PATH`
  for each changed relevant path only. Unrelated files (including binary `.beads/evidence/**`)
  are never decoded as UTF-8.
- Relevance = `is_test_path(path) || base_file_has_tests(...)`. Non-relevant paths are skipped
  before any diff decode.
- Actual file contents (base version and working-tree version) are read as raw bytes and
  validated with strict `std::str::from_utf8`; NUL/FF binary in a relevant `.rs` file is a
  `NonUtf8TestFile` finding (never silently dropped).
- No `Err(_) => continue` on malformed relevant hunks; non-UTF-8 relevant diff is a finding.
- Rename-out = `D old(test)` + `A new(non-test)` → old path flagged `DeletedTestFile` +
  `DeletedTestDeclaration` + `WeakenedAssertion`.
- JJ fallback stays working via `jj diff --types` (robust `XY path` type codes, `{old => new}`
  brace expansion for renames) + per-path `jj diff --git`; unsupported cases report explicitly.

### Justifications (hash-bound, exact)
`scripts/test-integrity-justifications.tsv` — optional; absent = no exceptions. 11 tab columns:
`base_commit | old_path | old_blob | new_path | new_blob | finding_kind | finding_detail |
replacement_paths | review_artifact_path | review_artifact_blob | bead_id`.

- Blobs are git SHA-1: base via `git rev-parse BASE:path`, current via `git hash-object path`;
  `ABSENT` for a file that does not exist on that side. Resolved via read-only git.
- A record justifies a finding only when ALL of: base commit, old path, finding kind, exact
  finding detail, old blob, new blob, review-artifact blob, and every replacement-path blob match.
- Reject at load: malformed columns, non-40-hex/ABSENT blobs, duplicate identity, and
  forbidden kinds. `NonUtf8TestFile` is never justifiable; an `IgnoredOrSkippedTest` whose
  detail is a code-level ignore attribute (`#[ignore`/`#[cfg_attr`) or silent `return`/`skip`
  requires a non-empty replacement-path binding (no failing-assert → no justification).
- Unknown finding/path cannot bypass: a record whose identity matches no finding is a hard error.
- Stale hash (changed test/replacement/review/base) → finding stays un-justified → FAIL.
- Justified findings print `JUSTIFIED|kind|path|detail` (visible, raw finding retained) and are
  removed from the failing set only; un-justified findings still FAIL.

### Self-tests (exact, propagating)
Assert exact expected finding kind+path (not mere exit code). Setup failures propagate (no
`.ok()`/`unwrap_or` masking). Use real temp git commits via an ELF-resolved git binary (skip
guard wrapper scripts), never the project VCS.

### Exit codes
`0` pass · `1` un-justified findings · `2` error (malformed/stale/forbidden justification,
invalid base, non-UTF-8 changed path) · `64` root not found.

## Acceptance
- `--self-test` passes (all exact-finding cases + justification mechanism cases on temp fixtures).
- Production gate with `TEST_INTEGRITY_BASE=da633fe0bbde9e414173d5f7aabe314a37f8752d` FAILs with
  the same 17 findings as `.beads/vb-ndx8b/evidence/real-origin-base-v2.txt` (no suppression).
- No integration justifications are populated yet (final review + Ask fix vb-b24vj pending).
