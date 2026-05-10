# Formal Verification Report

**STATUS: REJECTED**

## Summary

All 9 Lean layer proof obligations are **FAIL** due to missing tool (Lake). Per formal-verifier skill rule `tool_missing_is_not_pass`: if a required tool such as lake is missing, mark the obligation FAIL.

## Inputs

- proof-obligations.jsonl (vb-i94f): 33 total obligations, 7 lean layer
- proof-obligations.jsonl (vb-h6ix): 28 total obligations, 2 lean layer
- contract-verification-review.md: **NOT PRESENT** in vb-i94f or vb-h6ix beads
- traceability-matrix.jsonl: present in both beads

## Tool Availability

| Tool | Available | Version/Status |
|------|-----------|----------------|
| lake | **NO** | `which lake` returns empty; command not found |
| elan | **NO** | Not installed |
| lean | **NO** | Not installed |
| cargo | YES | Available |
| moon | UNKNOWN | Not checked |

## Installation Attempts (All Failed)

1. `which lake` → not found
2. `elan --version` → elan not found
3. `cargo install lake` → lake v0.2.0 has no binaries to install
4. GitHub release download (`lake-linux-x86_64.tar.gz`) → 404 Not Found
5. `curl https://raw.githubusercontent.com/leanprover/elan/master/elan/install_ctp.sh` → 404
6. Arch Linux package search (`pacman -Ss lake`) → no lake package

## Obligation Results

### vb-i94f Lean Obligations (7 total)

| ID | Target | Claim | Layer | Checker | Result | Evidence |
|----|--------|-------|-------|---------|--------|----------|
| POST-001 | `crate::engine::expr_eval::core::eval_expr_inner` | taint_accum at expr exit equals lattice join of all LoadSlot and LoadAccessor slot taints consumed | lean | lake build | **FAIL** | tool_missing |
| POST-002 | `crate::engine::object_list::build_object_with_taint` | accumulated_taint equals fold join over field taints; Clean iff all fields Clean | lean | lake build | **FAIL** | tool_missing |
| POST-003 | `crate::engine::object_list::build_list_with_taint` | accumulated_taint equals fold join over item taints; Clean iff all items Clean | lean | lake build | **FAIL** | tool_missing |
| POST-005 | `crate::engine::node_helpers::finish_run` | EngineSignal::Finished taint exactly equals read_taint(result) | lean | lake build | **FAIL** | tool_missing |
| INV-001 | `crate::value::join_taint` | join_taint is monotone: if a >= b then join(a,c) >= join(b,c) | lean | lake build | **FAIL** | tool_missing |
| INV-002 | `crate::value::join_taint` | join_taint is commutative, associative, Secret=top, Clean=bottom | lean | lake build | **FAIL** | tool_missing |
| INV-005 | `crate::engine::node_helpers::finish_run` | EngineSignal::Finished taint exactly equals read_taint(result); same as POST-005 | lean | lake build | **FAIL** | tool_missing |

### vb-h6ix Lean Obligations (2 total)

| ID | Target | Claim | Layer | Checker | Result | Evidence |
|----|--------|-------|-------|---------|--------|----------|
| INV-002 | `vb_storage/src/recovery/replay/core.rs` | latest attempt selection is independent of wall clock time; ordering determined by EventSeq and attempt number only | lean | lake build | **FAIL** | tool_missing |
| POST-003 | `vb_storage/src/recovery/replay/core.rs` | max attempt number observed across all events for the run is selected | lean | lake build | **FAIL** | tool_missing |

## Deliverables Created

1. `lean_proofs/lean-toolchain` - Lean version record (skeleton)
2. `lean_proofs/Lakefile.lean` - Lake build config (skeleton, cannot build)
3. `lean_proofs/Taint.lean` - Taint lattice model + INV-001/INV-002 proofs (admit blocks, incomplete)
4. `lean_proofs/EvalTaint.lean` - eval_expr taint accumulation proof (POST-001, admit blocks)
5. `lean_proofs/BuildTaint.lean` - build_object/list taint proofs (POST-002, POST-003, admit blocks)
6. `lean_proofs/FinishTaint.lean` - finish_run taint preservation (POST-005, INV-005, admit blocks)
7. `lean_proofs/ReplayAttempt.lean` - vb_storage replay attempt selection (INV-002, POST-003, admit blocks)
8. `lean_proofs/lean-build-report.txt` - output of `lake build` (shows command not found)

## Waivers

None. No formal-waivers.jsonl exists in vb-i94f or vb-h6ix beads.

## Residual Risk

**CRITICAL**: All 9 Lean proof obligations cannot be verified because Lake is not installed and cannot be installed with available tools. The proofs exist only as skeleton `.lean` files with `admit` blocks - they do not actually verify anything.

### Blocker Summary

| Blocker | Severity | Description |
|---------|----------|-------------|
| lake_missing | CRITICAL | Lake is not installed. Cannot run `lake build`. |
| no_elan | HIGH | elan (Lean toolchain manager) is not available |
| no_contract_approval | HIGH | contract-verification-review.md does not exist in vb-i94f or vb-h6ix |

### Required Actions

1. **Install Lake 4.x** using elan: `curl -fsSL https://raw.githubusercontent.com/leanprover/elan/master/elan-install.sh | sh`
2. **Verify contract approval** exists before running gauntlet
3. **Complete admit blocks** in all `.lean` files after Lake is available
4. **Run `lake build`** and update lean-build-report.txt with actual results

## Command Evidence

```bash
$ which lake
# (empty - lake not found)

$ cd /home/lewis/src/Velvet-ballistics/lean_proofs && lake build
zsh:1: command not found: lake
# Exit code: 127
```
