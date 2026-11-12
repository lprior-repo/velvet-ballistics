# Proof-to-Rust Bridge Review — vb-jpq7 Wave 1

reviewer_skill: proof-reviewer  
reviewer_invocation_id: proof-reviewer-vb-jpq7-wave1-bridge-provenance-rereview-2026-05-23-gpt55  
prior_review_invocation_id: proof-reviewer-vb-jpq7-wave1-bridge-review-2026-05-23-gpt55  
workdir: `/home/lewis/src/vb-jpq7-wave1-proof`  
artifact_dir: `.beads/vb-jpq7-proof-wave1/`  
review_date: 2026-05-23

## Decision

APPROVED. The repaired proof-to-implementation bridge closes the prior source-reference and provenance blockers. The bridge maps accepted Wave 1 proof claims to concrete Rust source refs, executable evidence refs, and exact rerun commands; Kani remains an explicit blocked/non-claim row; stale raw `rg` static-scan and old TLC-wrapper logs are not used as PASS evidence by the bridge. The previously stale bridge provenance hashes have been rechecked against the current files in this review.

## Prior finding closure

### finding/v1 BRIDGE-SOURCE-REFS-001 — CLOSED

- Artifact: `.beads/vb-jpq7-proof-wave1/rust-refinement-obligations.jsonl:3`
- Mirror artifact: `.beads/vb-jpq7-proof-wave1/proof-to-rust-map.md:19`
- Obligation IDs: `RRO-WAVE1-TAINT-001`, `OBL-TLA-TAINT-001`, `OBL-PROP-TAINT-001`
- Raw review evidence: local parser/line-range check over all `RRO-WAVE1-TAINT-001` `source_refs`; `.beads/vb-jpq7-proof-wave1/rust-refinement-obligations.jsonl:3`; `.beads/vb-jpq7-proof-wave1/proof-to-rust-map.md:19`
- Closure: `RRO-WAVE1-TAINT-001` now has 21 exact `path:line-range::symbol` source refs for `Taint`, `join_taint`, taint-bearing `RunFrame` state/methods, recovery seed/slot types, hydrate paths, replay summary taint paths, and the public property. No placeholder strings such as `per traceability`, `TODO`, `TBD`, or directory-only recovery placeholders remain; every referenced file exists and every line range is in bounds.

### finding/v1 BRIDGE-PROVENANCE-001 — CLOSED

- Artifact: `.beads/vb-jpq7-proof-wave1/agent-invocation-ledger.jsonl:13`
- Obligation IDs: bridge provenance for all `RRO-WAVE1-*`
- Raw review evidence: local SHA-256 recomputation for `.beads/vb-jpq7-proof-wave1/proof-to-rust-map.md` and `.beads/vb-jpq7-proof-wave1/rust-refinement-obligations.jsonl`; `.beads/vb-jpq7-proof-wave1/agent-invocation-ledger.jsonl:13`; this review's provenance append records the current hashes.
- Closure: the bridge repair ledger entry at `.beads/vb-jpq7-proof-wave1/agent-invocation-ledger.jsonl:13` is superseded for hash freshness by this proof-reviewer re-review. Current bridge file hashes are:
  - `proof-to-rust-map.md`: `0cbac5b8db34b3fca996181f1b1f398554ab1330d567443b73ea8e9e1ee6d367`
  - `rust-refinement-obligations.jsonl`: `71be3b10222eb5e1bee4b3be376d68b572aa2cc4aee58a7624870a373f54142c`

### finding/v1 OBL-CURRENT-SOURCE-RERUN-WAVE1-001 — CLOSED

- Artifact: `.beads/vb-jpq7-proof-wave1/verification-ledger.jsonl:39-41`
- Obligation ID: `OBL-CURRENT-SOURCE-RERUN-WAVE1-001`
- Raw review evidence: `.beads/vb-jpq7-proof-wave1/evidence/current-source-rerun-wave1-freshness.log:6`, `.beads/vb-jpq7-proof-wave1/evidence/current-source-rerun-wave1-freshness.log:1025-1028`, `.beads/vb-jpq7-proof-wave1/evidence/current-source-file-hashes.log:7-62`, `.beads/vb-jpq7-proof-wave1/evidence/current-source-lightweight-required-checks-20260523T1644Z.log:44-66`.
- Closure: the PASS row for `OBL-CURRENT-SOURCE-RERUN-WAVE1-001` is backed by executable `current-source-rerun-wave1-freshness.log` running `moon ci` with `exit_status=0`, plus a later lightweight executable confirmation row with exit 0. `INFO-CURRENT-SOURCE-FILE-HASHES-WAVE1-001` is explicitly `NON_EXECUTABLE_INFORMATIONAL` with `exit_status:null` and is not used as the PASS closure.

## Accepted checks

- Kani is not laundered into a PASS: `.beads/vb-jpq7-proof-wave1/proof-to-rust-map.md:11,28` and `.beads/vb-jpq7-proof-wave1/rust-refinement-obligations.jsonl:11` state blocked/non-claim; `.beads/vb-jpq7-proof-wave1/verification-ledger.jsonl:21-28` records `FAIL_GLOBAL` with no exit-0 Kani evidence.
- Static clippy/cargo evidence maps to exact commands and source scope: `.beads/vb-jpq7-proof-wave1/proof-to-rust-map.md:26-27`, `.beads/vb-jpq7-proof-wave1/evidence/OBL-STATIC-NO-UNSAFE-001.log`, `.beads/vb-jpq7-proof-wave1/evidence/OBL-CARGO-TEST-WAVE1-001.log`.
- Current TLA bridge evidence references direct Java/TLC `OBL-TLA-*.log` files and commands, not superseded `tlc-Wave1*` wrapper logs.
- Superseded raw `rg` static logs are not cited as PASS evidence for the static row; the bridge cites `OBL-STATIC-NO-UNSAFE-001.log` and the ledger notes prior raw `rg` evidence is superseded.
- Current-source freshness separates executable PASS evidence from informational hash evidence: executable rows cite `current-source-rerun-wave1-freshness.log` and `current-source-lightweight-required-checks-20260523T1644Z.log`; the hash-only row is informational/non-executable.

STATUS: APPROVED
