# Trusted Base Plan — vb-jpq7.21

STATUS: PLANNED.

Trusted/abstracted surfaces remain limited to postcard/serde internals, shard queue internals below enqueue/no-enqueue observability, compiler-provided `CompiledWorkflow` global validity outside bounded generated seam cases, SlotValue downstream semantic interpretation beyond decode/pass-through/encoded length, and scheduler timing outside the existing pending timer state.

No trusted item waives behavior-affecting AnswerAsk shape, handler decode/default/routing, malformed value rejection-before-mutation, runtime pending Ask derivation, or slot equality semantics.

Repair triggers: if proof-writer introduces unsafe, Miri becomes required; if production-bound Verus specs or Flux annotations/extern specs are added for scoped types, the corresponding not-applicable lane decisions and waiver candidates must be replaced with required obligations.
