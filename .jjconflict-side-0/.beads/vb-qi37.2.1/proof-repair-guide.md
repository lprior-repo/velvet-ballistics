# Proof Repair Guide: vb-qi37.2.1

**For proof-writer — rerun required**

## Rejection Reason

Proof-writer stage has not produced required artifacts. All 43 obligations are unexecuted.

## Required Artifacts (all currently MISSING)

1. **`.beads/vb-qi37.2.1/proof-writer-report.md`**
   - Must contain: obligation-to-artifact mapping, verifier commands run, raw stdout/stderr excerpts, pass/fail status
   - Format: markdown with sections per layer (Lean, Kani, proptest, integration, unit, static, fuzz, mutants, llvm-cov, gauntlet)

2. **`.beads/vb-qi37.2.1/proof-evidence.md`**
   - Must contain: raw command output (exit codes, stdout/stderr, artifact paths) for each verifier run
   - Every of the 43 obligations must have evidence or a formally accepted waiver

3. **`.beads/vb-qi37.2.1/proof-strategy.md`**
   - Must contain: tool selection rationale per layer, evidence standards used, vacuity checks applied, waiver documentation with owner/expiry/compensating-evidence

4. **`.beads/vb-qi37.2.1/proof-obligations.planned.jsonl`**
   - Must be an extended copy of `proof-obligations.jsonl` with added fields: `status`, `evidence_path`, `command_run`, `waiver_applied`

## Required Verifier Commands

Execute from the isolated workspace (`/home/lewis/src/vb-qi37-2-1`):

### Lean Layer (6 obligations)
```bash
cd /home/lewis/src/vb-qi37-2-1
lake build  # or: moon run :verify-proof
```
Evidence: `lean-report.md` or build artifacts

### Kani Layer (5 obligations)
```bash
cargo kani -p vb_core --no-default-features --lib --harness aggregate_resource_budget_kani 2>&1
```
Evidence: kani output with pass/fail per harness

### Proptest Layer (7 obligations)
```bash
cargo test -p vb_core --test aggregate_resource_budget_properties 2>&1
```
Evidence: test output with pass/fail counts

### Integration Layer (11 obligations)
```bash
cargo nextest run -p vb_runtime admission 2>&1
cargo nextest run -p vb_runtime shard 2>&1
```
Evidence: nextest output

### Unit Layer (5 obligations)
```bash
cargo nextest run -p vb_core aggregate 2>&1
```
Evidence: nextest output

### Static Layer (3 obligations)
```bash
cargo clippy -p vb_core -p vb_runtime 2>&1
moon ci 2>&1
```
Evidence: clippy/moon output

### Fuzz Layer (2 obligations)
```bash
cargo fuzz run workflow_aggregate_target 2>&1
cargo fuzz run artifact_aggregate_target 2>&1
```
Evidence: fuzz output

### Mutants Layer (1 obligation)
```bash
cargo mutants -p vb_core -p vb_runtime 2>&1
```
Evidence: mutants output with kill rate

### Coverage Layer (1 obligation)
```bash
cargo llvm-cov -p vb_core -p vb_runtime 2>&1
```
Evidence: coverage report

### Gauntlet Layer (2 obligations)
```bash
moon run :verify-proof 2>&1
moon run :verify-all 2>&1
```
Evidence: moon output

## Waiver Handling

WAIVER-001 and WAIVER-002 in lean-contract.md are permanent architectural waivers. They must be accepted by proof-reviewer only after compensating evidence (integration tests, Kani harnesses, proptest invariants) are verified as executed and passing.

## After Artifacts Are Written

proof-reviewer will re-evaluate with these artifacts present.
