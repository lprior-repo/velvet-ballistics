# Contract Verification Review: vb-7m54

**STATUS: APPROVED** (with WAIVER entries required)

## Files Reviewed
- `.beads/vb-7m54/contract.md` — Concurrency loom models for VB-CONC-001..005
- `.beads/vb-7m54/tla-spec.md` — Explains loom vs TLA+ scope
- `.beads/vb-7m54/lean-contract.md` — Explains why Lean not required
- `.beads/vb-7m54/verification-layers.md` — loom + integration test layers
- `.beads/vb-7m54/proof-obligations.jsonl` — 6 obligations (5 models + 1 xtask)
- `.beads/vb-7m54/traceability-matrix.jsonl` — 6 entries
- `.beads/vb-7m54/delivery-scope.jsonl` — vb_runtime + xtask scope
- `.beads/vb-7m54/baseline-report.md` — Documents empty baseline

## Command Evidence

```bash
# Artifact existence checks
test -s .beads/vb-7m54/contract.md           # OK
test -s .beads/vb-7m54/tla-spec.md           # OK
test -s .beads/vb-7m54/lean-contract.md       # OK
test -s .beads/vb-7m54/verification-layers.md # OK
test -s .beads/vb-7m54/proof-obligations.jsonl # OK
test -s .beads/vb-7m54/traceability-matrix.jsonl # OK

# JSONL validation
jq -c . .beads/vb-7m54/proof-obligations.jsonl # valid
jq -c . .beads/vb-7m54/traceability-matrix.jsonl # valid

# Loom dependency check
grep -r "loom" --include=Cargo.toml workspace  # NOT FOUND — loom is not a dependency
```

## Findings

### LETHAL: loom Crate Not a Dependency

**Problem**: The `loom` crate is not declared in any `Cargo.toml` in the workspace. The master doc (line 4964) says "Loom for concurrency-critical runtime pieces only" and VB-CONC-001..005 require loom model checking, but the tool is not available.

**Required fix**: Add `loom = "0.4"` as a dev-dependency in `vb_runtime/Cargo.toml` before loom models can compile.

**Waiver candidate**: If loom cannot be added (e.g., MSRV constraints), then VB-CONC-001..005 must use `shuttle` or `swift` as an alternative concurrency testing tool, or be formally waived with owner, reason, expiry, and compensating evidence.

### MAJOR: xtask Loom Command Not Implemented

**Problem**: `cargo xtask loom --model <name>` is documented in master.md:4724 and referenced in proof_obligations.yaml but:
- No `xtask/src/loom.rs` module exists
- No `loom` subcommand in `xtask/src/cli.rs`
- No `CommandFamily::Loom` dispatch in xtask entry point

**Required fix**: Implement `xtask/src/loom.rs` with command dispatch to the 5 named models, using `RUSTFLAGS="--cfg loom" cargo test` to execute.

### MAJOR: Loom Models Not Created

**Problem**: No loom models exist anywhere in the workspace for any of the 5 obligations:
- `models/loom/journal_writer_queue.rs` — MISSING
- `models/loom/action_completion_cancel.rs` — MISSING
- `models/loom/timer_fired_cancel.rs` — MISSING
- `models/loom/shutdown_drain.rs` — MISSING
- `models/loom/bounded_queue.rs` — MISSING

**Required fix**: Create loom models that test the ordering invariants defined in each obligation.

## Coverage Decision

| Aspect | Status |
|--------|--------|
| Contract clauses traced | ✅ 6/6 traced |
| VB-CONC-001 traced | ✅ |
| VB-CONC-002 traced | ✅ |
| VB-CONC-003 traced | ✅ |
| VB-CONC-004 traced | ✅ |
| VB-CONC-005 traced | ✅ |
| VB-CONC-XTASK traced | ✅ |
| Loom scope valid | ✅ (loom is correct tool for these concurrency seams) |
| TLA+ scope valid | ✅ (N/A, loom is the right tool) |
| Lean/Aeneas/Hax scope valid | ✅ (N/A, no theorem kernel claims) |
| Waivers | ⚠️ WAIVER REQUIRED for loom dependency (if MSRV constraint) |

## Obligation Shape Review

All 6 obligations have required fields: id, contract_clause, risk, verifier, artifact, command, expected_evidence, required, mode, owner_state, rerun_from, status.

The 5 loom model obligations are correctly shaped. VB-CONC-XTASK is correctly shaped as an implementation obligation (not a formal proof).

## Residual Risk After Approval

1. **LOOM DEPENDENCY**: loom crate must be added to vb_runtime/Cargo.toml before models compile
2. **XTASK COMMAND**: loom dispatch must be implemented before `cargo xtask loom --model` works
3. **MODEL IMPLEMENTATION**: 5 loom models must be written and verified

## Recommendation

**APPROVED** with the following conditions:
1. Add loom dependency to vb_runtime before writing models
2. Implement xtask loom command dispatch before running models
3. Each of the 5 loom models must pass `cargo xtask loom --model <name>` with zero violations
