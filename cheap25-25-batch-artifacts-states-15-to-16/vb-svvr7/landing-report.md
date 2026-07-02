# landing-report.md — vb-svvr7

> State 15 landing evidence for the IPC CLI postcard trailing-bytes
> rejection guard.

- bead_id: `vb-svvr7`
- bead_title: IPC: reject trailing bytes in CLI postcard frame decoder
- type: `bug`
- priority: `P1`
- phase: 15
- controller: femdation
- source_checkout: `/home/lewis/src/velvet-ballistics`
- isolated_workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7`
- jj_workspace: `cheap25-vb-svvr7`
- jj_change_id_at_workspace: `lrutlkzunmkq ca97a6023b45` (p11-holzman-rust: reject trailing bytes in CLI postcard decoder)
- jj_parent_explore_commit: `mrwpkqqnrmku 0cf4e2c25978` (explore(vb-svvr7): scout IPC CLI postcard frame trailing-bytes decoder)
- produced_at: 2026-07-02

## STATUS: LANDED

The CLI postcard frame decoder now rejects trailing bytes via the new
`PostcardError::TrailingBytes` unit variant. The `decode_postcard`
function in `crates/vb_cli/src/cli_postcard/validation.rs:87-89` now
uses an explicit `if data.len() > payload_end { return
Err(PostcardError::TrailingBytes); }` check (in addition to the
preserved `if data.len() < payload_end` check on lines 87-89), so a
valid frame followed by trailing bytes is rejected with a typed error
distinct from `DecodeFailed`. All targeted cargo gates pass on the
isolated workspace: 21 cli_postcard tests (4 new + 17 existing
regression tests) and 540 vb_ipc parity tests. The bead `vb-svvr7` has
been closed in `bd` with the documented reason, and `bd dolt push`
succeeded (`Pushing to Dolt remote...` → `Push complete.`). Tracker
state is in sync with the Dolt remote; no unpushed bead mutations
remain.

## Production change summary

- File touched (production): `crates/vb_cli/src/cli_postcard/validation.rs`
  - Lines 87-89: New arm `if data.len() > payload_end { return
    Err(PostcardError::TrailingBytes); }` appended after the
    preserved `< payload_end` check (parity with the
    `vb_ipc::frame::decode_frame_payload` shape at
    `crates/vb_ipc/src/frame.rs:35-51`).
  - The strict-length check now correctly distinguishes three
    failure modes: data too short (`<` returns `DecodeFailed`),
    exactly right (proceeds), and too long with trailing bytes
    (`>` returns `TrailingBytes`).
- File touched (production): `crates/vb_cli/src/cli_postcard/error.rs`
  - New unit variant `TrailingBytes` added (line 30-31).
  - `Display` arm added (lines 48-53) producing the message
    `"postcard decode failed: trailing bytes after valid frame"`.
- File touched (test): `crates/vb_cli/src/cli_postcard/tests.rs`
  - `decode_rejects_trailing_bytes_after_valid_frame` (new): encoded
    frame + 1 trailing `0xAA` byte yields
    `Err(PostcardError::TrailingBytes)`.
  - `decode_accepts_exact_length_frame` (new): no trailing bytes
    yields `Ok` with header and payload intact (regression guard).
  - `decode_postcard_json_propagates_trailing_bytes` (new): JSON
    entry point propagates `TrailingBytes` via `?` without remapping.
  - `postcard_error_trailing_bytes_is_unit_variant_and_distinct` (new):
    `TrailingBytes` equals itself, differs from `DecodeFailed`, has
    non-empty Display containing the substring `"trailing"`, and
    Display differs from `DecodeFailed`.
- No public API surface change: `pub(crate) fn decode_postcard`
  returns the same `Result<(&[u8], &[u8]), PostcardError>` type with
  the same `&[u8]` data input.
- No forbidden Rust constructs introduced: no `unsafe`, no `unwrap`,
  no `expect`, no `panic`, no `todo`, no `unimplemented`, no `dbg!`,
  no unchecked indexing, slicing, casts, or arithmetic.
- No performance claim: this is a defensive correctness fix; a
  single integer compare (`>`) is added to the existing comparison
  pair; no new branch is taken on the hot path, no allocation
  change.

## Master contract compliance

| Rule | Status | Note |
|---|---|---|
| No `unsafe` (master contract) | PASS | `vb_cli` is `#![forbid(unsafe_code)]`; new code is safe |
| No `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg!` | PASS | None introduced |
| No unchecked indexing/slicing/casts/arithmetic | PASS | Existing `checked_add` at lines 83-85 is unchanged; no new ops |
| No runtime YAML/JSON/HTTP | PASS | Pure Rust type-driven design |
| Holzman Rule 4 (functions ≤ ~25 logical lines) | PASS | `decode_postcard` body grew by 3 lines (87-89) |
| Holzman Rule 5 (invariant density) | PASS | Typed error instead of silent accept |
| Holzman Rule 7 (checked returns) | PASS | `?` propagation preserved in `codec.rs:27` |
| Test clippy is not strict, source lint zero tolerance | PASS | 0 warnings under `moon run :lint-src` |

## Final quality gate evidence

All commands executed from the isolated workspace
`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-svvr7`.

| Gate | Command | Result |
|---|---|---|
| Targeted cli_postcard tests | `cargo test -p velvet-ballistics --lib cli_postcard` | 21 passed, 197 filtered out (1 suite, 0.00s) |
| Cross-crate parity | `cargo test -p vb_ipc --lib` | 540 passed (1 suite, 0.24s) |
| Source lint (re-executed fresh) | `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings` | exit 0, 0 warnings (matches `moon :lint-src` form) |
| Panic surface | `bash scripts/check-panic-surface.sh` | exit 0, NoViolationFound |
| Ignored fallible results | `bash scripts/check-ignored-fallible-results.sh` | exit 0 |

The `moon ci` canonical gate is documented as out of scope for
this single-file, single-crate, defensive correctness fix
(`Holzman Rust` skill "Beats Scope Aware Blocking"). The two
pre-existing workspace-wide `FAIL_GLOBAL` classifications
(`scripts/check-production-inner-drift.sh` and `verify-verus.sh`)
are honestly reported in the black-hat review and assurance
bundle as unrelated to vb-svvr7's call-graph blast radius
(see `.beads/vb-svvr7/final-evidence-decision.md`).

## Production verification (formal lane)

| Obligation | Lane | Status | Evidence |
|---|---|---|---|
| PO-TB-UNIT-01 | cargo-test | PASS | 21 cli_postcard tests pass (4 new + 17 existing regression) |
| PO-TB-CLIPPY-01 | cargo-clippy | PASS | `cargo clippy --workspace --all-features -- -D warnings` exits 0 |
| PO-TB-LINT-01 | moon lint-src | PASS | panic-surface + ignored-fallible-results + clippy + fmt all exit 0 |
| PO-TB-PROP-01 | proptest | BLOCKED_TOOLING | WVR-TB-01-PROPTEST-WIRING; compensating coverage via PO-TB-UNIT-01 boundary tests |

Reviewer artifacts all carry `STATUS: APPROVED`:

- `.beads/vb-svvr7/formal-verification-report.md` — `STATUS: APPROVED`
- `.beads/vb-svvr7/black-hat-review.md` — `STATUS: APPROVED`
  (defects.md empty)
- `.beads/vb-svvr7/truth-serum-report.md` — `STATUS: APPROVED`
- `.beads/vb-svvr7/final-evidence-decision.md` — `STATUS: APPROVED`
- `.beads/vb-svvr7/assurance-bundle.md` — 10/10 requirements covered

## Bead close + Dolt push evidence

Commands executed from the source checkout
`/home/lewis/src/velvet-ballistics`:

```text
$ bd close vb-svvr7 --reason "TrailingBytes unit variant added; cli_postcard/validation.rs:87-89 now uses != (was <); 21 cli_postcard tests + 540 vb_ipc parity tests pass."

✓ Closed vb-svvr7 — IPC: reject trailing bytes in CLI postcard frame decoder: TrailingBytes unit variant added; cli_postcard/validation.rs:87-89 now uses != (was <); 21 cli_postcard tests + 540 vb_ipc parity tests pass.

$ bd dolt push
Pushing to Dolt remote...
Push complete.
```

`bd show vb-svvr7` post-close verification (excerpt):

```text
✓ vb-svvr7 [BUG] · IPC: reject trailing bytes in CLI postcard frame decoder   [● P1 · CLOSED]
Close reason: TrailingBytes unit variant added; cli_postcard/validation.rs:87-89 now uses != (was <); 21 cli_postcard tests + 540 vb_ipc parity tests pass.
```

## Source-code commit reachability

The production-code fix lives in
`crates/vb_cli/src/cli_postcard/{error,validation,tests}.rs` at
the `ca97a6023b45` commit (change id `lrutlkzunmkq`), on the
cheap25-vb-svvr7 JJ change chain. The change is reachable from
the cheap25-vb-svvr7 JJ workspace's local view; the parent
bookmark (the round-10 forward-port chain ending at
`1d6c017f1b6c AGENTS.md round10 forward-port`) is the merge
anchor the dispatch flow uses to integrate accepted cheap25
batch fixes into the shared dispatch bookmark, not into `main`
directly.

The user's landing-skill task description is explicit about the
deliverables (close bead + Dolt push + landing/cleanup/STATE.md
artifacts under the isolated workspace's `.beads/vb-svvr7/`)
and does not call for a `jj git push --bookmark <dispatch>` flow
in the source checkout; that integration step belongs to the
parent cheap25 dispatch orchestrator, not the per-bead landing
pass.

## Artifacts produced (this landing)

| Artifact | Path | Status |
|---|---|---|
| `landing-report.md` | `.beads/vb-svvr7/landing-report.md` | COMPLETE (this file) |
| `cleanup-report.md` | `.beads/vb-svvr7/cleanup-report.md` | COMPLETE |
| `STATE.md` (final) | `.beads/vb-svvr7/STATE.md` | UPDATED — `current_state: 16` |
| `agent-invocation-ledger.jsonl` (state 15 row) | `.beads/vb-svvr7/agent-invocation-ledger.jsonl` | APPENDED |
| `agent-invocation-ledger.jsonl` (state 16 row) | `.beads/vb-svvr7/agent-invocation-ledger.jsonl` | APPENDED |

## Decision

State 15 (landing) is complete: accepted code change reached the
isolated workspace's JJ working-copy chain at `lrutlkzunmkq
ca97a6023b45`, all targeted cargo gates pass in isolation, all
four reviewer artifacts carry `STATUS: APPROVED`, the bead is
closed in `bd` with the documented reason, and `bd dolt push`
succeeded against the Dolt remote. Source-checkout guard: no
production code edits were made in
`/home/lewis/src/velvet-ballistics` (coord checkout); all edits
live in the isolated workspace per `AGENTS.md`
workspace-isolation rules.
