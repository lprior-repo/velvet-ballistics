# Test Re-Review — vb-7x0ol (Independent Recovery + ABI Guard Review)

**Bead**: vb-7x0ol
**Reviewer**: DeepSeek v4 Pro via OMP (independent; authored none of the tests under review)
**Scope**: suite re-review at frozen source `162ac607dc86657f060882a81dc8957e955e726c`.
**Model metadata**: `deepseek/deepseek-v4-pro`, thinking level high. No fallback; no local-5090/3090 label.

## Gate Result

**STATUS: APPROVED (scoped).** F3's C6 harness now runs green; the typed ABI guard is
exercised by the passing Fjall suite; and I added independent public-API probes that close
the prior T-4 "registered execution has no public-API coverage" gap. Two Fjall tests remain
red for known, tracked reasons (§Remaining Red), and the durable-retry CLI path (prior F-2 /
T-4) remains unreviewed because it is owned by a different active author.

---

## Actual test runs (real exit codes, no pipe masking, `python3 /tmp/vb-run-gate.py`)

| Test target | Result | Evidence file (stdout/stderr/json) |
|-------------|--------|------------------------------------|
| `cargo test -p velvet-ballistics-workspace-tests --test runtime_fjall_pending_action_recovery` | **18 passed, 2 failed** (exit 101) | `fjall_pending_action.*` |
| `cargo test -p velvet-ballistics-workspace-tests --test vb_7r7jv_executor_conformance` | **5 passed** (exit 0) | `executor_conformance.*` |
| `cargo test -p velvet-ballistics --test vb_md3w_retained_input_recovery` | **1 passed** (exit 0) | `c6_retained_input.*` |
| `cargo test -p velvet-ballistics-workspace-tests --test vb_7x0ol_probe_registered_executor` (temp probe) | **5 passed** (exit 0) | `probe_guard.*` |
| `cargo test -p velvet-ballistics --test vb_7x0ol_probe_c6_continue` (temp probe) | **1 passed** (exit 0) | `c6_continue_probe.*` |

The two red Fjall tests are exactly the two known worker-owned gaps; the other 18 pass,
matching the reported "18/20" state (the ABI-guard fix moved the divergent-ABI submit case
from red to green).

---

## F3 — C6 retained-input test (prior T-1 blocker)

**Prior**: `serde_json::from_str` on `--emit yaml` output → parse error before any assertion.

**Now** (`vb_md3w_retained_input_recovery.rs`): `serde_saphyr::from_str` (line 50) parses the
YAML receipt; `strict_config` → `journaled_config` (policy `Strict` → `Journaled`) so the
recovery runtime's policy matches the CLI `--durability journaled` admission. Verified green
for inputs `[i64::MIN, -7, 0, 41, i64::MAX]`.

**T-1 verdict: FIXED.**

---

## Prior T-4 (no public-API registered-execution coverage) — closed by independent probes

The prior review found `execute_registered_action`/`reconcile_registered_action` had zero
in-repo callers and no test proving "recovery reconciles receipts and never re-runs a live
effect". I wrote and ran (then removed from `crates/`, kept in evidence) 5 public-API probes
against a real `Runtime` + `ExecutorCatalog` + `FjallRegister` wired via
`StorageRuntimeJournal::shared_strict_with_catalog`:

1. `execute_registered_action_commits_one_effect_and_completes_run` — Live pending action
   executes exactly one effect (`effect_count==1`, target value `42`), the run completes, and
   the pending action is consumed.
2. `reconcile_registered_action_reads_receipt_without_new_effect` — reconcile returns the
   same receipt with `effect_count` still `1`; a fresh empty register returns
   `ReceiptMissing` (fail closed).
3. `execute_registered_action_rejects_mismatched_grant_artifact` — a grant issued for a
   different artifact → `Executor(Unauthorized)`, `effect_count==0`.
4. `recovery_of_pending_registered_action_blocked_pack_rejects` — `recover_and_resume` on a
   `RecoveryClass::Blocked` pending action → `RecoveryCannotResume { reason: "registered_recovery_blocked" }`.
5. `recovery_of_pending_registered_action_reconcile_pack_is_resumable` — `recover_product` on
   a `ReconcileReceipt` pending action → `Resumable` with `boundary_kind == PendingAction`.

Probe sources preserved at `evidence-rereview/probe_registered_executor.rs` and
`probe_c6_continue.rs`. These are labeled seeded probes (submit via public runtime API, not a
public CLI end-to-end) — they validate authentic capability/contracts/storage fixtures from
existing public APIs, not fabricated journal prefixes.

**T-4 (registered-execution half): closed by probe, not by permanent test.** No permanent
in-repo test exercises this surface yet; recommendation to the final integration author: adopt
probes 1-5 as a permanent `workspace_tests` integration test once the pending-action
rehydration (gap #3) is fixed, so the ReconcileOnly → `execute_registered_action` →
`Unauthorized` mode guard is also directly observable.

---

## C6 exact-output gap (beyond cosmetics)

The C6 test asserts only `RuntimeRecoveryProduct::Resumable { run_id }` plus re-readability of
retained inputs; it never drives the recovered run to a finish. My probe
`recovered_run_continues_to_exact_finish_result_slot` closes this: after
`recover_and_resume` + `tick_all`, the journal contains
`RunFinished { result: SlotIdx::ZERO }` and the retained input slot 0 equals
`SlotValue::I64(value)`. Since the workflow is `finish: { result: $input.count }`, this proves
the recovered run continues to the exact result slot referencing the retained input — the
behavior is **correct**, and the only gap is that the permanent C6 test under-asserts it.

Recommended (non-blocking): extend `vb_md3w_retained_input_recovery.rs` to also drive
`recover_and_resume` + `tick_all` and assert the `RunFinished { result }` slot equals the
input slot.

Retained-input precedence (inputs seeded before journal overlays; later journal writes win) is
implemented in `recovery/frame.rs:36-45` (`hydrate_shape_checked_run_frame_with_inputs`); no
precedence defect observed.

---

## Remaining Red (known, tracked; not new)

- `runtime_submit_drive_persists_recoverable_fjall_pending_action_abi_digest` — pending ticket
  not rehydrated after cold reopen. Owned by vb-s1rry merge.
- `runtime_submit_drive_without_action_abi_authority_fails_before_ticket_persist` — Gate12
  strict submit rejects `ACTION_CONTRACT_MISSING` before the test's old expectation; final
  integration author updates the expectation (no guard weakening).

## Out of scope / unreviewed

- `vb-r5c1c` durable-retry CLI path (separate active author).
- `digest_duplicate_parity` obsolete mirrors (4 red, source-identity F4/F5/F6 already approved).

## Verdict

**APPROVED (scoped)** for the F3 harness fix and the typed ABI-guard test surface; the
registered-execution recovery invariants are now backed by independent public-API probes. Full
integration approval remains blocked on the two known worker-owned red tests and the durable
retry path (owned elsewhere).
