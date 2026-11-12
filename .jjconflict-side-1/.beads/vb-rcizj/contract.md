# Effective Rust contract: issued action ticket authority

This document incorporates contract-error-precedence-clarification.md and supersedes contradictory original/addendum draft sentences; those historical artifacts remain retained. No production or proof rule is relaxed.

Public ownership: missingrun=>RunNotFound; full exact pendingticket equality=>proceed to existing preflight; genuineoldticketonlyif0<attempt<pending.attempt, run/step/action/capacitysame, canonicalkeyvalid,seq<=pending.seq=>StaleAttempt{incoming,current}; all otherforged/nopendingticket=>InvalidActionCompletion. All rejection precedes journal/state mutation.

Pure attemptvalidation still rejectsattempt0/capacity0/attempt>capacity=>AttemptBeyondMax, lowerthanrunattempt=>StaleAttempt, future=>InvalidActionCompletion, equalityvalid. These independent helper semantics are tested directly without fabricating impossible publicpendingauthority.

Tests: actualissuedtickets1/2/3, exacterrorvariant+fields, fulljournalandpendingticketequality, PC/bothslotvalues+taintsunchanged, exacthappyActionCompletedEnvelopeincludingticket/value/taint/len/digest. Preserve retryPC/errorhandlerslot requirements and genuineoldstale cases. Do notclaimforcedencoderfailure, fullslot-preservationclosure, allmutations,proofclosure/fullCI/v1. Current25purehelper+28publictargetedtests and5specificguarddeletionmutations are scoped evidence. No unsafe/unwrap/expect/panic/uncheckedoperations authored in newcode.
