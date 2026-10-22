# vb-s4hfc Handoff

## Scope
Corrective pass: rewrote `crates/vb_storage/tests/policy_decision_wire.rs` to eliminate all repository-rule violations (panic!, unwrap(), unchecked indexing) while preserving all 7 test behaviors and strengthening assertions via `JournalEvent`'s derived `PartialEq`.

## Files Changed
- `crates/vb_storage/tests/policy_decision_wire.rs` (only file changed)

## Tests
All 7 tests pass:
- `test_policy_layers_all` - round-trips all 4 PolicyLayer values
- `test_decision_codes_all` - round-trips all 3 DecisionCode values
- `test_flag_permutations` - round-trips all 4 flag combinations
- `test_fjall_journal_round_trip` - Fjall append/drop/reopen/read with exact equality
- `test_multiple_events_distinct_digests` - multiple ordered events preserve distinct digests/fields (seq 0 and 1)
- `test_malformed_payload` - truncated payload returns error (asserted via is_err())
- `test_legacy_tag_stability` - RunAccepted serializes with tag byte 10, round-trips exactly

## Verification Commands and Exit Codes (V4-FRESH raw gates)
- `python3 /tmp/vb-run-gate.py 120 .beads/vb-s4hfc/evidence/V4-FRESH-1 cargo test -p vb_storage --test policy_decision_wire` → exit 0, 7 passed (V4-FRESH-1.json, V4-FRESH-1.stdout)
- `python3 /tmp/vb-run-gate.py 90 .beads/vb-s4hfc/evidence/V4-FRESH-2B cargo clippy -p vb_storage --lib` → exit 0, no warnings (V4-FRESH-2B.json, V4-FRESH-2B.stderr). Note: `-D warnings` flag is not passed through by the gate script, but clippy ran clean with exit 0 and no warning output.
- `python3 /tmp/vb-run-gate.py 60 .beads/vb-s4hfc/evidence/V4-FRESH-3B cargo +1.89.0 fmt --check --all` → exit 1 (V4-FRESH-3B.json). My edited file `crates/vb_storage/tests/policy_decision_wire.rs` is fmt-clean (0 matches in V4-FRESH-3B.stdout). Remaining diffs are pre-existing in `crates/vb_runtime/src/policy.rs`, `crates/vb_runtime/src/lib.rs`, `crates/vb_runtime/src/error/diagnostics.rs` — not part of this bead's scope.
- `python3 /tmp/vb-run-gate.py 120 .beads/vb-s4hfc/evidence/V4-FRESH-4 cargo test -p vb_storage --test policy_decision_wire` → exit 0, 7 passed (re-run after formatting fix; V4-FRESH-4.json)

## Design Decisions
- Replaced match/panic variant checks with `assert_eq!(decoded, event)` (stronger: verifies ALL fields)
- Replaced indexed read verification with `assert_eq!(read_events, vec![event])` or `assert_eq!(read_events, events)`
- Malformed payload check uses `assert!(result.is_err())`
- Tag stability check uses `wire.first().copied()` against `Some(10)` (actual RunAccepted tag)
- No panic!, unwrap(), expect(), unchecked indexing, or unchecked arithmetic remain
- File is 232 lines (under 300 limit)
- Old tag fixture preserved (RunAccepted tag 10)
- Tinycorrection: `test_malformed_payload` truncated-payload slicing now uses `checked_sub(4)` + `.get(..end).ok_or(...)?` instead of unchecked `wire[0..wire.len()-4].to_vec()` — no unchecked subtraction or slicing

## Parent Acceptance Context
Codec-only scope on the old JournalEvent shape. Full identity/pinned policy/recovery/dispatch parent acceptance is tracked separately.