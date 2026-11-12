# vb-kyyf State 13 Provenance Waiver

STATUS: APPROVED

## Scope

- Bead: `vb-kyyf` only.
- State: 13 owner-authorized substitute evidence packaging only.
- Attempt: `owner-authorized-substitute-1`.
- Isolated workspace: `/home/lewis/src/bd-vb-kyyf-bdd`.

## Owner Authorization

The user explicitly said no evidence-package agent is required and instructed femdation to use subagents and resolve the provenance differences. This direct substitute packaging subagent is authorized for the State 13 owner-authorized substitute evidence packaging lane.

## Missing-Agent Issue

The original State 13 blocker remains real and is not hidden: `.beads/vb-kyyf/blocker-report-state13-evidence-packaging-agent.md:22-26` records that no OpenCode `evidence-packaging` agent file was found, only an `evidence-packaging` skill existed, child agents were forbidden from invoking skills, and a prior fallback-to-default-agent pattern had been rejected for provenance.

## Explicit Substitute Provenance

This package is not represented as output from the missing `evidence-packaging` agent. It is an owner-authorized substitute package prepared by a direct child of femdation, using only the isolated workspace and existing artifacts. The provenance is explicit rather than laundered so later truth-serum or landing review can distinguish this substitute lane from a specialist-agent lane and audit the owner authorization directly.

## Inputs Checked

- Manifest owner authorization: `.beads/vb-kyyf/dispatch-state13-owner-waived-packaging.json:35-37`.
- Blocker provenance: `.beads/vb-kyyf/blocker-report-state13-evidence-packaging-agent.md:22-36`.
- State 11 approval and PO results: `.beads/vb-kyyf/formal-verification-report.md:3`, `.beads/vb-kyyf/formal-verification-report.md:26-39`.
- State 12 approval: `.beads/vb-kyyf/black-hat-review.md:3`, `.beads/vb-kyyf/black-hat-review.md:43-45`.
- JSONL validation was run in the isolated workspace with `jq -c .` on `.beads/vb-kyyf/traceability-matrix.jsonl` and `.beads/vb-kyyf/verification-ledger.jsonl`; both commands exited 0.
