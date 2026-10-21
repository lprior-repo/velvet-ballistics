# Independent code review

**Bead**: vb-9agmk.1  
**State**: independent code-review gate, before separate test review  
**Reviewer**: black-hat-reviewer / independent Codex reviewer  
**Source checkout**: /home/lewis/src/isoloated/velvet-ballistics-vb-9agmk-code-review-sol-20261021  
**Attempt**: 1  
**Exact source**: 948f19cb7cd92afec3d95e9fb285216cc6689d41  
**Contract evidence**: b5705c1508740a913e1ee2a4e6978928a6c6118f, evidence-only child of the exact source; .beads/vb-9agmk/{contract,type-contracts,boundary-map,hazard-analysis}.md.

## Gate Result

**STATUS: REJECTED**

The envelope verifier improves claim honesty, but runtime can verify one execution tuple and enqueue another. A second mismatch allows compiler/storage acceptance of registries rejected by production runtime. These are source correctness findings, independently reproduced through public APIs. Existing runtime weaknesses remain in scope because C6 and master 77.1/77.2 require the checked artifact to be the runtime trust unit; this is not a claim that every affected runtime line was introduced by this candidate.

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|---|---|---|
| C1/C4: executed checks only; no constructor-created durability or full gate claim | Pass for inspected constructors | VerificationProof::new now produces Unverified and false legacy flags; VerifiedCompilation private fields; closures distinguish assumptions. |
| C2: bind workflow, emitted bytes, action declarations and policy | Partial / blocked | Envelope digests recompute, but submitted workflow/contracts can differ from verified stored tuple: FINDING-001. |
| C3: reject invalid candidate before acceptance | Fail | Sparse and duplicate registries pass both new checked constructors: FINDING-002. |
| C5: ten typed obligation categories | Present, not proof closure | Ledger has all ten categories; replay/recovery/taint remain explicit external assumptions. No new formal proof is claimed by this review. |
| C6: runtime independently validates hostile executable payload and evidence | Fail | Stored evidence is checked, while caller-owned executable IR/contracts are not bound to that check: FINDING-001. |
| Source provenance when actual source supplied | Compiler API present; CLI integration pending | compile_verified_artifact binds source bytes; unchanged CLI caller uses unverified legacy constructor. Owned separately by vb-ezymz, integration requirement I-001. |
| Explicit legacy failure | Pass for inspected verifier path | Unverified -> MigrationRequired, unknown schema -> UnsupportedSchemaVersion; focused suite exercises both. |
| Master 77.1/77.2: executes accepted numeric artifacts only | Fail | Strict Runtime::submit_direct accepts altered constant with stale claimed digest. |

## PHASE 2: Farley Engineering Rigor

| Function | Lines | Limit | Status |
|---|---|---|---|
| Aesthetic/function-size gate | Not assessed after correctness rejection | 25 | Deferred per skill instruction to reject at Phase 1 before aesthetics. |

Pure preparation and verification are separated from journal writes in the inspected design. This does not resolve the execution-tuple trust boundary.

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status |
|---|---|
| Zero unsafe / unwrap / expect / panic / todo / dbg in new artifact verification modules | No occurrence observed in source inspection; no whole-repository lint claim. |
| Checked arithmetic / bounded decoding | Explicit IR byte ceiling and checked length conversion inspected; not an exhaustive allocation proof. |
| Parse into trusted state at boundary | Fail: verified stored workflow is discarded and caller-owned workflow/contracts reach runtime queue. |
| Types distinguish compiler candidate from runtime authority | Improved: VerifiedCompilation is private/immutable; serialized ArtifactEvidence remains hostile until checked. |

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status |
|---|---|
| No Option-based state machine in new evidence schema | Observed: evidence and closure states use enums. |
| Domain trust transition is explicit and total | Fail: no trusted execution tuple crosses admission into queued submission. |
| One contract-registry meaning across compiler/storage/runtime | Fail: shared IR gate accepts IDs by membership; runtime requires unique dense index==ID. |
| Remaining aesthetic/CUPID review | Deferred after Phase 1 rejection. |

## PHASE 5: The Bitter Truth

Passing the artifact API tests does not demonstrate that the runtime uses the checked artifact. The 15 existing focused tests pass; three independent public probes also pass by asserting the counterexamples exist. Their passing status is evidence of the reported defects, not a green product gate. No broad CI, speed claim, proof claim, or independent test-review approval is supplied here.

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---|---|---|---|
| FINDING-001: bind actual submitted IR and contracts to verified artifact | HIGH / P1 | crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:93 | Open on exact candidate |
| FINDING-002: checked constructors must enforce production registry shape | MEDIUM / P2 | crates/vb_compile/src/artifact.rs:168; crates/vb_storage/src/admission/verification.rs:132 | Open on exact candidate |

### FINDING-001: Bind the actual execution tuple before enqueue

**Location**: crates/vb_runtime/src/admission/parts/chunk_005_admit_core.rs:93; callers crates/vb_runtime/src/runtime.rs:257 and :274; crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:279-284.

**Problem**: Admission verifies the stored AcceptedArtifact and discards the returned CompiledWorkflow. Runtime then enqueues the caller-owned workflow and action contracts. The workflow's claimed digest is sufficient to select the stored artifact, and the supplied contracts are checked only for registry shape/capability coverage/workflow references. Neither the submitted canonical bytes nor submitted action-contract digest is compared with the accepted envelope. Therefore checking a valid stored ledger does not establish that the executable tuple was accepted.

**Evidence**: focused-review-probes.log and archived vb_9agmk_code_review_probes.rs, exact candidate:

1. Persist Strict pure workflow with constant 42. Replace its constant with I64(99), preserve the original claimed workflow digest, reconstruct via CompiledWorkflow::try_from_parts. verify_compiled_artifact rejects the altered workflow; Strict Runtime::submit_direct returns Ok(()).
2. Persist Strict dense action0 workflow with contract None/DeterministicPure/Safe. Submit same workflow with action0 Writes/AtLeastOnceExternal/Unsafe. prepare_compiled_artifact rejects the replacement contracts; Strict Runtime::submit_direct_with_grants_and_contracts returns Ok(()).

Both probes use a real temporary Fjall journal through StorageRuntimeJournal::shared_strict. They stop after public submission acceptance: this report proves admission/enqueue bypass, not an observed external effect.

**Required Fix**: Ensure Strict/Journaled submission enqueues exactly the runtime-verified workflow and accepted contract registry, or recompute and compare the complete submitted tuple before admission/enqueue. Cover contract-free, contract-bearing, input-bearing, queued, IPC and recovery entry points according to their actual call graph. Do not rely on CompiledWorkflow::digest() alone. Preserve Relaxed policy deliberately rather than using it to mask Strict rejection. Add regression oracles expecting rejection of both substitutions before run allocation, queue mutation or journal acceptance.

### FINDING-002: Reject registries the production resolver cannot dispatch

**Location**: crates/vb_compile/src/artifact.rs:168 and crates/vb_storage/src/admission/verification.rs:132. Underlying shared check: crates/vb_validate/src/gates.rs:1432. Production invariant: crates/vb_runtime/src/shard/contract_validation.rs:77.

**Problem**: validate_with_contracts verifies action-ID membership/completeness but does not reject duplicate IDs or require index==ID. Both newly introduced checked constructors therefore certify registry shapes that the runtime immediately rejects. Candidate test helpers use action ID7 at registry index0, hiding the disagreement by stopping at artifact admission rather than Runtime submission.

**Evidence**: checked_artifact_accepts_registry_rejected_by_runtime in the archived probe. For contracts [ID7], compiler/storage construction and independent artifact verification succeed; Runtime returns ContractValidationError: "contract at index 0 has ID 7 (expected 0)". For [ID7, ID7], the same checks succeed; Runtime returns "duplicate action ID". Relaxed runtime policy is used in this probe only to isolate its registry validator from envelope loading; the prepared artifacts use Journaled policy.

**Required Fix**: Put unique/dense/order registry validation in a shared appropriate cold/core layer and invoke it consistently from compiler checked construction, storage preparation/reverification, and runtime dispatch admission. Preserve typed diagnostics. Make action-positive fixtures use an executable registry and add negative sparse/duplicate cases plus a valid dense runtime submission case.

## Cross-stack Integration Requirement

**I-001**: Candidate source still has crates/velvet_ballistics/src/run.rs:102-133 submit_cli_compiled_artifact using VerificationProof::new with claimed gate/durability arguments. This now serializes Unverified, so durable CLI submission cannot satisfy new runtime verification. CLI source provenance and caller migration are required in the assembled stack. Author reports concurrent ownership by vb-ezymz / chat01a160a3-fad9-7291-b93f-793d98ea34ad. This review does not duplicate that owner's implementation or approve an unseen integration revision.

## Quality Gates

| Gate | Result | Evidence |
|---|---|---|
| Candidate focused suite: cargo test -q -p velvet-ballistics-workspace-tests --test vb_9agmk_artifact_evidence -- --nocapture | Exit0; 15 passed,0 failed,0 ignored | focused-existing-tests.log and command JSON |
| Independent public counterexample probes: cargo test -q -p velvet-ballistics-workspace-tests --test vb_9agmk_code_review_probes -- --nocapture | Exit0; 3 counterexample observations passed | focused-review-probes.log and command JSON; source archived beside report |
| Initial probe compile | Exit101; reviewer used nonexistent enum spelling, corrected to AtLeastOnceExternal | focused-review-probes-attempt1.log; not a candidate defect |
| Intermediate two-case probe | Exit0; 2 passed | focused-review-probes-two-cases.log |
| moon ci / full workspace tests / full clippy / formal tools | Not run | Review scope explicitly forbids broad verification; no inherited result represented as a fresh pass. |

Both successful commands were independently bounded under a user systemd scope, MemoryHigh12G/MemoryMax16G/MemorySwapMax0, Cargo jobs2, timeout180s, RUSTC_WRAPPER cleared. Full argv and actual exits are retained. Probe source was temporarily placed under crates/workspace_tests/tests and moved to this evidence directory after execution; no production/test source changes are published in the review commit. Reproduce by copying it back to that test directory at the exact candidate, then invoking the logged command. Existing artifact_evidence_support helpers remain candidate source.

## Verdict

**STATUS: REJECTED**

The candidate makes evidence claims more honest and binds its stored envelope, but does not close runtime executable binding or registry validity. Re-review an exact repaired source revision and its integration with concurrent CLI work. Independent test review remains a separate gate.

## Required Repair Actions

1. HIGH: Bind the submitted workflow and contracts to the verified stored artifact across actual submission routes; preserve both substitution counterexamples as rejection regressions.
2. MEDIUM: Align checked registry validation with unique dense production dispatch; replace invalid positive fixtures and retain sparse/duplicate negatives.
3. Integrate the separately owned CLI migration, then provide its exact assembled source revision for review. Keep parent vb-9agmk open until repairs and independent gates complete.
