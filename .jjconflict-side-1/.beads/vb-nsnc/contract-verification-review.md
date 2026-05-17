# Contract Verification Review

STATUS: REJECTED

## Files Reviewed
- contract.md
- tla-spec.md (MISSING)
- lean-contract.md
- verification-layers.md
- proof-obligations.jsonl
- traceability-matrix.jsonl

## Command Evidence
- `jq -c . .beads/vb-nsnc/proof-obligations.jsonl` -> valid JSONL
- `jq -c . .beads/vb-nsnc/traceability-matrix.jsonl` -> valid JSONL
- `test -s .beads/vb-nsnc/tla-spec.md` -> MISSING (blocker)

## Findings

### Severity: LETHAL
- **Clause:** N/A (missing file)
- **Problem:** `tla-spec.md` is absent. Per mandatory gate rule `tla_temporal_default`, the spec is required. However, this contract describes pure cold-path data schema validation with no temporal/state-over-time behavior, no workflows, no protocols, no concurrent state, no lifecycle transitions, and no scheduler behavior. TLA+ is not applicable.
- **Required fix:** Either (a) add `tla-spec.md` with a waiver explaining TLA+ is inapplicable because this is static data validation (pure functions over bounded strings), naming limitation, owner, expiry, and compensating evidence; or (b) if the reviewer accepts TLA+ inapplicability as self-evident for pure data-schema validation, waive this requirement with explicit rationale. The contract cannot proceed without resolving this gate.

### Severity: LETHAL
- **Clause:** Review Axis 5 / rule `executable_obligation_schema`
- **Problem:** Every `proof-obligations.jsonl` entry is missing the required fields: `expected_evidence`, `risk`, `scope`, `owner_state`, `rerun_from`. The rule states these are mandatory for every line. No entry has `expected_evidence` (mechanically observable pass/fail criterion), `risk` (high/medium/low), `scope` (exact package/target/function), `owner_state` (who owns rerunning), or `rerun_from` (restart point). All 31 entries fail this schema contract.
- **Required fix:** Every JSONL entry must include all 16 required fields. Add `expected_evidence` with exact pass/fail observable criterion per entry (e.g., "exit code 0, no CapabilityName* in stderr"). Add `risk` field (high/medium/low). Add `scope` (exact Rust module path or test target). Add `owner_state` and `rerun_from` for each.

### Severity: MAJOR
- **Clause:** AC-8 / STATIC-SAFETY-001
- **Problem:** Rule `source_lint_not_test_style` requires source clippy to target production/source code, not test helper structure. `STATIC-SAFETY-001` uses `moon ci` which is a crate-wide gate — it may lint test targets as well as production code. The rule explicitly rejects using lint on test targets to judge helper, loop, table-driven, or local-mutability structure.
- **Required fix:** Narrow `STATIC-SAFETY-001` command to explicitly target production source only (e.g., `cargo clippy -p vb_validate --lib --bins`, not `--tests`). Tests are judged by compile, execution, and assertions.

### Severity: MAJOR
- **Clause:** Review Axis 1 / rule `layer_completeness`
- **Problem:** `API-COMPAT-001` uses `contract_clause: "future"` which is not a defined contract clause label. No AC-10 or AC-11 appears in proof-obligations (acceptance criteria for `moon ci` final gate and CLI integration). Traceability matrix covers only 15 of 31 proof obligation entries; some clauses appear in compound form (e.g., "PRE-1 POST-1", "POST-3 POST-4 POST-5 POST-6") making granular traceability opaque.
- **Required fix:** Map all acceptance criteria (AC-1 through AC-11) explicitly. Replace `"future"` with the actual clause being covered. Expand traceability matrix to one entry per contract clause or justify compound grouping.

### Severity: MINOR
- **Clause:** Rule `theorem_contract_required`
- **Problem:** `lean-contract.md` correctly states Verus is not the appropriate layer for this pure-data-validation kernel and that Kani provides compensating formal verification. However, WAIVER-003 in `lean-contract.md` states "Lean theorems not yet encoded" without naming a concrete implementation plan, timeline, or owner for the Lean encoding. The waiver expires "never" but provides no path to eventual Lean adoption.
- **Required fix:** If Lean is the long-term goal, name a concrete owner, milestone, and follow-up condition. If Kani is the permanent compensating layer, rename WAIVER-003 to reflect that Kani is the final layer (not a placeholder) and update expiry rationale accordingly.

## Coverage Decision

| Category | Status |
|----------|--------|
| Contract clauses traced | PARTIAL — I3, I4, I5, I9, I10, POST-3..7, PRE-1, POST-1 covered; AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-7, AC-10, AC-11 absent or compound |
| TLA+-owned clauses covered | N/A — TLA+ not applicable (pure data validation); needs explicit waiver |
| Verus-owned clauses covered | N/A — Kani chosen over Verus for this kernel; correctly justified |
| Theorem-owned clauses covered | PARTIAL — 6 Lean theorems named but not encoded; WAIVER-003 is indefinite |
| Proof obligations traced | YES — all 31 entries trace to clauses via traceability matrix |
| TLA+ scope valid | NO — spec missing; TLA+ inapplicable but not waived |
| Verus scope valid | N/A — Kani used instead |
| Lean/Aeneas/Hax scope valid | PARTIAL — scope correct (pure grammar/length/dup/detection), encoding deferred |
| Waivers valid | PARTIAL — WAIVER-001 and WAIVER-002 are well-formed; WAIVER-003 is indefinite |

## Summary

The contract describes a pure cold-path data schema validation task — grammar checking, length bounds, action-relation enforcement, and duplicate detection over bounded capability name strings. This is not a temporal, concurrent, protocol, or state-machine problem, so TLA+ is inapplicable but requires an explicit waiver rather than absence.

The proof obligation coverage for the actual validation logic (I3, I4, I5, I9) is strong: Kani for bounded model checking, proptest for property-based grammar coverage, cargo-fuzz for adversarial inputs, and unit/integration/e2e for concrete cases. This is the right verification stack for pure data validation.

The blocking issues are: (1) missing TLA+ spec or waiver — must be resolved before approval; (2) proof-obligations.jsonl missing 5 required fields on all 31 entries — correctable; (3) static safety scope may include test targets — needs narrowing to production source only.

This contract can reach APPROVED once: a TLA+ inapplicability waiver is added (or TLA+ spec is added if temporal behavior is identified), all 31 proof-obligation entries are augmented with expected_evidence/risk/scope/owner_state/rerun_from, and STATIC-SAFETY-001 command is narrowed to production source only.
