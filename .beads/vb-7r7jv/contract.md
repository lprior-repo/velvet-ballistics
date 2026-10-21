# vb-7r7jv contract draft

K1 Strict accepts a side-effecting action only when exact manifest, executor
identity/version, scope, receipt schema, timeout and required duplicate-ticket
conformance are validated against registered execution authority.
K2 Pure actions do not acquire fabricated external conformance claims.
K3 Exact accepted pack digest binds the artifact and numeric runtime executor
selection; callers cannot substitute executors/contracts at dispatch.
K4 Duplicate logical tickets with identical inputs return the same durable receipt
without repeating the adapter's actual effect. Changed input/resource/pack fails.
K5 Crash/reopen reconciliation consumes a durable receipt without live effects.
K6 Non-compensatable/unclassified external actions default to Blocked; invalid or
unsupported recovery classes reject, independently from idempotency class.
K7 Bounded receipt/input/resource capacities reject overflow before effects.
K8 Unknown/old incompatible schemas require explicit migration; no default field
substitution upgrades legacy authority.
K9 Evidence identifies executed adapter observations and raw report bytes; no
hardcoded success digest or caller boolean can construct checked conformance.

This is contract/source exploration only. Root must coordinate the concrete
local-register adapter, trusted conformance publication/resolver, and accepted
action-pack envelope with pending schema3 semantic source identity and the
following scoped grant lane vb-hk1j8. Production changes are intentionally held
at this boundary. Domain decisions are tracked in live bead vb-7r7jv.
