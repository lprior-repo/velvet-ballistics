# Proof Plan Review Input — vb-f7k6 — Attempt 3 Repair

## Review Request

Review the repaired State 4 proof plan after contract-verification rejection. This attempt fixes schema defects and explicitly routes the production/proof authority mismatch without claiming current RunId-only production is proven.

## Artifacts Under Review

- `.beads/vb-f7k6/proof-obligations.jsonl`
- `.beads/vb-f7k6/proof-obligations.planned.jsonl`
- `.beads/vb-f7k6/proof-strategy.md`
- `.beads/vb-f7k6/STATE.md`

## Repaired Findings

- `TLA-TW-001`..`TLA-TW-006`: now include `state_constraints`.
- `VERUS-TW-001` / `VERUS-TW-002`: waiver rows now use `status:"planned"`; waiver semantics are in `mode` and `waiver`.
- `PO-009`: planned Verus waiver row also uses `status:"planned"`.
- `TEST-TW-001` / `PO-008`: runtime evidence path remains `.beads/vb-f7k6/test-report.md`; State 5 is required to write it from the already-run `/usr/bin/env cargo test -p vb_runtime timer` evidence.
- `AUTH-TW-001` / `PO-011`: added required State 10 production authority-binding plan.

## Authority Binding Decision

Option A chosen. Current production authority is `Runtime::timer_fired(run)` / `ShardCommand::TimerFired { run }`, while TLA/Loom authority is freshness-bearing. The plan therefore marks TLA/Loom stale-fire evidence as target-design pre-implementation until State 10 changes production to carry or derive and validate timer freshness metadata/token equivalent to `(run, generation, deadline, kind)`.

No waiver is claimed for RunId-only delivery.

## Reviewer Checklist

1. JSONL parses line-by-line.
2. Every canonical TLA row has `state_constraints`.
3. Verus waiver rows have `status:"planned"`, not `status:"waived"`.
4. Durable runtime parity evidence is planned at `.beads/vb-f7k6/test-report.md` and assigned to State 5.
5. Production/proof authority mismatch is routed to State 10 with required evidence.
6. No proof success is claimed by this State 4 repair.
