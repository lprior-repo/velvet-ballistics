# Native TEST followup for amended projection

STATUS: REJECTED for expanded native overflow-parity coverage. Earlier bounded nonoverflow refusal/replay TEST verdict remains APPROVED. No proof validity verdict.

Frozen followup commit: eb3c2a81cfb615f7d340576b1259383ebcd41fa6, parent ae8ab1a7440cd65e01e52b32b18d5295bd3539e1. Diff contains only verification/verus/production_inner/execute_do_production.rs and verification/verus/vb_runtime_execute_do_spec.rs. Native production SHA256 remains 0e5595ca10b06f72f57a2d794060c1a89ebcf4979cc35d543047cc274f3fcdee; native test SHA256 remains 900344c441d7ec8c315a6a6c6e99fe505df64766c567a64d18426b0b3453be62. Both were remeasured directly from frozen Git blobs. Git/JJ roots remain the assigned review workspace.

## Finding

P2 blocker, crates/vb_runtime/tests/action_resume_contract.rs:97: The amended projection adds the explicit permitted sequence-overflow outcome and its precedence relative to exhaustion/refusal, but native controls only use seq11/attempt1. No existing test exercises resume_action_outcome with seq=u64::MAX. The suite therefore survives substituting RetryExhausted for the reachable InternalInvariantViolation emitted by next_retry_sequence. Add public API cases with seq=u64::MAX: permitted retry below capacity must equal Core(InternalInvariantViolation { reason: seq_overflow_on_retry }); nonpure Unsafe must equal the existing nonretryable refusal before overflow; permitted retry at exhausted capacity must equal RetryExhausted. Assert exact typed outcomes and fields. This is a blocker to the newly expanded overflow-parity evidence, not a claim that this proof-only commit introduced a native runtime regression.

## Review basis

Read the complete amended diff, all calls of resume_action_outcome under crates/vb_runtime, existing engine/action capacity tests, engine/tests/tests_execute.rs resume tests, tests_blackhat.rs resume tests, tests_proptest_blackhat_review.rs resume calls, retry_helpers.rs, and unchanged new action_resume_contract.rs. Repo-wide searches for seq_overflow_on_retry and attempt_overflow_on_retry found only production helper definitions; the large-value idempotency hash tests call a different function and cannot cover retry error outcomes. Amended projection/spec now distinguishes contract refusal, exhausted capacity, then overflow; the new proof_resume_permitted_sequence_overflow_is_invariant_error is at verification/verus/vb_runtime_execute_do_spec.rs:1248. Proof artifacts are being reviewed separately by CODE reviewer.

A native attempt-overflow case below capacity is impossible: attempt and capacity are u16 and attempt=u16::MAX cannot satisfy attempt<capacity. Do not manufacture an impossible native input to match an unconstrained projection Boolean. Exhausted-at-maximum can be used as an additional ordinary capacity control if desired.

No native source changed and no additional execution was needed to establish this missing input coverage; prior executed 3/3 native contract tests remain scoped to their actual inputs. Full MASTER65 mapping and vb-eirmk.60 closure remain outside this approval and blocked by parent decision.
