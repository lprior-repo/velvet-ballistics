# State 13 Provenance Waiver — vb-vt2f

STATUS: APPROVED

## Scope

- Bead: `vb-vt2f` only.
- State: 13 owner-authorized substitute evidence packaging only.
- Attempt: `owner-authorized-substitute-1`.
- Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`.

## Owner Authorization

The owner authorization recorded in `.beads/vb-vt2f/dispatch-state13-owner-waived-packaging.json:29-31` is:

`User explicitly stated no evidence-package agent is required and instructed femdation to use subagents and resolve the provenance differences. Substitute packaging by a direct general subagent is authorized for this State 13 lane.`

## Previous Fallback Issue

- `.beads/vb-vt2f/truth-serum-report.md:149-156` rejected the previous State 13 bundle because the required named `evidence-packaging` delegate was unavailable/fell back and the bundle did not disclose that provenance.
- `.beads/vb-vt2f/truth-serum-report.md:163-168` required either a valid named specialist rerun or an explicit approved provenance waiver.
- The owner-authorized substitute manifest provides that waiver route, and this artifact makes the provenance auditable.

## Non-Laundering Statement

This packaging result is explicit substitute provenance, not laundered specialist provenance. It does not claim that the missing `evidence-packaging` agent ran, and it does not convert the prior fallback/default-agent artifact into specialist approval. The approval is limited to owner-authorized State 13 substitute packaging by a direct child subagent using the isolated workspace and existing evidence artifacts.

## Decision

The provenance difference is disclosed and accepted for this State 13 lane. Downstream reviewers should evaluate `.beads/vb-vt2f/assurance-bundle.md` as an owner-authorized substitute evidence package, not as output from the unavailable `evidence-packaging` specialist.
