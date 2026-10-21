# Stopped independent CODE review

**STATUS: REJECTED** — confirmed identity defect; remaining audit stopped at user request. This is rejection evidence, not a completed acceptance review.

Exact combined candidate: 376979e05670c4771700f61b73f50fec23b02149, including source candidate31e5182d0d739b345e1ea608c495033c2f1bf869. Follow-up bug: vb-a2l02, discovered-from vb-6jn77. Review bead vb-um4oi remains in progress.

## Confirmed P0: source identity is not boundary-safe

compile_source now trusts canonical_digest(source) at crates/vb_compile/src/mod_compile_lowering/part_01.rs:24. The existing canonical encoder in part_05_digest.rs concatenates nested step IDs (Repeat arm), primitive tags and Set output/value strings without field lengths or other unambiguous framing.

Actual public parse_workflow_source and compile_source both succeed for these two sources. Use this workflow and replace the nested pair as shown:

```yaml
version: velvet-ballistics/v1
name: collision
when:
  manual: {}
steps:
  - id: retry
    repeat:
      max_attempts: 3
      steps:
        - id: a
          set:
            output: setb
            value: "1"
  - id: done
    finish:
      result: 0
```

The second source uses nested `id: aset` and `output: b`, otherwise identical. Both feed the same bytes: `a + set + setb + 1 == aset + set + b + 1`. Actual canonical identities, compiled digests and exact WorkflowParts all compare equal. This defeats the intended binding of lowered-away nested identifiers; it is an encoding collision, not a cryptographic collision.

The external Rust probe links the actual final candidate's vb_compile library. Full output and true exit1 are preserved in evidence/current/collision-probe.*. First launcher attempt failed before execution because it found two feature-variant rlibs; collision-launch-failed.* preserves that exit127 separately. The successful probe uses libvb_compile-639c086a7bce7f55.rlib. Its source/binary remain at /tmp/vb-um4oi-collision.rs and /tmp/vb-um4oi-collision.

Required repair: use an unambiguous canonical encoding of semantic fields/variants/collection boundaries, retain formatting/comment equivalence, and add this real public compiler regression. No repair was performed and no assertion was weakened.

## Completed observations, not acceptance

On final376: compiler497 passed; payload3 passed; finish14 passed and1 pre-existing ignored visibility marker; repeat1 passed; artifact25 passed; proof-admission BDD3 passed. SSEI3 passed/1 known failed (vb-6hqsd). The ignored integration marker was not introduced by this candidate and does not cover the reproduced collision. Raw logs/exits are in evidence/current. No full CI, Kani or remaining audit after user STOP. Schema3 migration tests passed; that does not close semantic identity correctness.

No production/test source was edited. Evidence only is preserved. No commands or delegated workers remain running. Tracker remote sync was held by coordination; the new bug is local server state until the designated sync owner publishes it.
