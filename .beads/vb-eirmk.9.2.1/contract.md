# Contract: refuse ambiguous immediate-retry recovery

## Scope

Containment for an actual Strict admitted action run whose durable prefix resolves a retry-capable zero-delay Do failure before recording the selected retry or handler outcome. Baseline `f24a7401974a84259e4507cf5204c0b520ade1b9`. Full immediate-retry preservation remains blocked under `vb-eirmk.9.2`; this child makes no preservation claim. Timed Wait/Ask wheel and completed-action/RunResumed continuation are outside scope.

## Domain and evidence

The actual failure producer records `ActionFailedEvent.attempt` from `ticket.attempt`, the action attempt; the runtime journal converter preserves it unchanged. The prior matching issuer-created `ActionScheduledTicket` also carries the distinct `ticket.run_attempt` epoch. Retry eligibility is loaded from explicit compiled Do retry policy and the matching full ticket; legacy dynamic RetryCheck capacity is never guessed. The original action ticket, accepted artifact, retained receipt, policy, run attempt, ABI, and sequence remain production-validated authority. Failure events omit failure code and selected outcome; an available handler alone cannot recover a selected decision. RunResumed is metadata and supplies no missing action-decision authority.

## Required behavior

- Common public `Runtime::recover_product` and its `recover_and_resume` caller return existing `RuntimeError::RecoveryCannotResume { reason: "immediate_retry_decision_missing" }` when an explicit zero-delay retry-capable Do fails before exhaustion and the durable suffix has no selected decision authority.
- Apply the check after validated delayed retry recovery and before generic or owned hydration/enqueue, for both handler and no-handler source workflows, with or without actual durable RunResumed metadata.
- The refusal causes no run/frame/pending-ticket/counter/trace/enqueue, journal, receipt, or declared-keyspace mutation, checked across complete drop/reopen.
- Preserve ordinary nonretry handlers, positive-delay checkpoint authority, terminal run outcomes, exhausted action attempts, and genuinely later journaled handler entry. Handler exemption requires first non-RunResumed suffix event to be StepStarted at the exact production-resolved handler; normal replay validates remaining evidence.
- Do not treat an ActionScheduledTicket after failure alone as valid immediate-retry authority; current production replay checks still decide that case.
- An earlier failure must not block an authorized terminal run retry. Current-attempt evidence and the latest matched issued schedule constrain refusal.

## Boundaries and hazards

The guard is a pure crate-private calculation over loaded journal records and compiled workflow; admission and effect boundaries remain unchanged. False refusal hazards are run/action attempt confusion, exhausted attempts, earlier failures from a prior run attempt, dynamic retry metadata, and accepted handler progression. False acceptance hazards are treating Resumed as a decision, trusting a copied handler state, or accepting unvalidated later action schedules. Existing typed refusal avoids new error variants, persisted formats, and trust expansion.

## Delivery and validation

Register actual source/artifact/admission/ticket Strict fault-cut tests before implementation and capture native red. Verify exact typed refusal plus complete cold keyspace identity and live-state observations. Add preservation controls for nonretry and exhausted handlers, delayed authority, and terminal outcomes as needed beyond existing suites. Run pinned nightly bounded nextest (two Cargo jobs, four threads), source lint, formatting, and exact diff review. Independent CODE and TEST reviews are coordinated by parent. No new dependencies, APIs, schema/format, gate/config, suppression, skipped cases, GPU/Python/tmp, broad CI, main push, or child agents.
