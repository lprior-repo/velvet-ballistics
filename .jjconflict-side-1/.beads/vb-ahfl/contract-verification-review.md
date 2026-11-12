# Contract Verification Review: vb-ahfl State 6 Attempt 3

STATUS: REJECTED

## Skill Basis

- Read `/home/lewis/.claude/skills/contract-verification-reviewer/SKILL.md`: lines 21-32 require independent JSONL-backed review, TLA+/Verus-first coverage, executable obligations, and no hallucinated evidence.
- Read `/home/lewis/.agents/skills/contract-verification-reviewer/SKILL.md`: same v1.5.0 content; per startup rule, `.agents` would win on conflict.

## Files Reviewed

- `.beads/vb-ahfl/contract.md`
- `.beads/vb-ahfl/tla-spec.md`
- `.beads/vb-ahfl/lean-contract.md`
- `.beads/vb-ahfl/verification-layers.md`
- `.beads/vb-ahfl/proof-obligations.jsonl`
- `.beads/vb-ahfl/traceability-matrix.jsonl`
- `.beads/vb-ahfl/proof-obligations.planned.jsonl`
- `.beads/vb-ahfl/proof-writer-report.md`
- `.beads/vb-ahfl/proof-evidence.md`
- `.beads/vb-ahfl/proof-review.md`
- `.beads/vb-ahfl/proof-findings.jsonl`
- `.beads/vb-ahfl/STATE.md`

## Command Evidence

- `test -s .beads/vb-ahfl/contract.md && test -s .beads/vb-ahfl/tla-spec.md && test -s .beads/vb-ahfl/lean-contract.md && test -s .beads/vb-ahfl/verification-layers.md && test -s .beads/vb-ahfl/proof-obligations.jsonl && test -s .beads/vb-ahfl/traceability-matrix.jsonl && test -s .beads/vb-ahfl/proof-obligations.planned.jsonl && test -s .beads/vb-ahfl/proof-writer-report.md && test -s .beads/vb-ahfl/proof-evidence.md && test -s .beads/vb-ahfl/proof-review.md && test -s .beads/vb-ahfl/proof-findings.jsonl && jq -c . .beads/vb-ahfl/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-ahfl/traceability-matrix.jsonl >/dev/null && jq -c . .beads/vb-ahfl/proof-obligations.planned.jsonl >/dev/null && jq -c . .beads/vb-ahfl/proof-findings.jsonl >/dev/null` -> exit 0.
- `jq -e -s 'all(.[]; . as $row | ["id","contract_clause","target","claim","layer","checker","command","evidence","expected_evidence","risk","scope","required","mode","owner_state","rerun_from","status"] | all(.[]; . as $k | ($row | has($k)))) and all(.[]; .status == "planned")' .beads/vb-ahfl/proof-obligations.jsonl` -> exit 0, output `true`.
- `jq -r 'select((.risk|test("critical|high|proof|release")) and (.command|test("^WAIVER:|^waived$|not_applicable"))) | [.id,.risk,.layer,.required,.command] | @tsv' .beads/vb-ahfl/proof-obligations.jsonl .beads/vb-ahfl/proof-obligations.planned.jsonl` -> exit 0; found required high/proof/critical/release waived obligations including `VERUS-META-001`, `VERUS-BOUNDS-001`, `VERUS-REDACT-001`, `VERUS-GRAPH-001`, `KANI-CANON-001`, `PROP-PARITY-001`, `API-COMPAT-001`, `MUT-ERR-001`, and `FUZZ-REDACT-001`.

## Findings

1. Severity: LETHAL
   - Clause: `PRE-001` / `BLOCKER-SCOPE-001` / `MANUAL-SCOPE-001`
   - Problem: The reviewed contract remains provisional UI schema parity while the bead reality is still recorded as engine YAML-to-IR semantic evidence (`contract.md:7`, `proof-writer-report.md:27`, `proof-review.md:11`). This invalidates TLA+/Verus layer selection: if the real scope is engine YAML-to-IR, temporal compile/admit/run/journal/replay behavior requires a real TLA+ model, not the UI-scope `WAIVED-TLA-001` waiver.
   - Required fix: Owner/orchestrator must either accept UI artifact schema parity as the `vb-ahfl` scope in the bead/orchestrator record, or regenerate State 2/3/4/5 for engine YAML-to-IR with TLA+ coverage for the workflow lifecycle.

2. Severity: LETHAL
   - Clause: `PRE-002`, `PRE-003`, `PRE-005`, `POST-001`, `POST-002`, `POST-003`, `POST-004`, `POST-005`, `POST-006`, `INV-001`, `INV-003`, `INV-004`, `INV-005`, `INV-006`
   - Problem: Required Verus-first production-bound obligations remain waivers after State 5. `proof-obligations.jsonl` still uses `layer: waiver`/`WAIVER:` for `VERUS-META-001`, `VERUS-BOUNDS-001`, `VERUS-REDACT-001`, and `VERUS-GRAPH-001`; `proof-writer-report.md:28-31` states the Verus pass is only an abstract local model and not production-bound.
   - Required fix: Replace these waiver rows with exact production-bound Verus commands naming concrete Rust modules/functions/specs/proofs and raw verifier output, or route back until implementation target discovery creates the proof surface.

3. Severity: LETHAL
   - Clause: `PRE-004`, `POST-007`, `INV-002`
   - Problem: `KANI-CANON-001` remains a high-risk waiver/blocked target after its planned owner state. `proof-writer-report.md:32` and `proof-review.md:13` record no canonicalization API or Kani harness. The current obligation cannot prove bounded canonicalization cannot report false parity.
   - Required fix: Name canonicalization APIs and provide an exact `cargo kani` harness command with bounds and raw pass evidence, or regenerate the proof plan after targets exist.

4. Severity: MAJOR
   - Clause: `POST-007`, `INV-002`, `PRE-005`, `POST-006`, `INV-004`, `API-001`, `UiArtifactError::*`, `REL-001`
   - Problem: Required property, fuzz, API compatibility, mutation, and CI/release obligations are still waived, later-state-owned, or not run (`proof-evidence.md:67-74`, `proof-findings.jsonl:3`). These are valid future work markers, not approval evidence.
   - Required fix: Keep State 6 rejected until the owning states replace each waiver with exact executable commands and raw evidence, or record state-appropriate reviewer-approved waivers that do not mask final closure.

5. Severity: MAJOR
   - Clause: `PRE-006`, `POST-008`, `INV-007`, `STATIC-BOUNDARY-001`
   - Problem: The executable static boundary obligation is overbroad and failed its expected no-match evidence by matching `tokio`/`async` in a source comment (`proof-review.md:17`, `proof-findings.jsonl:4`). As written, it does not reliably distinguish dependency/import violations from documentation text.
   - Required fix: Refine the static boundary gate to inspect dependencies/imports or comment-stripped source, then rerun and record raw output.

## Coverage Decision

- Contract clauses traced: structurally traced in JSONL, but not approval-worthy because scope is unresolved.
- TLA+-owned clauses covered: no; UI-scope waiver is insufficient while `BLOCKER-SCOPE-001` can still restore engine YAML-to-IR temporal scope.
- Verus-owned clauses covered: no; production-bound Verus obligations remain waivers/abstract local model only.
- Theorem-owned clauses covered: acceptable only for provisional UI scope; blocked by unresolved scope.
- Proof obligations traced: structurally yes; substantively no for production closure.
- TLA+ scope valid: no under unresolved bead scope.
- Verus scope valid: no for production-bound closure.
- Lean/Aeneas/Hax scope valid: provisionally yes for UI scope only.
- Waivers valid: no for State 6 approval; multiple required high/proof/critical waivers have reached or passed their owner-state replacement point.
