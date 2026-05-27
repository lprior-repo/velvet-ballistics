# Trusted Base Plan — vb-om21

Planner notes for assumptions, bounds, and reductions. This is not approval. Proof-reviewer/formal-verifier must close or reject each trusted surface.

## Planned trusted surfaces
| Ref pattern | Applies to | Trusted kind | Reason | Compensating evidence |
|---|---|---|---|---|
| `TB-vb-om21-*-bounds` | All obligations | model_bound/model_reduction | Proof lanes use finite run/key/sequence edge bounds and scoped helper seams to avoid whole-storage proof explosion. | Kani/proptest/fuzz edge generation plus TLA+ MAX_U64 overflow state and production-bound Verus/Flux obligations. |
| Fjall snapshot consistency | Storage shell obligations | external_dependency | Fjall iterator/snapshot correctness is external to this bead; bead proves local prefix/parser/fold logic around it. | Integration tests and scoped formal obligations must bind to `FjallJournal` seams; no global cache/concurrency behavior admitted. |
| No Restate source | All obligations | external_reference_absent | External Restate file unavailable and no-copy fence applies. | Local contract.md/domain-model.md/source refs are authoritative. |
| Parser malformed bytes | key-parse obligations | hostile_input_bound | Fuzz target excludes Postcard payload decode per model boundary. | Replay parity obligations keep payload validation separate. |

## Forbidden trust
- No `admit`, `assume`, `trusted`, `ignore`, or disabled checks may be introduced without a trusted-base ledger row in downstream states.
- No behavior-affecting waiver candidate is planned.
