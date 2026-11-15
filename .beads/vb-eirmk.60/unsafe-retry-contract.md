# Bounded unsafe retry repair

Given a failed action outcome, action resumption must consult the supplied action contract before issuing another action ticket. A side-effecting action with `RetrySafety::Unsafe` must return the existing exact `RuntimeEngineError::Core(EngineError::UnsupportedPrimitive { primitive: "action_failed_non_retryable" })` outcome, even when the failure requests retry and ticket capacity remains.

Existing `RetrySafety::Safe` and `RetrySafety::KeyRequired` retry behavior remains unchanged. `SideEffect::None` retains the existing pure-action bypass for every current retry-safety class. A nonretryable failure still refuses retry for every classification, and capacity exhaustion retains its existing typed outcome for permitted retry classifications.

This repair does not change enum discriminants, ticket identity or key computation, persisted formats, compiler admission, canonical taxonomy, or cold recovery. Key-required admission remains a separate obligation. Verification uses the public `resume_action_outcome` API with explicit expected refusal and retry outcomes across every current side-effect class and retry-safety class, including refusal precedence over exhausted capacity.
