# Plan — vb-god2f.4: vb-god2f gated fuzz obligations after non-fuzz closure

| Field | Value |
|---|---|
| Bead | `vb-god2f.4` (P0 IN_PROGRESS, parent `vb-god2f`) |
| Planner invocation | `proof-planner/vb-god2f.4@2026-07-01` |
| State | 4 (planned) — proof-planner output, awaiting proof-plan-reviewer |
| Companion review | `.beads/vb-god2f/dispatch/black-hat.md` |
| Verifier lane | **cargo-fuzz** (bounded dynamic evidence only — NOT formal proof) |
| Production-binding | **N/A — cargo-fuzz is not a Verus lane; production-binding is a Verus-only mechanism.** Fuzz harnesses must still bind to production source via direct invocation of the production entry point (no model/stub). |
| Re-derivation note | Parent `vb-god2f` claimed this file existed at this path on 2026-06-30. It did not. This is the re-derived version produced via the proof-planner → proof-plan-reviewer cycle per `vb-240tk`. |

## 1. Goal

Execute three gated fuzz obligations — `HVR-PO-STORAGE-004`,
`HVR-PO-STORAGE-007`, and `HVR-PO-BI-003` — after the non-fuzz Kani /
Verus gates have closed and been reviewed. Per `bd show vb-god2f.4`
these obligations were **deferred from the formal-verifier rerun on
20260619T115530Z** because the non-fuzz gates were still dirty.

The plan must preserve the **acceptance criterion from `bd show vb-god2f.4`**
verbatim: *"after non-fuzz blockers close/review, run exact approved
cargo-fuzz commands from `.beads/vb-god2f/hard-verus-proof-obligations.planned.jsonl`
with raw logs, proof-reviewer evidence approval, and formal-verifier
ledger rows. **Treat fuzz as bounded dynamic evidence only, not formal
proof.**"*

## 2. Obligations in scope

| Obligation ID | Crate | Surface | Fuzz target binding |
|---|---|---|---|
| `HVR-PO-STORAGE-004` | `vb_storage` | Journal / envelope decoder | `fuzz/fuzz_targets/journal_event.rs` (or named equivalent) |
| `HVR-PO-STORAGE-007` | `vb_storage` | Recovery / classification round-trip | `fuzz/fuzz_targets/recovery_classify.rs` (or named equivalent) |
| `HVR-PO-BI-003` | `vb_boundary_inventory` | Boundary validation under hostile input | `fuzz/fuzz_targets/boundary_inventory.rs` (or named equivalent) |

The exact `cargo fuzz` commands are taken **verbatim** from
`.beads/vb-god2f/hard-verus-proof-obligations.planned.jsonl`. The
proof-writer MUST NOT rewrite the command. If a command needs
modification, that is a new obligation row, not a parameter edit
to an existing one.

## 3. Pre-execution gate (BLOCKING)

Before any fuzz run is recorded as PASS, the proof-writer MUST
verify **all three** of the following:

1. **Non-fuzz gates closed**: `HVR-PO-BI-001` and `HVR-PO-CORE-004`
   (`vb-god2f.2`) and `HVR-PO-STORAGE-001` (`vb-god2f.3`) have all
   closed (PASS or accepted non-closure) with raw evidence in
   `.beads/vb-god2f*/evidence/`.
2. **Three fuzz target source files exist**: `ls fuzz/fuzz_targets/`
   MUST list a `.rs` file for each of the three obligations. The
   file count and basenames are recorded in the acceptance
   evidence (`plans-re-derived.txt` cites them).
3. **Fuzz harness build**: `cargo fuzz build` exits 0 for every
   target that will be run. The build log is captured as
   `.evidence/vb-god2f.4/fuzz-build-<ts>.log`.

If any of (1)/(2)/(3) fails, fuzz execution is **deferred**, not
run. This is per the parent black-hat handoff note (2):

> *"vb-god2f.4 proof-writer MUST verify three fuzz target binaries
> exist under fuzz/fuzz_targets/ before execution."*

This plan executes that pre-check and records the result in §8.

## 4. Verifier lane decision

| Lane | Required? | Rationale |
|---|---|---|
| Verus | **not_applicable** | These obligations are fuzz; the property is *behavioural pressure on hostile input*, not pure/core invariant territory. No Verus spec is being created. |
| Kani | **not_applicable** | Bounded model check would overlap but is a different obligation. Out of scope here. |
| Flux | not_applicable | Refinements are not the closure lane. |
| proptest | not_applicable | Property-pressure is fuzz here, not proptest. |
| **cargo-fuzz** | **required** | Default Rust behavior lane for parsers, codecs, hostile input, persisted bytes, IPC/storage decoding, fuzzable canonicalization boundaries per `verification-lane-policy.md`. All three obligations fit those buckets. |

## 5. Fuzz as bounded dynamic evidence (NOT formal proof)

Per `bd show vb-god2f.4` acceptance: *"Treat fuzz as bounded dynamic
evidence only, not formal proof."* This plan commits to that
classification explicitly.

What fuzz PASS does and does NOT establish:

| Does establish | Does NOT establish |
|---|---|
| That no panic, abort, or UB-triggering input was found in the bounded corpus during the run window. | That *no* such input exists in the input space (no completeness). |
| That no `unwrap`/`expect`/`panic!`/`unreachable!` path was hit on a randomly mutated input. | That the property holds under symbolic execution (that's Verus / Kani). |
| That coverage of the targeted module grew along expected edges. | That coverage is exhaustive. |
| That the executable was free of crashes for `N` CPU-hours with seed corpus `C`. | That the executable is correct, only that no observed inputs crashed it. |

Therefore the formal-verifier ledger row for each obligation MUST
include the field `evidence_class: bounded-dynamic-evidence`
(distinct from `formal-proof` for Verus / Kani rows).

## 6. Production-binding strategy (for fuzz harnesses)

Cargo-fuzz obligations do not carry `production_binding` (a Verus-only
field). However, fuzz harnesses MUST satisfy the spirit of
production-binding by:

1. Invoking the **production** entry point (e.g.
   `vb_storage::journal::decode(...)`) — not a hand-rolled decoder
   that lives in the fuzz harness.
2. Using `Arbitrary` / `ArbitraryFromArbitrary` derivations on
   small wrapper types whose field layout matches the production
   byte envelope (cite the production source path in a header
   comment).
3. Naming the production source path the harness exercises in a
   `// FUZZ PRODUCTION BINDING:` header that the proof-reviewer can
   grep for.

A fuzz harness that decodes a hand-rolled struct not present in
production source is a **vacuum fuzz** (no production binding). It
MUST be rejected.

## 7. GOD-RULE compliance markers

| Rule | Plan posture |
|---|---|
| **GOD-RULE 1** — No hardcoded Kani shapes | N/A — Kani is not the lane here. The fuzz analogue is: no hardcoded seed inputs that mask the property. Seed corpus must be derived from a coverage-guided dictionary or from a real captured-envelope sample, not from a hand-rolled fixed buffer. |
| **GOD-RULE 2** — No vacuum Verus proofs | N/A — no Verus proof created. Fuzz harnesses bound to production entry points per §6. |
| **GOD-RULE 3** — No unbounded TLA+ math | N/A — no TLA+. |
| **GOD-RULE 4** — No loop oscillations | If a fuzz crash surfaces a real bug in production, the proof-writer MUST patch production. The fuzz harness MUST NOT be silently weakened (smaller `max_len`, fewer mutations) to make the run green. |
| **GOD-RULE 5** — No blind verification mutations | Stay inside `fuzz/fuzz_targets/` + the targeted production module's blast radius. No fleet-wide `cargo fuzz`. No fuzzing of unrelated crates in the same workspace. |

## 8. Acceptance criteria for `vb-god2f.4`

1. **Pre-check evidence** (cited in `plans-re-derived.txt` or
   `.beads/vb-240tk/evidence/`):
   a. `ls fuzz/fuzz_targets/` lists exactly the three fuzz targets
      named in §2 (or their renamed-but-1:1-mapped equivalents,
      recorded with old→new mapping).
   b. `cargo fuzz build` exits 0 for each target, with raw build
      log at `.evidence/vb-god2f.4/fuzz-build-<ts>.log`.
   c. Non-fuzz gates (`vb-god2f.2`, `vb-god2f.3`) are closed or
      accepted-non-closure (cited by bead ID + status + raw evidence
      path).
2. **Run evidence**: For each obligation,
   `.evidence/vb-god2f.4/fuzz-runs/<PO>.log` contains the **verbatim
   `cargo fuzz` invocation from
   `.beads/vb-god2f/hard-verus-proof-obligations.planned.jsonl`**,
   with exit code, run duration, corpus size, and (if a crash was
   found) the crashing input path.
3. **Ledger rows**: formal-verifier records three rows with
   `evidence_class: bounded-dynamic-evidence`, **not** `formal-proof`.
4. **No weakening**: no fuzz run was made green by lowering
   `max_len`, shrinking the seed corpus, or disabling mutators.
   proof-reviewer must spot-check the run commands against
   `hard-verus-proof-obligations.planned.jsonl` for identity.
5. **No hardcoded seeds**: the seed corpus is documented
   (dictionary or captured-envelope source). proof-reviewer
   spot-checks the corpus directory.
6. `proof-plan-reviewer` record at
   `.beads/vb-god2f/dispatch/black-hat.md` shows
   `STATUS: APPROVED` for this plan (no `blocker` findings).

## 9. Cross-references

- Parent black-hat handoff note (2): parent `vb-god2f` NOTES.
- Approved commands source: `.beads/vb-god2f/hard-verus-proof-obligations.planned.jsonl`.
- Predecessor rerun failure: `.evidence/vb-god2f/formal-runs/20260619T115530Z/logs/`.
- Sibling blockers: `vb-god2f.2` (HVR-PO-BI-001 / HVR-PO-CORE-004
  non-fuzz) and `vb-god2f.3` (HVR-PO-STORAGE-001 non-fuzz) MUST
  close first.

## 10. Out of scope (per `vb-240tk`)

- No fuzz harness source code is written in this plan file.
- No `vb-god2f.4` bead-record edit (status stays `IN_PROGRESS`).
- No `cargo fuzz run` is invoked by this planner (the proof-writer
  invokes them downstream).
- No new fuzz targets are added by this plan. The three target
  source files are **pre-existing** per the black-hat handoff; this
  plan only commits to *verifying their existence* before execution.