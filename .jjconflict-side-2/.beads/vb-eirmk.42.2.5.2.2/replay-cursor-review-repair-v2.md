# Reviewed replay cursor repair

Preserve input-context contract SHA256 ddd53dd5de73428573dbec15421c84061fef23be0b2e4350c0a4b04f41ea32e5 and frozen rejected candidate44413e607e23928125c2aaf8cef78e572c2eb7e5. This successor supersedes the unaccepted cursor addendum's counter-restoration policy and stale three-case fixture implications, without changing approved input seeding, original validation order, durable slot authority or any public cold oracle.

RCTX-C1: apply only validated successor PC to the hydrated frame; preserve its existing executed count exactly, including journal-hydration executed0. Scratch replay's historical execution count does not become live execution accounting. Persisted retry checkpoint counter preservation remains an independent unchanged contract.

RCTX-C2: carry a private typed replay outcome distinguishing ThroughTarget from BeforeTargetFallback. Only actual through-target replay may supply successor cursor evidence. Before-target nondeterministic fallback must return no cursor while preserving scratch-store reset, seeded roots/taints, recovered-slot fallback, durable override order and later-journal-boundary precedence.

Add bounded independent regressions for a genuinely completed Do target yielding no cursor, a deterministic succeeded target yielding the exact successor PC, and later journal boundary suppressing cursor. Execute the completed-Do case against the unfixed44413 implementation and preserve its actual RED before production repair; rerun all controls GREEN afterward.

Copy the complete current seventeen-case workspace test dependency unchanged from immutable101901350019ce1dc1b5e57deeba5f6994ec46b9, recording exact blobs/hashes and foreign ownership. Preserve both cold registrations, budget1/2 cuts and executed1 oracle, all RAW corrections and every other assertion. No second-reopen/budget-plan requirement is inferred from stale fixtures.

Production repair fence is existing input_context.rs and recovery_seed.rs only unless actual compilation requires a minimal typed private adaptation within the original eleven-path delta; no schema, persisted DTO, public Runtime API, dependency, feature, gate or allowance changes. New bounded tests and this successor contract are separately owned. Every new/changed function must fit25 logical lines including signature; new/changed production files must fit300 lines. Freeze source before affected regressions, three-package library tests, owned regressions, current full17 and strict source gates; independent CODE then TEST remain required.
