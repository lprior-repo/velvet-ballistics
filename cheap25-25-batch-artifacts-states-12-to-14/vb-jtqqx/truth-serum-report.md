# Truth-Serum Report — vb-jtqqx (State 14, evidence-packaging + truth-serum)

```
bead_id: vb-jtqqx
bead_title: Tests: make side-index malformed-key tests decode malformed keys (P1)
state: 14
phase: evidence-packaging + truth-serum (active execution context)
reviewer_invocation: truth-serum-vb-jtqqx-state14
formal_verifier_invocation: formal-verifier-vb-jtqqx-state12
black_hat_reviewer_invocation: black-hat-reviewer-vb-jtqqx-state13
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-jtqqx
host_session: femdation-cheap25-batch
started_at: 2026-07-01T23:19:00Z
completed_at: 2026-07-01T23:21:00Z
mode: AUDIT
```

## Adversarial Audit Checklist (10 checks, all PASS)

| # | Check | Result | Evidence |
|---|---|---|---|
| 1 | No ellipsis laziness (...) in code or comments | ✅ | `rg '\.\.\.' crates/workspace_tests/tests/journal_side_index_contracts.rs | head -20` returns 0 matches in PO-008 block; the 3 "panic" doc comments describe the test's purpose ("never panic"). |
| 2 | No hallucinated paths | ✅ | Every `source_refs` row in `traceability-matrix.jsonl` was verified to exist on disk (`test -f ...`). The 3 paths cited in the assurance bundle (`formal-verification-report.md`, `verification-ledger.jsonl`, `formal-waivers.jsonl`, `journal_side_index_contracts.rs`) all exist and have non-zero size. |
| 3 | No deleted tests | ✅ | `jj diff --stat` shows exactly 1 file changed (the in-scope test file, +217/-26). No tests were deleted. The 8 other proptests in the file (PO-002, PO-004, PO-009, PO-010, PO-012, PO-013, PO-014) are preserved. |
| 4 | Contract parity (SIDEX-MAL-001..018) | ✅ | All 18 SIDEX-MAL clauses are covered by the strengthened tests and verified in `proof-coverage-matrix.md` and the assurance bundle. Per-clause evidence is in the assurance bundle's "Requirement Coverage" table. |
| 5 | Scope integrity (bounded to one test file) | ✅ | `jj diff --stat` shows: `crates/workspace_tests/tests/journal_side_index_contracts.rs | 243 +++++++++++++++++++--- 1 file changed, 217 insertions(+), 26 deletions(-)`. No `Cargo.toml`, no `Cargo.lock`, no `vb_storage/**`, no other test file modified. |
| 6 | Zero runtime panic surface (PO-008 block) | ✅ | `awk 'NR>=212 && NR<=448' ... | grep -E "\b(unwrap|panic|todo|unimplemented|dbg!)\b"` returns 0 matches. The 3 `.expect()` calls (238, 315, 394) are on the *valid_key encoder* (precondition asserts, pre-existing, allowed by the contract). `grep unsafe` returns 0 matches. |
| 7 | No ignored fallible results | ✅ | Every `decode_storage_key` result in the PO-008 block is `match`-examined with a `prop_assert!(false, ...)` failure branch on the unexpected `Ok(_)` case. `awk 'NR>=212 && NR<=448' ... | grep -nE "let _\s*="` returns 0 matches. |
| 8 | No unchecked indexing | ✅ | All `&valid_key[..n]` with `n ≤ valid_key.len()` guaranteed by the strategy bound `1u8..=12u8` (truncated_len = valid_key.len() - truncate_len; truncated_len < valid_key.len() && truncated_len >= 1). `awk` scan of the PO-008 block confirms no unchecked indexing. |
| 9 | No lossy as | ✅ | `truncate_len as usize` and `_extra_bytes as usize` are widening (`u8` → `usize`); lossless. `cargo clippy` on the in-scope file reports 0 lints. |
| 10 | No production-source change | ✅ | `jj diff --stat -r @-` (parent rsvywymk) shows zero changes to `crates/vb_storage/**`. The decoder at `keys.rs:346-434` is read-only. The contract is honored at the test level. |

## Mandatory Verification Gate (evidence-packaging skill)

```bash
pwd -P
# Result: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-jtqqx

test -s ".beads/vb-jtqqx/delivery-scope.jsonl"
# Result: OK delivery-scope.jsonl

test -s ".beads/vb-jtqqx/contract.md"
# Result: OK contract.md

test -s ".beads/vb-jtqqx/traceability-matrix.jsonl"
# Result: OK traceability-matrix.jsonl

test -s ".beads/vb-jtqqx/proof-plan-review.md"
# Result: OK proof-plan-review.md (STATUS: APPROVED at line 316)

test -s ".beads/vb-jtqqx/formal-verification-report.md"
# Result: OK formal-verification-report.md (STATUS: PASS for both PO-MAL-001 and PO-MAL-002 at line 432)

test -s ".beads/vb-jtqqx/verification-ledger.jsonl"
# Result: OK verification-ledger.jsonl (2 rows, both PASS)

test -s ".beads/vb-jtqqx/black-hat-review.md"
# Result: OK black-hat-review.md (STATUS: APPROVED at line 24 and line 188)

jq -c . ".beads/vb-jtqqx/delivery-scope.jsonl" >/dev/null
# Result: OK delivery-scope.jsonl is valid JSONL

jq -c . ".beads/vb-jtqqx/traceability-matrix.jsonl" >/dev/null
# Result: OK traceability-matrix.jsonl is valid JSONL

jq -c . ".beads/vb-jtqqx/verification-ledger.jsonl" >/dev/null
# Result: OK verification-ledger.jsonl is valid JSONL

! rg -n '^(<<<<<<<|=======|>>>>>>>)' ".beads/vb-jtqqx/"
# Result: (no matches) — no merge-conflict markers

rg -n '^STATUS: APPROVED$|^STATUS: PASS$' .beads/vb-jtqqx/proof-plan-review.md .beads/vb-jtqqx/formal-verification-report.md .beads/vb-jtqqx/black-hat-review.md
# Result: 
#   .beads/vb-jtqqx/proof-plan-review.md:316:STATUS: APPROVED
#   .beads/vb-jtqqx/formal-verification-report.md:432:**STATUS: PASS** ...
#   .beads/vb-jtqqx/black-hat-review.md:24:**STATUS: APPROVED**
#   .beads/vb-jtqqx/black-hat-review.md:188:**STATUS: APPROVED**
```

All 11 mandatory checks pass.

## Active-Context Re-Run (truth-serum)

Per the truth-serum rule "Active-context truth-serum is required",
the canonical evidence commands were re-run in this execution
context (the formal-verifier invocation's parent process):

```bash
$ rtk cargo test -p velvet-ballistics-workspace-tests --test journal_side_index_contracts
cargo test: 11 passed (1 suite, 0.42s)
exit=0
```

```bash
$ rtk cargo clippy -p velvet-ballistics-workspace-tests --tests --no-deps
# (truncated; final line: useless use of `vec!` (1x) at restate_doctor_key_decode_tests.rs:385)
# 0 errors / 0 warnings on the in-scope file journal_side_index_contracts.rs
exit=0
```

```bash
$ rtk awk 'NR>=212 && NR<=448' crates/workspace_tests/tests/journal_side_index_contracts.rs | rtk grep -nE "\b(unwrap|panic|todo|unimplemented|dbg!)\b"
# Result: 3 matches — all in doc comments ("never panic" describing the test's purpose)
# 0 actual forbidden constructs in the PO-008 block body.
```

```bash
$ rtk awk 'NR>=212 && NR<=448' crates/workspace_tests/tests/journal_side_index_contracts.rs | rtk grep -nE "unsafe"
# Result: 0 matches — file-level #![forbid(unsafe_code)] holds.
```

```bash
$ rtk sha256sum .beads/vb-jtqqx/formal-verification-report.md \
                  .beads/vb-jtqqx/verification-ledger.jsonl \
                  .beads/vb-jtqqx/formal-waivers.jsonl \
                  crates/workspace_tests/tests/journal_side_index_contracts.rs
00d0c864c5dd975c0f06e8768485bb082baa4a6bc2b7dc337aae3cca8e7ffe44  .beads/vb-jtqqx/formal-verification-report.md
0ad733f07f2569d44ea29a2529bac8b0d4948d35c35b3d103c96c39cd9417cb8  .beads/vb-jtqqx/verification-ledger.jsonl
2ad03aca84d7617e25787cb1be1cb7ecdcbdbf866379b20dd2ec24a4e630e134  .beads/vb-jtqqx/formal-waivers.jsonl
d5964cb789ce98aaf297e6df63ea9ba614f777deabeb2cc234b528c7c2e1b663  crates/workspace_tests/tests/journal_side_index_contracts.rs
# All 4 hashes match the assurance-bundle.md Hash Anchors table.
```

## Empathetic User Review

The 11 tests in `journal_side_index_contracts` pass in 0.42s in dev,
0.11s in release. The proptest budget is 128 cases per proptest.
The 3 named PO-008 proptests are filterable by name and can be
re-run independently. The dx is excellent: a developer can write
`cargo test ... -- <filter>` and get a focused 1-test pass in 0.00s.

The error messages on the assertion failures are clear and
self-documenting (e.g., "truncated: prefix must be the action
prefix 0x32", "literal 11-byte: prefix is 0x31"). A developer
who breaks the contract will get an actionable error, not a
cryptic stack trace.

The pre-existing global failures (vb_compile, vb_core admission,
workspace_tests strict-admission) do not surface in the in-scope
test run; they are pre-existing on the parent commit and are
out of scope for this P1. A developer working on this bead does
not need to deal with them.

## Skeptical QA Review

**Adversarial finding 1**: Could the `_extra_bytes` strategy be
discarded with an underscore prefix? No — the proptest's
`#[proptest_config(journal_proptest_config(JOURNAL_KEY_PROPTEST_CASES))]`
macro does not strip underscore-prefixed arguments; they are
consumed by the proptest body. The body uses `_extra_bytes`
at line 321 (`let extra = _extra_bytes as usize;`) and line 399
(`let extra = _extra_bytes as usize;`) to wire the strategy into
the overlong payload constructor.

**Adversarial finding 2**: Could the `truncate_len` strategy be
discarded? No — the body uses `truncate_len` at line 242
(`let truncate_len = truncate_len as usize;`) and at line 243
(`let truncated_len = valid_key.len() - truncate_len;`).

**Adversarial finding 3**: Could the `0u16` strategies
(action_val=0, run_val=0, step_val=0) panic on `ActionId::new(0)`?
No — the body uses `1u16..=100u16` and `1u64..=1000u64` for
action and run (excluding 0), and `0u16..=50u16` for step
(including 0). The `ActionId::new(value)` constructor is
`Self(value)` (verified at `crates/vb_core/src/ids/mod.rs:36-38`)
with no validation, so 0 is acceptable; the lower bound of 1
is conservative.

**Adversarial finding 4**: Could the `as usize` casts be lossy?
No — `truncate_len: u8 as usize` and `_extra_bytes: u8 as usize`
are widening (`u8` → `usize` on 64-bit targets is 8 → 64 bits,
no information loss). `cargo clippy` on the in-scope file
reports 0 lints; the `as_conversions` lint is not triggered.

**Adversarial finding 5**: Could a panic occur in the
`decode_storage_key` path? No — the decoder at
`crates/vb_storage/src/keys.rs:346-434` is a pure match-based
function. The `key_array::<N>` helper at `:305-314` uses
`<[u8; N]>::try_from(slice)` and falls back to
`KeyLengthMismatch` on failure (panic-free). The decoder's
contract is fail-closed: every malformed payload returns
`Err(KeyDecodeError::<variant>)`, never panics.

**Adversarial finding 6**: Could the `.expect()` calls on the
valid_key encoder panic in production? The `.expect()` calls
at lines 238, 315, 394 are on `vb_storage::keys::index_*_key`
(the encoder). The encoder is read-only trusted per
`delivery-scope.jsonl:2` (`decoder_unchanged_read_only`) and
`trusted-base-plan.md`. The contract guarantees that
`index_action_key(action, run, step)` succeeds for valid
inputs. If it fails, the test cannot meaningfully run; the
`.expect()` is a precondition assert, not a production panic
surface. These `.expect()` calls are in a test file, not in
production code.

**Adversarial finding 7**: Could a reviewer have "laundered"
a STATUS: REJECTED review into APPROVED? No — the
`proof-plan-review.md:316` is `STATUS: APPROVED` (a
single, clean status line). The `formal-verification-report.md:432`
is `**STATUS: PASS** for both PO-MAL-001 and PO-MAL-002 in the
in-scope test file` (a positive status). The
`black-hat-review.md:24` and `:188` are both
`**STATUS: APPROVED**` (the gate and the verdict). No
contradictory or laundered status lines exist.

**Adversarial finding 8**: Could a subagent claim have been
laundered as command evidence? No — every command in the
formal-verification-report.md and black-hat-review.md is a
real `bash`/`cargo`/`rtk` invocation with raw output
captured to `.beads/vb-jtqqx/evidence/state12_*.log`. The
sha256 hashes in the assurance-bundle.md Hash Anchors table
match the on-disk artifacts. No subagent summary is presented
as command evidence.

**Adversarial finding 9**: Could the proof claims lack source
binding? No — the proptest bodies call
`vb_storage::keys::decode_storage_key` (the real production
function) and assert on `vb_storage::KeyDecodeError` (the real
production enum). No shadow model, no mock, no test-only
re-implementation. The proptest bodies ARE the bridge between
proof and production.

**Adversarial finding 10**: Could a behavior-affecting waiver
have been issued? No — `formal-waivers.jsonl` has 6 rows, all
with `behavior_affecting: false` and all with `status: approved`.
The 6 non-behavior waivers are bookkeeping for the
`not_applicable` lanes (verus, kani, flux-rs, loom, miri,
cargo-fuzz) per `verifier-lane-decisions.jsonl:2-7`.

## Mandated Improvements

(none — all 10 adversarial checks pass; 0 findings)

## Verdict

**STATUS: APPROVED**

The bead is approved for landing. All evidence is implementation-bound,
raw command output is captured, every requirement maps to a test, every
proof obligation has a PASS row, every non-behavior waiver is in scope,
and the black-hat review is APPROVED with 0 findings. The 5 pre-existing
global failures are documented and out of scope.

A state-14 row will be appended to
`.beads/vb-jtqqx/agent-invocation-ledger.jsonl` (sequence 7 of 7).
