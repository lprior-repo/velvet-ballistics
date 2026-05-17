# proof-repair-guide.md — vb-qi37.13.2

## STATUS: REJECTED — Required artifacts absent

### Blockers Preventing Proof Review

The following proof artifacts are listed in `STATE.md` phase 5 as produced but are **not present** in `.beads/vb-qi37.13.2/`:

| Artifact | Required For |
|---|---|
| `proof-writer-report.md` | Proof execution summary, verifier command log, pass/fail per obligation |
| `proof-evidence.md` | Raw command output, harness paths, coverage data |
| `proof-strategy.md` | Verifier selection rationale, obligation mapping |
| `proof-obligations.planned.jsonl` | Machine-readable obligation list for verification gate |

### Required Fixes

1. **Persist `proof-writer-report.md`** — Summarize the proof execution: which verifier was invoked (Kani/proptest), which harnesses/property blocks were run, and the per-obligation pass/fail outcome.

2. **Persist `proof-evidence.md`** — Record raw command output for every verifier invocation. Include paths to generated harnesses (`kani/qi37-13-2-diagnostic_envelope.rs`) and proptest blocks (`tests/cli_envelope_proptest.rs`).

3. **Persist `proof-strategy.md`** — Document verifier selection: why Kani for PO-001,003,004,008 and proptest for PO-002,005,006. Map each obligation to the verifying artifact.

4. **Persist `proof-obligations.planned.jsonl`** — Machine-readable list of all obligations with ID, description, verifier, harness path, and expected result.

### Routing

Route back to **proof-writer (phase 5)**. Once all four artifacts are materialized in `.beads/vb-qi37.13.2/`, re-invoke **proof-reviewer (phase 6)**.
