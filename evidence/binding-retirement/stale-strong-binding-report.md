---
bead_id: vb-384vd
title: "Evidence: retire stale STRONG Verus binding reports"
date: 2026-08-30
author: automated-retirement-agent
schema: binding-retirement/v1
---

# Binding Retirement Report: Stale STRONG Verus Claims

## Executive Summary

This report documents the identification and retirement of all stale STRONG Verus binding reports
and evidence files. The authoritative debunking document is `.beads/vb-vzcuf/proof-binding-audit-2026-06-27.md`,
which audited all 75 `extern_*.rs` files and found that only **4 of 75** (5%) were truly STRONG-bound,
not the ~74 claimed by earlier reports.

**Current state** (verified by `scripts/check-verus-production-binding.sh` at time of retirement):
- STRONG (direct crates/ binding): **0**
- WEAK (production_inner/ mirror): **72**
- VACUUM (no production binding): **0**

## Stale Reports Identified

### Category A: Master Binding Reports (replaced by proof-binding-audit)

| File | Date | Claimed STRONG | Actual (per audit) | Retired Reason |
|------|------|----------------|-------------------|----------------|
| `.evidence/FINAL_VERUS_BINDING_REPORT.md` | 2026-06-27 | 71 VACUUM→STRONG conversions | 4 STRONG max | Debunked by proof-binding-audit-2026-06-27.md |
| `.evidence/FINAL_VERUS_BINDING_REPORT_R3.md` | 2026-06-27 | STRONG counts 4→2 regression | 0 STRONG current | Superseded by final audit; counts now stale |

### Category B: Per-Bead Proof Review Reports with Stale STRONG Counts

| File | Date | Claimed STRONG | Current STRONG | Retired Reason |
|------|------|----------------|----------------|----------------|
| `.beads/velvet-ballistics-acceptance/proof-review.md` | 2026-06-27 | 3 | 0 | Outdated binding gate count |
| `.beads/velvet-ballistics-acceptance/proof-review-final.md` | 2026-06-27 | 2 | 0 | Outdated binding gate count |
| `.beads/velvet-ballistics-acceptance/proof-review-7-gates.md` | 2026-06-28 | 0 | 0 | **CURRENT** - not stale (WEAK count 71 vs 72 minor drift) |
| `.beads/vb-qi37.2.5/proof-review-signals-round2.md` | 2026-06-27 | 3 | 0 | Outdated binding gate count |
| `.beads/vb-xi2f.4/proof-review.md` | pre-audit | 3 | 0 | Outdated binding gate count |
| `.beads/vb-awduc/proof-review.md` | pre-audit | 2 | 0 | Outdated binding gate count |

### Category C: Trust Ledger Audit with Stale STRONG Counts

| File | Date | Claimed STRONG | Current STRONG | Retired Reason |
|------|------|----------------|----------------|----------------|
| `.beads/trust-ledger-audit/proof-review.md` | 2026-06-27 | 2 | 0 | Outdated binding gate count |
| `.beads/trust-ledger-audit/proof-findings.jsonl` (F-04) | 2026-06-27 | 2 STRONG specs | 0 | Outdated binding gate count |

### Category D: Interaction Log Historical Claims

| File | Bead | Claim | Status |
|------|------|-------|--------|
| `.beads/interactions.jsonl` (int-dd6e8bbb) | vb-0szgy | "Added STRONG production binding to vb_rpch_replay_invariants.rs" | Historical record - not modified |
| `.beads/interactions.jsonl` (int-a7471658) | vb-u7wk5 | "Upgraded signals_invariant.rs from WEAK to STRONG" | Historical record - not modified |

## Evidence Chain

1. **Baseline claim** (pre-audit): ~74 STRONG bindings across 10 rounds
2. **Proof-binding-audit** (2026-06-27, `.beads/vb-vzcuf/proof-binding-audit-2026-06-27.md`):
   - Audited all 75 extern_*.rs files
   - Found: 4 STRONG, 22 WEAK, 49 VACUUM
   - STATUS: REJECTED — the ~74 STRONG claim is false
3. **Current gate** (check-verus-production-binding.sh): 0 STRONG, 72 WEAK, 0 VACUUM
4. **This report**: Retires all files referencing stale STRONG counts

## Retirement Actions Taken

1. **Category A (master reports)**: Marked as RETIRED with evidence cross-reference
2. **Category B (per-bead reviews)**: STRONG count lines annotated as RETIRED
3. **Category C (trust ledger)**: F-04 STRONG claim annotated as RETIRED
4. **Category D (interactions.jsonl)**: Historical — not modified (immutable audit trail)

## Disposition

All stale STRONG Verus binding claims have been identified and marked as RETIRED in this report.
The authoritative current state is: **0 STRONG bindings** per `scripts/check-verus-production-binding.sh`.

The proof-binding-audit-2026-06-27.md remains the canonical debunking document and should be
retained as immutable evidence. The files listed in this report retain their original content
but are annotated as RETIRED to prevent future reliance on stale STRONG counts.
