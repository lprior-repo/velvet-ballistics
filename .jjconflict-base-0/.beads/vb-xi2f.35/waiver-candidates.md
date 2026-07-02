# Waiver Candidates: ResourceContract Digest Coverage

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Waiver Policy

Per proof-planner waiver policy: only **non-behavior exceptions** may become waiver candidates. Behavior-affecting waiver candidates are invalid and must be rejected by the proof-plan-reviewer.

## Candidate WC-001: YAML Contract Parsing (Clause C7)

### Classification

| Field | Value |
|-------|-------|
| **Waiver ID** | WC-001 |
| **Contract Clause** | C7: YAML Contract Parsing |
| **Proof Seeds** | PS-011 |
| **Obligations** | PO-F01 (cargo-fuzz), Kani/Ptest waivers |
| **Behavior-Affecting** | NO — currently, all contracts are DEFAULT. No YAML-sourced contracts exist. Adding YAML contract parsing enables a new feature but does not change existing DEFAULT-based behavior. |
| **Priority** | P2 per contract.md C10: explicitly out of scope for this bead. |
| **Severity** | MEDIUM (contract.md C7 status: NOT-IMPLEMENTED) |

### Reason

The YAML parser currently has no support for `resource_contract` sections. The parser whitelist in `vb_yaml/src/ast/parse.rs` rejects unknown fields including `resource_contract`. `WorkflowSource` has no contract fields. All contracts are `ResourceContract::DEFAULT`, hardcoded at compilation entry points.

Adding YAML contract parsing:
- Is feature work (not a bug fix)
- Would change the user-facing API (YAML authoring)
- Requires coordination with the YAML language specification
- Is explicitly deferred to P2 per contract.md Sections C7 and C10

No behavior is affected by deferring this to P2: workflows continue to use DEFAULT contracts regardless of whether the parser recognizes a `resource_contract` section.

### Boundary Proof

The current system produces identical behavior with or without YAML contract parsing:
1. Parser rejects unknown fields including `resource_contract` (current behavior).
2. All compilation entry points hardcode `ResourceContract::DEFAULT` (current behavior).
3. After P1 fix: entry points accept an explicit `contract: ResourceContract` parameter, but the DEFAULT is unchanged.
4. P2 future: YAML-sourced contracts would override the DEFAULT, but until then, all compiled workflows use DEFAULT.
5. The DEFAULT contract itself is unchanged by this bead — it is a well-known constant.

### Compensating Evidence

- All existing YAML parsing tests pass (YAML parser is unchanged by P1).
- The parser whitelist continues to reject `resource_contract` as an unknown field (safe rejection, not a panic).
- P2 bead will add cargo-fuzz, Kani, and Proptest coverage for parser changes.

### Owner, Expiry, Follow-Up

| Field | Value |
|-------|-------|
| **Owner** | Bead tracker: P2 bead assigned for YAML contract parsing |
| **Expiry** | Must be resolved before any YAML-sourced contract feature ships |
| **Follow-Up Bead** | TBD — YAML contract parsing bead (part of vb-xi2f.35 series or follow-on) |
| **Reviewer Status** | Pending proof-plan-reviewer approval |

### Scope Boundary

This waiver applies to:
- PO-F01 (cargo-fuzz of YAML parser contract section)
- Kani obligations for PS-011 (parser invariants)
- Proptest obligations for PS-011 (valid/invalid YAML generation)

This waiver specifically does NOT apply to:
- Any digest computation proof (C1, C3, C4) — all are P1 critical
- Any type resolution proof (C2) — all are P1 critical
- Any validation proof (C5) — all are P1
- Any backward compatibility proof (C8) — all are P1
- Runtime enforcement proofs (C4) — all are P1

## No Other Waiver Candidates

All other 16 proof seeds address CRITICAL or HIGH severity hazards that are currently active bugs. They are behavior-affecting and cannot be waived:

- **PS-001 through PS-004** (C1: digest orphan): CRITICAL active bug — digest insensitive to contract changes.
- **PS-005, PS-006** (C2: duplicate types): HIGH active divergence — two types with same name, different fields.
- **PS-007, PS-017** (C3: entry points): HIGH active bug — all entry points hardcode DEFAULT.
- **PS-008, PS-009** (C4: taint): HIGH active security gap — allows_secret_results not hashed.
- **PS-010** (C6: dual paths): MEDIUM at-risk — drift possible between two paths.
- **PS-012** (C5: validation): HIGH active gap — two contract fields not validated.
- **PS-013, PS-014** (C1: test coverage): HIGH active gap — zero tests for this behavior.
- **PS-015** (C1: digest split): MEDIUM architectural concern.
- **PS-016** (C1: encoding injectivity): MEDIUM future-proofing.

These are all behavior-affecting. No waiver can apply.
