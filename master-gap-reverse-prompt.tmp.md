# Reverse Prompt: Master Gap Improvement Sprint

Use this prompt to bring a new agent up to speed on `velvet-ballastics` and drive the next improvement pass from the master contract.

```text
You are working in /home/lewis/src/Velvet-ballistics.

Authoritative source of truth:
- /home/lewis/src/Velvet-ballistics/velvet-ballistics-MASTER.md
- AGENTS.md in the repository root

Canonical naming:
- Product, binary, and package: velvet-ballastics
- Rust crate/module: velvet_ballastics
- The spelling velvet-ballistics is invalid except for the existing repository path, master filename, or explicitly labeled migration artifacts.

First commands:
- Run bd prime.
- Run bd list and bd ready serially, not in parallel, because embedded Dolt can hold a single writer lock.
- Do not create markdown task lists as tracking artifacts. Use beads for issue tracking.
- Do not commit unless the user explicitly asks.

Current high-level state:
- The repository has broad implementation surface for phases 1-37 and some phase-extension work through 46.
- The master document explicitly says public functions are only surface evidence; a phase is not accepted without tests, fuzz/property coverage, benchmarks where applicable, gate output, and closed bead evidence.
- bd currently shows no open or ready work, but 18 issues are in progress/blocked. That state is suspicious and should be reconciled before starting new implementation.

Most important master-doc gaps:
1. Runtime/admission finality is not fully accepted.
   - Master sections: Phase 39, Phase 41, Section 66.
   - Code surface exists in crates/vb_runtime/src/admission.rs and crates/vb_core/src/capability.rs.
   - Gap: AcceptedArtifact, VerificationProof, strict admission, secret availability, durable RunAccepted metadata, and full artifact binding need end-to-end evidence and bead closure.
   - Related beads: vb-2cna, vb-6smd, vb-o1hw.

2. Recovery evidence chain is still a release blocker.
   - Master sections: Phase 40, Phase 44, DRIFT-2, Phase 57.
   - Code surface exists in crates/vb_runtime/src/recovery.rs and storage recovery helpers.
   - Gap: crash recovery must reconstruct live slot values, taint, step states, and deterministic step evidence from journal events. Summary hydration is not enough.
   - Check for silent journal discard patterns such as Ok(()) | Err(_) => {} and replace with typed propagation or logged diagnostics.
   - Related beads: vb-57el, vb-6smd, vb-9fy4, vb-xki0.

3. Generated Rust mode still lacks full final IR parity.
   - Master sections: Phase 32, Phase 58, Round 2 generated Rust gap.
   - Code surface exists in crates/vb_codegen/src/lib.rs.
   - Current generated subset rejects or only partially supports several final IR areas, including Together, Reduce, Repeat internals, expression helpers such as Append/Merge/Sum/Unique, and nested accessor traversal.
   - Gap: every final primitive needs generated-vs-IR equivalence tests and suspension/error parity.
   - Related beads: vb-q46j.

4. Source-to-IR lowering for the full primitive set is not proven.
   - Master sections: Phases 3-12 and Round 2 YAML/validation/compile gap.
   - Code surface exists in crates/vb_compile/src and crates/vb_validate/src.
   - Gap: constructor/API presence is not enough. Need full v1 YAML source lowering evidence for all primitives, validators, accessors, slots, constants, taint, and error paths.
   - Related bead: vb-ygy2.

5. Whole-workflow boundedness and resource enforcement are partly implemented but need hard acceptance.
   - Master sections: Phase 37, Phase 45, Section 64, DRIFT-3.
   - Code surface exists in crates/vb_core/src/budget.rs, crates/vb_core/src/value_store.rs, and ResourceContract in crates/vb_core/src/workflow/mod.rs.
   - Gap: prove nested fanout, sequential sums, conditional max, unbounded rejection, policy ceilings, ValueStore arena caps, StepBudget hard ceiling, and collect per-run pagination state.
   - Pay attention to validate_budget and BudgetError coverage. There is a bead for completing BudgetError handling.
   - Related beads: vb-rzco, vb-71e3.

6. Runtime taint propagation must be accepted end-to-end.
   - Master sections: Phase 43, DRIFT-1, Section 47.
   - Gap: EvalExpr, BuildObject, BuildList, and Finish must preserve or reject taint at runtime, not only at compile time.
   - The final Finish boundary must carry taint or prevent tainted emission.

7. IR structural validation is a security boundary, not just a compile helper.
   - Master sections: Phase 46, DRIFT-4.
   - Gap: CompiledWorkflow::try_from_parts must treat decoded artifacts as untrusted and validate reachable nodes, forward-only edges, loop pairing, SymbolId ranges, and accessor path segments.
   - Verify this is actually enforced on artifact loading and run-compiled paths, not only in compiler tests.

8. Validation deduplication remains architectural debt.
   - Master sections: Phase 42, DRIFT-5.
   - Gap: vb_validate and vb_compile still need a single shared validation pipeline or conclusive evidence that duplication has been removed without API breakage.

9. CLI agent contract is not just documentation.
   - Master sections: Section 48 onward, especially agent-context and CLI design rules.
   - Moon has an agent-cli-contract task, but confirm the binary emits the full versioned machine-readable context and rejects banned vocabulary through scripts/check-agent-cli-contract.sh.
   - Related CLI beads include vb-0x7b, vb-3gzb, vb-hub7, vb-lp8v.

10. Release gates need refreshed full evidence.
    - Master sections: 36-41 and 44.
    - Required surfaces: tests, proptests, fuzz targets, miri, coverage, mutants, supply chain, source length, bench-build, maxperf, maxperf-native, benchmark metadata, sanitizer jobs, Moon CI.
    - Fuzz harness files exist under fuzz/src/bin, including yaml_events, expression, ipc_frame, journal_event, compiled_ir, generated_compare, and additional phase-extension fuzzers. Existence is not evidence; record command output.
    - Related beads: vb-uby5 plus full-gate-evidence-refresh from master line 1768.

Known bead state to reconcile:
- bd list showed 18 in-progress issues and zero open/ready issues.
- P0 blocked/in-progress items include vb-2cna, vb-57el, vb-6smd, vb-9fy4, vb-o1hw, vb-q46j, vb-rzco, vb-uby5, vb-xki0, vb-ygy2.
- Before implementation, inspect dependency blockers with bd show <id>. If an issue is stale in-progress but no agent owns it, update/claim according to the beads workflow.

Recommended next sprint order:
1. Reconcile beads so at least one P0 gap is claimable or unblock the dependency chain.
2. Start with recovery evidence chain if engine end-to-end durability is the goal: SlotWritten value+taint journaling, StepStarted/StepSucceeded for deterministic steps, and full live-frame hydration.
3. Then finish admission/artifact binding: AcceptedArtifact, VerificationProof, RunAdmission, durable RunAccepted metadata, capability and secret checks.
4. Then close generated-vs-IR final parity and compiler full primitive lowering.
5. Finish with whole-workflow boundedness/resource enforcement evidence and full gate refresh.

Quality rules:
- Use the smallest correct changes.
- Keep source code free of the forbidden constructs listed in AGENTS.md and master Section 2, including unchecked indexing/slicing/casts/arithmetic, ignored Results, runtime YAML/JSON/HTTP, and unbounded resources.
- Prefer moon ci as the canonical final gate. Use narrower cargo/moon commands while iterating.
- Do not treat compileable scaffolds, public API existence, or placeholder benchmarks as acceptance evidence.

Deliverable expected from the next agent:
- A concise gap-to-bead mapping.
- One claimed bead or a clear blocker update in bd.
- Implemented code only after claiming.
- Verification command outputs.
- bd close or bd update with evidence.
- bd dolt push before ending.
```
