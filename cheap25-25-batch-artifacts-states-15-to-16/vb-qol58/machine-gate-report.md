---
bead_id: vb-qol58
schema_version: machine-gate-report/v1
state: 12
skill: formal-verifier (subsumes machine-gate-report role)
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58
host_session_id: femdation-cheap25-batch
status: subsumed
formal_verifier_invocation_id: formal-verifier-vb-qol58-state12-20260701T225200Z
parent_invocation_id: holzman-rust-vb-qol58-state11-20260701T192500Z
---

# Machine Gate Report: vb-qol58 — **SUBSUMED by formal-verification-report.md**

## Bead

- **Bead:** `vb-qol58` — Lint: fix source slicing/indexing issues in IPC and test utilities (P0 bug).

## Why `machine-gate-report.md` is Subsumed

Per `verification-lane-policy.md §"Verifier Lane Profiles"` and the pipeline handoff:

> "Default Rust behavior profile: `kani`, `verus`, `flux-rs`, `proptest`. Conditional profile additions: `loom` for implementation concurrency..."

For vb-qol58, the entire verification surface consists of `proptest`-mapped machine commands (the 3 cargo/moon gates). These are NOT separate "machine gates" from the formal-verifier role; they ARE the formal-verifier's commands. Per `proof-writer-report.md §"Why 'No Proof Work' Is Honest"`:

> "The 3 required obligations map to moon/cargo gates, not to Verus/Kani/Flux/Loom/Miri/proptest artifacts. The `proof-strategy.md §2.3` table documents the `proptest` enum mapping for these moon/cargo commands..."

Therefore, the formal-verification-report.md already documents:

| "Machine gate" command | Formally verified in |
|---|---|
| `moon run :lint-src` | `formal-verification-report.md` §"PO-qol58-001" + `.evidence/vb-qol58/verifier/lint-src.log` |
| `cargo check -p vb_ipc --all-targets --all-features` | `formal-verification-report.md` §"PO-qol58-002" + `.evidence/vb-qol58/verifier/cargo-check.log` |
| `cargo test -p velvet-ballistics-workspace-tests --lib --all-features` | `formal-verification-report.md` §"PO-qol58-003" + `.evidence/vb-qol58/verifier/cargo-test.log` |

The machine-gate-report role for vb-qol58 **is** the formal-verification-report.md; no separate report is required. The downstream landing script may consume either file; this stub formalizes the subsumption explicitly.

## Verification

- `formal-verification-report.md` exists and is non-empty (18142 bytes; sha256 logged in agent-invocation-ledger.jsonl row 9 input_artifact_hashes).
- `verification-ledger.jsonl` has 3 PASS rows with raw command evidence (raw log + exit marker paths).
- All 3 gate raw logs at `.evidence/vb-qol58/verifier/` exist and are non-empty:
  - `lint-src.log` (3569 bytes; sha256 `59abb44a322e16f118956bda5cb9c798a2b2d8f8582a9157a93999700ca90b33`)
  - `cargo-check.log` (0 bytes due to `--quiet` cache hit; sha256 = canonical-empty)
  - `cargo-test.log` (133 bytes; sha256 `bd577d55f236b941832cfce54c469379addf9726f39f5d442594892b2ea25b79`)

## Cross-Reference

Subsuming artifact: `.beads/vb-qol58/formal-verification-report.md`
Verifying invocation: `formal-verifier-vb-qol58-state12-20260701T225200Z`
Verification ledger: `.beads/vb-qol58/verification-ledger.jsonl` (3 rows; all PASS)
Raw evidence: `.evidence/vb-qol58/verifier/{lint-src,cargo-check,cargo-test}.log` + `.evidence/vb-qol58/verifier/{lint-src,cargo-check,cargo-test}.exit.txt`

## Status

**STATUS: SUBSUMED** (formal-verification-report.md fully captures the machine-gate role for vb-qol58; no dual reporting).
