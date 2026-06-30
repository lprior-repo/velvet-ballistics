# vb-vt2f Domain Model Review

## Domain Terms

- Direct Rust API: in-process public `vb_runtime` facade used for fastest local embedding; master line 53 and line 344 make this mandatory ingress for manual trigger behavior.
- Run: shard-owned runtime execution instance identified by `RunId`.
- Compiled workflow: immutable validated IR accepted by current direct submission or accepted-artifact admission path.
- Suspension: deterministic stop on action, wait, ask, retry, queue, storage policy, cancellation, or shutdown boundary.
- Evidence: observable public output from snapshot, inspect response, trace, journal, counters, typed errors, and nextest scenario result.

## Public Surface Boundary

- In scope: `Runtime::{submit_direct,submit_compiled,tick_all,inspect_run,snapshot_run,take_inspect_response,cancel_run,resume_run,complete_action_with_output,fail_action,answer_ask,timer_fired,list_events,drain_trace,collect_metrics,counters_snapshot,shutdown_graceful}` and public model types from `vb_runtime`/`vb_core` listed in `delivery-scope.jsonl`.
- Out of scope: private shard internals, crate-local fixture helpers, binary IPC, YAML parser/validator/compiler CLI paths, HTTP/JSON, generated Rust parity, Fjall crash recovery.

## Model Risks

- API name drift: master says `Runtime::submit`, current map says `submit_direct`/`submit_compiled`; BDD must bind to current public names and make drift visible.
- Admission policy drift: master describes boolean `RuntimePolicy` fields, current map says enum-like policies; BDD must assert current typed behavior and expose mismatch rather than papering it over.
- Fixture leakage: existing runtime tests may use private helpers; workspace BDD must rebuild fixtures using public constructors only.
- Weak evidence: direct API calls that return `Ok(())` are insufficient; every scenario needs state/trace/journal/counter/snapshot/typed-error assertions.
- Determinism: scenarios must never rely on sleeping, external processes, wall-clock timing, shared global state, IPC, or filesystem side effects unless explicitly isolated.

## Illegal States To Make Observable

- A scenario passes while not driving a public API surface.
- A scenario lacks Given/When/Then metadata or exact assertion evidence.
- A run mutation occurs for the wrong `RunId`/ticket.
- Trace list/drain semantics are asserted only by length without checking run identity or event class.
- Strict/admission-required policy silently accepts legacy raw direct submission.
- Shutdown allows hidden panics or unstated post-shutdown behavior.

## Review Conclusion

The domain model is acceptance-test-first, not production-code-first. The correct abstraction is a scenario catalog row backed by public facade behavior and typed evidence. Formal temporal/pure proofs are non-blocking for this bead only because no runtime semantics should change; if later states must alter runtime/admission code to make scenarios pass, the proof plan must be reopened.

## State 3 Repair Note

- Every PRE/POST/INV/ERR clause now has direct traceability to a proof obligation and traceability row.
- Strict admission lifecycle behavior has a separate TLA waiver path: `WAIVED-TLA-002` / `WAIVER-TLA-VT2F-002` for `POST-012` and `ERR-002`.
- Public-surface review is constrained to public API import/use violations and private runtime-core access, not test helper style.
