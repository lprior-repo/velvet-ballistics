# vb-jskf Proof Review

## Findings

1. **CRITICAL — `kani/admission_atomic_sequence_k01_k03.rs` K01 is hardcoded and non-admissible.** Obligation `K01/KANI-PROP-007`. Lines 24-46 use fixed `EventSeq::new(1)`, `RunId::new(1)`, `WorkflowDigest::from_bytes([1;32])`; no production admission/readback call is checked.
2. **CRITICAL — `verification/verus/yaml_e2e_digest_roles.rs` is a detached mirror model.** Obligations `VERUS-DIG-004/005`. File header admits BLAKE3, Fjall I/O, postcard decode, and runtime scheduling are trusted shell boundaries; proofs only cover pure int/bool predicates.
3. **HIGH — `verification/verus/accepted_run_atomic_admission.rs` is a detached mirror model.** Obligations `VERUS-PRE-001..VERUS-ERR-006`. File header admits Fjall/codecs/runtime/production structs are outside proof.
4. **HIGH — `verification/tla/YamlE2eChain.tla` hides bounded runtime/resource failures.** Obligations `TLA-LIFE-001/TLA-DUR-002/TLA-REC-003`. It extends `Naturals` and abstracts parser/digest/proof/replay as booleans.
5. **HIGH — `verification/tla/EngineYamlRecovery.tla` is too synthetic.** Obligation `TLA-REC-001/PO-004`. Replay is `seq < 3`; no journal order, digest, snapshot/tail, overflow, or durable failure model.
6. **HIGH — `kani/pipeline.rs` uses a fixed workflow shape.** Obligation `K30`. It proves one two-node happy-path shape and only asserts entry range if validation succeeds.
7. **MEDIUM — `verification/tla/EngineYamlRunLifecycle.tla` guards overflow away.** Obligation `TLA-LIFE-001/PO-003`. `seq < MaxSeq` prevents overflow but does not model the required Err/suspend/fail transition at hardware bounds.

## Review decision

I did not rewrite proofs. These are real proof-design gaps, not small mathematical edits. I added non-admission/superseding ledger rows so the listed artifacts cannot be cited as production validation evidence until rebound to production structures/traces and bounded failure models.

## Existing/open bead mapping

- `vb-jskf`: owner for YAML recovery proof binding and non-admission triage.
- `vb-mnv0`: related open Kani generator/arbitrary shape work.
- `vb-h3fx`: related open Verus production API binding pattern.
- `vb-w20g`: related open bounded TLA/resource/overflow model pattern.

STATUS: REJECTED
