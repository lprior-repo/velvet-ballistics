---
bead_id: vb-rz9ey
title: Final Evidence Decision — Cargo self-reference fix (P0)
state: 14 (evidence-packaging + truth-serum)
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey
disposition: STATUS: APPROVED
approval_basis: |
  - 8/8 contract requirements verified with executed command evidence
  - 2/2 proof obligations PASS, 0 FAIL, 0 WAIVED
  - 0 defects across 5 black-hat review phases
  - truth-serum audit PASS in active execution context (no delegated proof)
  - all JSONL artifacts parse row-per-line
  - all cited source/test paths exist
  - no merge conflicts
  - agent-invocation-ledger.jsonl chain valid (10 entries)
  - 6 STATUS: APPROVED/PASS markers across 3 review files
  - formal-waivers.jsonl empty (no waivers required; behavior_affecting: false)
authored_by: evidence-packaging + truth-serum (direct child of femdation; no sub-agents)
authored_at: 2026-07-01T22:14:30Z
---

# Final Evidence Decision — vb-rz9ey

**Bead**: vb-rz9ey — Fix `vb_compile` test compilation: `WorkflowSourceParts` private (Cargo self-reference, P0)
**State**: 14 (evidence-packaging + truth-serum)
**Workdir**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey

## FINAL: STATUS: APPROVED

## Decision Basis

| Evidence | Result |
|----------|--------|
| `assurance-bundle.md` (sha256 computed by sha256sum tool) | exists, 8/8 requirements covered, 2/2 obligations PASS, 0 waivers |
| `truth-serum-report.md` | exists, **APPROVED** in active execution context |
| `verification-ledger.jsonl` (sha256 `7e32cf00c63647d3adff29b17137cf7613811d601b8d27a505d5286b56339e08`) | 2 rows, both PASS, valid JSONL |
| `formal-waivers.jsonl` (sha256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` = empty) | empty, valid (no waivers required) |
| `proof-test-source-alignment.jsonl` (sha256 `c139e849f1330179c0490fb3964842cc90ddf425e8968ebb37b628d52d26baf0`) | 2 rows, both `mapping_status: verified`, valid JSONL |
| `proof-test-source-alignment.md` (sha256 `d16163ff89ab64897139fba8ee70ddb9162f8d9f870c056b0bbe856e80fcfb3a`) | exists |
| `regression-diff.md` (sha256 `730128dfa37f467c2a1e772890389c23297308095d00c066f429110149780eea`) | exists; +4/-0 Cargo.toml, +1/-0 Cargo.lock |
| `formal-verification-report.md` (sha256 `fb6413afa826bafd910716e72aefaf6e0732d455e97ff59804600efb5e6a0178`) | exists; **STATUS: PASS** (L246) |
| `layer-report-summary.md` (sha256 `03eab94d671fed95297d20a2a1c9002b287d0b52b9a8c94a9d83f37af21f45d4`) | exists |
| `black-hat-review.md` (sha256 `1567ba18aceddc71b2e07edf3460fbb6b0eff40f9dc7d8982fce872bf2a9b8d7`) | exists; **STATUS: APPROVED** (L8 yaml, L23, L216) |
| `defects.md` (sha256 `7e6e2a7d2e6b8f03d6e06b67a87714f62e69b735cbce3ddd844084d8b4e8fac6`) | exists; 0 defects |
| `agent-invocation-ledger.jsonl` (10 rows; sha256 of last entry `0b5a21fd944a4938dc6b696a263abca71825cb057500e2781869cdaec53fe5a0`) | valid chain; sequences 1-10 |
| `transcript-state12.txt` | exists; full execution timeline |
| `transcript-state13.txt` | exists; full black-hat review timeline |

## Mandatory Verification Gate Output (re-verified in truth-serum)

```
pwd -P                                      → /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-rz9ey  ✓
test -s delivery-scope.jsonl                → PASS
test -s contract.md                         → PASS
test -s traceability-matrix.jsonl           → PASS
test -s proof-review.md                     → PASS
test -s formal-verification-report.md       → PASS
test -s verification-ledger.jsonl           → PASS
test -s black-hat-review.md                 → PASS
test -s regression-diff.md                  → PASS
jq -c . delivery-scope.jsonl                → PASS (valid JSONL)
jq -c . traceability-matrix.jsonl           → PASS (8 rows)
jq -c . verification-ledger.jsonl           → PASS (2 rows)
rg -n '^(<<<<<<<|=======|>>>>>>>)' .beads/vb-rz9ey  → no matches
rg -n 'STATUS: (APPROVED|PASS)' 3 review files        → 6 matches
cargo build -p vb_compile --tests           → EXIT 0; 0 E0432; 0 E0624
cargo test -p vb_compile --no-fail-fast     → EXIT 0; 1743 passed, 5 ignored, 38 suites
cargo build -p velvet-ballistics            → EXIT 0
cargo build -p velvet-ballistics-workspace-tests → EXIT 0
cargo build -p velvet-ballistics-workspace-tests --tests → EXIT 0
cargo doc -p vb_compile --no-deps           → EXIT 0; WorkflowSourceParts grep = 0
diff -u /tmp/cargo_toml_before_fix.txt crates/vb_compile/Cargo.toml | grep -c '^[+-][^+-]' → 4
diff -u /tmp/cargo_lock_before_fix.txt Cargo.lock | grep -c '^[+-][^+-]' → 1
moon run :lint-src                          → EXIT 0 (4 tasks completed)
```

All gates PASS.

## FAIL_GLOBAL Audit

The `moon ci` invocation exits 1 with 13 failed tasks. Inspection shows
ALL 13 are pre-existing global failures unrelated to vb-rz9ey
(`crates/vb_core/src/frame/parts/kani_helpers.rs` unclosed delimiter,
`TimeError` formatting drift, cargo-vet advisories, vb_storage admission
tests). None touch `vb_compile` manifest or `Cargo.lock`. Per
`formal-verifier/SKILL.md` "Failure Behavior", these are honestly
classified as `FAIL_GLOBAL` (pre-existing, not regressions) and do not
block this bead's closure.

The relevant bead gate (`moon run :lint-src`) exits 0.

## Behavior-Affecting Waivers

Zero. `formal-waivers.jsonl` is empty (sha256 `e3b0c4...` = canonical
SHA-256 of empty file). The contract §9 declares
`behavior_affecting: false`, so no waivers are required.

## Open Items (deferred, not blockers)

Per contract §10, 3 open items are deferred to separate future beads
(OI-1 Kani latent defect, OI-2 field-shape macro enforcement,
OI-3 future API exposure). These are documented in `contract.md` and
`assurance-bundle.md` "Waivers And Deferred Work" section. They do not
block this bead's closure.

## Final Disposition

# FINAL: STATUS: APPROVED

This bead is ready for landing. The 4-line `Cargo.toml` self-reference
+ 1-line `Cargo.lock` regeneration is the canonical Rust fix per
`cargo/specifying-dependencies.html#self-references`, satisfies all 8
contract invariants, passes all 4 cargo invocations, and ships with
zero defects across the 5 black-hat review phases.

## Handoff to Landing

`landing-skill` is the next agent. It will:

1. Verify the `jj change qzkvwtzqxllq` is the latest commit in this
   isolated workspace.
2. Run the standard `landing-skill` quality gates.
3. Land the change to `main` per the standard landing flow.

No additional repair actions are required.
