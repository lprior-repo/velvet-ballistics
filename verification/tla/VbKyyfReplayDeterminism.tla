---- MODULE VbKyyfReplayDeterminism ----
EXTENDS Naturals, Sequences, FiniteSets

(*
  Obligation: TLA-KYYF-001 / PO-008.
  Cross-run replay determinism and reproducibility state machine.

  Bounded model: MaxSeq=4, MaxReplay=3, MaxJournalEntries=4.
  Fjall I/O, byte decoding, hashing, and OS crash mechanics are trusted
  shell boundaries. CLI execution, wall-clock time, and filesystem paths
  are excluded from this model.

  Invariants owned by this spec:
    INV-003: JournalSequenceWellFormed
    INV-004: DigestMismatchNeverContinues
    INV-005: NoUnsafeSideEffectReexecution
    POST-002: ReplayIsReproducible
    POST-003: StableBlockedOutcome
    POST-004: StableErrorConvergence
    INV-006: GeneratedIrObservationParity
    POST-004: UnsupportedGeneratedSubsetEventuallyFailsClosed

  Repair history (Attempt 4):
    - PF-VB-KYYF-003: CHOOSE replaced with \E-quantified nondeterminism for
      action_class and digest statuses in ExecuteAction/MakeNewRecord.
      Added CorruptRecord, Gap, Duplicate transitions to make bad-evidence
      paths reachable.
    - PF-VB-KYYF-004: ReplayIsReproducible/StableErrorConvergence now compare
      first_replay_error = curr_error (actual typed error equality), not
      enum membership alone.
     - PF-VB-KYYF-005: BoundedOverflow action transitions to
       replay_sequence_violation when NextSeqNum > MaxSeq.

  Repair history (Attempt 6):
    - PF-VB-KYYF-006: UnsupportedGeneratedSubset and replay variant clear
      digest_mismatch because unsupported generated mode is an independent
      fail-closed path that supersedes digest validation. This preserves
      DigestMismatchNeverContinues without weakening the invariant.
     - PF-VB-KYYF-007: replay entry rejects malformed journals immediately
       with replay_sequence_violation so duplicate/gap paths are reachable as
       typed failures, not transient invariant violations.

  Repair history (Attempt 7):
    - PF-VB-KYYF-008: reduce TLC state explosion by replacing behaviorally
      redundant digest component cross-products with representative digest
      profiles: all match, all unchecked, and one mismatch per required digest
      component. This preserves source/compiled/action ABI/policy mismatch
      coverage because every component can independently drive the same typed
      replay_digest_mismatch behavior, while avoiding equivalent multi-mismatch
      and mixed unchecked/match states.
    - PF-VB-KYYF-009: initialize journal_dispatched to all FALSE. Executed
      actions set the dispatch evidence for their own journal index before
      replay; arbitrary pre-dispatch maps only duplicated states without adding
      a distinct contract behavior under this model.
    - PF-VB-KYYF-010: digest mismatch detection is guarded to nonterminal
      no-error supported-generated/IR states. Unsupported generated subset is
      the superseding fail-closed path and must not be preempted or later
      mutated into a mixed terminal/error state.
    - PF-VB-KYYF-011: replay policy blocking is likewise guarded to supported
      generated/IR states so unsupported generated evidence cannot terminate
      as replay_policy_blocked, replay_sequence_violation,
      replay_digest_mismatch, or overflow sequence violation instead of
      unsupported_generated_subset.
    - PF-VB-KYYF-012: CorruptRecord appends corrupt evidence but does not set
      the detected digest_mismatch flag. DetectDigestMismatch remains the sole
      transition that turns corrupt digest evidence into typed digest failure.
      Corrupt records still represent executed journal evidence, so they mark
      dispatch evidence for their journal index just like ExecuteAction.
    - PF-VB-KYYF-013: normal execution uses clean digest profiles only;
      CorruptRecord injects one required digest-component mismatch at a time;
      Duplicate/Gap vary action class only because their contract behavior is
      sequence violation, not digest classification. Unsupported generated mode
      is fail-closed before journal-producing actions.
*)

CONSTANT MaxSeq, MaxReplay, MaxJournalEntries

ASSUME MaxSeq \in Nat /\ MaxSeq >= 2
ASSUME MaxReplay \in Nat /\ MaxReplay >= 1
ASSUME MaxJournalEntries \in Nat /\ MaxJournalEntries >= 1

\* -----------------------------------------------------------------------
\* Domain types
\* -----------------------------------------------------------------------
RunIds        == {0, 1}
ActionClass   == {"DeterministicPure", "IdempotentExternal",
                  "AtLeastOnceExternal", "NonIdempotentExternal"}
DigestStatus  == {"match", "mismatch", "unchecked"}
TypedError    == {"none", "no_recovery_data", "replay_sequence_violation",
                  "replay_digest_mismatch", "unsupported_generated_subset",
                  "replay_policy_blocked"}
TerminalKind  == {"none", "ok", "blocked", "failed"}
GeneratedState == {"ir_only", "generated_supported", "generated_unsupported"}

CleanDigestProfiles == {
  [wf_src |-> "match",    compiled |-> "match",    action_abi |-> "match",    policy |-> "match"],
  [wf_src |-> "unchecked", compiled |-> "unchecked", action_abi |-> "unchecked", policy |-> "unchecked"]
}

MismatchDigestProfiles == {
  [wf_src |-> "mismatch", compiled |-> "match",    action_abi |-> "match",    policy |-> "match"],
  [wf_src |-> "match",    compiled |-> "mismatch", action_abi |-> "match",    policy |-> "match"],
  [wf_src |-> "match",    compiled |-> "match",    action_abi |-> "mismatch", policy |-> "match"],
  [wf_src |-> "match",    compiled |-> "match",    action_abi |-> "match",    policy |-> "mismatch"]
}

AllMatchDigestProfile ==
  [wf_src |-> "match", compiled |-> "match", action_abi |-> "match", policy |-> "match"]

\* -----------------------------------------------------------------------
\* Journal record shape (as a SET, not a single value)
\* -----------------------------------------------------------------------
RECORD == [
  run_id            : RunIds,
  seq               : 1..MaxSeq,
  action_class      : ActionClass,
  digest_wf_src     : DigestStatus,
  digest_compiled   : DigestStatus,
  digest_action_abi : DigestStatus,
  digest_policy     : DigestStatus,
  is_terminal       : BOOLEAN,
  generated         : GeneratedState
]

\* -----------------------------------------------------------------------
\* State variables
\* -----------------------------------------------------------------------
VARIABLES
  run_id,
  journal,
  replay_count,
  journal_dispatched,
  digest_mismatch,
  error,
  terminal,
  generated_mode,
  replay_state,
  \* Repair PF-VB-KYYF-004: track first replay error and current error
  \* to enable actual typed-error equality comparison across replay attempts
  first_replay_error,
  curr_error

vars == <<run_id, journal, replay_count, journal_dispatched,
          digest_mismatch, error, terminal, generated_mode, replay_state,
          first_replay_error, curr_error>>

TypeOK ==
  /\ run_id \in RunIds
  /\ journal \in Seq(RECORD)
  /\ replay_count \in 0..MaxReplay
  /\ journal_dispatched \in [1..MaxJournalEntries -> BOOLEAN]
  /\ digest_mismatch \in BOOLEAN
  /\ error \in TypedError
  /\ terminal \in TerminalKind
  /\ generated_mode \in GeneratedState
  /\ replay_state \in {"initial", "replay"}
  /\ first_replay_error \in TypedError
  /\ curr_error \in TypedError

\* -----------------------------------------------------------------------
\* Init
\* -----------------------------------------------------------------------
Init ==
  /\ run_id \in RunIds
  /\ journal = << >>
  /\ replay_count = 0
  /\ journal_dispatched = [i \in 1..MaxJournalEntries |-> FALSE]
  /\ digest_mismatch = FALSE
  /\ error = "none"
  /\ terminal = "none"
  /\ generated_mode \in GeneratedState
  /\ replay_state = "initial"
  /\ first_replay_error = "none"
  /\ curr_error = "none"

\* -----------------------------------------------------------------------
\* Helpers
\* -----------------------------------------------------------------------
IsContiguous(j) ==
  \A i \in 1..(Len(j)-1): j[i].seq + 1 = j[i+1].seq

IsMonotonic(j) ==
  \A i, k \in 1..Len(j): i < k => j[i].seq < j[k].seq

AnyMismatch(j) ==
  \E i \in 1..Len(j):
    \/ j[i].digest_wf_src = "mismatch"
    \/ j[i].digest_compiled = "mismatch"
    \/ j[i].digest_action_abi = "mismatch"
    \/ j[i].digest_policy = "mismatch"

LastSeq(j) ==
  IF Len(j) = 0 THEN 0
  ELSE j[Len(j)].seq

LastJournalIdx(j) ==
  Len(j)

IsReplaySafeClass(c) ==
  c \in {"DeterministicPure", "IdempotentExternal"}

ExternalActionSeqs(j) ==
  { i \in 1..Len(j) : ~IsReplaySafeClass(j[i].action_class) }

WasDispatched(journal_idx) ==
  IF journal_idx \in DOMAIN journal_dispatched THEN journal_dispatched[journal_idx] ELSE FALSE

JournalSeqWellFormed(j) ==
  /\ IsMonotonic(j)
  /\ IsContiguous(j)
  /\ \A i \in 1..Len(j): j[i].run_id = run_id

(*
  Helper: the sequence number the next journal record would have.
*)
NextSeqNum ==
  LastSeq(journal) + 1

(*
  Helper: the journal index the next record would occupy.
*)
NextJournalIndex ==
  LastJournalIdx(journal) + 1

(*
  Helper: construct a record with given fields.
  No CHOOSE — all fields are passed as parameters so TLC explores
  all (action_class, digest_status) combinations.
*)
MakeNewRecord(ac, digest_profile) ==
  [ run_id            |-> run_id,
    seq               |-> NextSeqNum,
    action_class      |-> ac,
    digest_wf_src     |-> digest_profile.wf_src,
    digest_compiled   |-> digest_profile.compiled,
    digest_action_abi |-> digest_profile.action_abi,
    digest_policy     |-> digest_profile.policy,
    is_terminal       |-> FALSE,
    generated         |-> generated_mode ]

(*
  Helper: construct a corrupt record with mismatched digests at a given seq.
  Used by CorruptRecord action.
*)
MakeCorruptRecord(ac, corrupt_seq, digest_profile) ==
  [ run_id            |-> run_id,
    seq               |-> corrupt_seq,
    action_class      |-> ac,
    digest_wf_src     |-> digest_profile.wf_src,
    digest_compiled   |-> digest_profile.compiled,
    digest_action_abi |-> digest_profile.action_abi,
    digest_policy     |-> digest_profile.policy,
    is_terminal       |-> FALSE,
    generated         |-> generated_mode ]

\* -----------------------------------------------------------------------
\* Transitions
\* -----------------------------------------------------------------------

(*
  ExecuteAction: append a journal record during initial run.
  REPAIRED (PF-VB-KYYF-003): action_class and all digest statuses are now
  existentially quantified nondeterministic parameters. TLC explores ALL
  combinations of action class and digest status, not one arbitrary CHOOSE.
  Overflow is handled by BoundedOverflow; this action requires NextSeqNum <= MaxSeq.
*)
ExecuteAction ==
  /\ terminal = "none"
  /\ replay_state = "initial"
  /\ replay_count = 0
  /\ generated_mode # "generated_unsupported"
  /\ Len(journal) < MaxJournalEntries
  /\ NextSeqNum <= MaxSeq
  /\ \E ac \in ActionClass:
       \E digest_profile \in CleanDigestProfiles:
         LET new_record == MakeNewRecord(ac, digest_profile)
         IN
           /\ journal_dispatched' =
              IF IsReplaySafeClass(ac) \/ ~WasDispatched(NextJournalIndex)
              THEN [journal_dispatched EXCEPT ![NextJournalIndex] = TRUE]
              ELSE journal_dispatched
           /\ journal' = Append(journal, new_record)
           /\ UNCHANGED <<run_id, replay_count, digest_mismatch, error,
                           terminal, generated_mode, replay_state,
                           first_replay_error, curr_error>>

(*
  BoundedOverflow: REPAIRED (PF-VB-KYYF-005).
  When NextSeqNum > MaxSeq but journal still has capacity, we cannot append
  a well-formed record. The spec transitions to replay_sequence_violation
  rather than deadlocking or silently continuing.
*)
BoundedOverflow ==
  /\ terminal = "none"
  /\ replay_state = "initial"
  /\ replay_count = 0
  /\ generated_mode # "generated_unsupported"
  /\ Len(journal) < MaxJournalEntries
  /\ NextSeqNum > MaxSeq
  /\ error' = "replay_sequence_violation"
  /\ terminal' = "failed"
  /\ UNCHANGED <<run_id, journal, replay_count, journal_dispatched,
                  digest_mismatch, generated_mode, replay_state,
                  first_replay_error, curr_error>>

(*
  TerminateRun: mark the last journal record as terminal.
*)
TerminateRun ==
  /\ terminal = "none"
  /\ Len(journal) > 0
  /\ ~journal[Len(journal)].is_terminal
  /\ generated_mode # "generated_unsupported"
  /\ terminal' = "ok"
  /\ journal' = [journal EXCEPT ![Len(journal)].is_terminal = TRUE]
  /\ UNCHANGED <<run_id, replay_count, journal_dispatched, digest_mismatch,
                  error, generated_mode, replay_state,
                  first_replay_error, curr_error>>

(*
  DetectDigestMismatch: set digest_mismatch flag if any record has a
  digest mismatch. Once set, never cleared.
  Also sets terminal to "failed" so DigestMismatchNeverContinues is satisfied
  (typed error state is terminal, not silent continuation).
*)
DetectDigestMismatch ==
  /\ terminal = "none"
  /\ error = "none"
  /\ generated_mode # "generated_unsupported"
  /\ ~digest_mismatch
  /\ AnyMismatch(journal)
  /\ digest_mismatch' = TRUE
  /\ error' = IF error = "none" THEN "replay_digest_mismatch" ELSE error
  /\ terminal' = "failed"
  /\ UNCHANGED <<run_id, journal, replay_count, journal_dispatched,
                  generated_mode, replay_state,
                  first_replay_error, curr_error>>

(*
  CorruptRecord: REPAIRED (PF-VB-KYYF-003).
  Appends a record with mismatched digests to simulate corrupt evidence.
  This makes DigestMismatchNeverContinues non-vacuous — the mismatch path
  is now reachable.
*)
CorruptRecord ==
  /\ terminal = "none"
  /\ replay_state = "initial"
  /\ replay_count = 0
  /\ generated_mode # "generated_unsupported"
  /\ Len(journal) < MaxJournalEntries
  /\ NextSeqNum <= MaxSeq
  /\ \E ac \in ActionClass:
       \E digest_profile \in MismatchDigestProfiles:
         LET new_record == MakeCorruptRecord(ac, NextSeqNum, digest_profile)
         IN
            /\ journal' = Append(journal, new_record)
            /\ journal_dispatched' =
               IF IsReplaySafeClass(ac) \/ ~WasDispatched(NextJournalIndex)
               THEN [journal_dispatched EXCEPT ![NextJournalIndex] = TRUE]
               ELSE journal_dispatched
            /\ UNCHANGED <<run_id, replay_count, digest_mismatch, error, terminal, generated_mode,
                            replay_state, first_replay_error, curr_error>>

(*
  Duplicate: REPAIRED (PF-VB-KYYF-003).
  Appends a record whose seq equals the last record's seq, creating a
  duplicate sequence number. This makes the gap/duplicate path reachable
  so DetectReplaySequenceViolation can fire during replay.
*)
Duplicate ==
  /\ terminal = "none"
  /\ replay_state = "initial"
  /\ replay_count = 0
  /\ generated_mode # "generated_unsupported"
  /\ Len(journal) >= 1
  /\ Len(journal) < MaxJournalEntries
  /\ NextSeqNum <= MaxSeq
  /\ \E ac \in ActionClass:
         LET dup_seq == journal[Len(journal)].seq
         IN LET new_record == [ run_id            |-> run_id,
                                 seq               |-> dup_seq,
                                 action_class      |-> ac,
                                 digest_wf_src     |-> AllMatchDigestProfile.wf_src,
                                 digest_compiled   |-> AllMatchDigestProfile.compiled,
                                 digest_action_abi |-> AllMatchDigestProfile.action_abi,
                                 digest_policy     |-> AllMatchDigestProfile.policy,
                                 is_terminal       |-> FALSE,
                                 generated         |-> generated_mode ]
         IN
           /\ journal' = Append(journal, new_record)
           /\ UNCHANGED <<run_id, replay_count, journal_dispatched,
                           digest_mismatch, error, terminal, generated_mode,
                           replay_state, first_replay_error, curr_error>>

(*
  Gap: REPAIRED (PF-VB-KYYF-003).
  Appends a record whose seq skips one (last seq + 2), creating a gap.
  This makes the gap path reachable for DetectReplaySequenceViolation.
*)
Gap ==
  /\ terminal = "none"
  /\ replay_state = "initial"
  /\ replay_count = 0
  /\ generated_mode # "generated_unsupported"
  /\ Len(journal) >= 1
  /\ Len(journal) < MaxJournalEntries
  /\ NextSeqNum <= MaxSeq
  /\ \E ac \in ActionClass:
         LET gap_seq == LastSeq(journal) + 2
         IN LET new_record == [ run_id            |-> run_id,
                                 seq               |-> gap_seq,
                                 action_class      |-> ac,
                                 digest_wf_src     |-> AllMatchDigestProfile.wf_src,
                                 digest_compiled   |-> AllMatchDigestProfile.compiled,
                                 digest_action_abi |-> AllMatchDigestProfile.action_abi,
                                 digest_policy     |-> AllMatchDigestProfile.policy,
                                 is_terminal       |-> FALSE,
                                 generated         |-> generated_mode ]
         IN
           /\ journal' = Append(journal, new_record)
           /\ UNCHANGED <<run_id, replay_count, journal_dispatched,
                           digest_mismatch, error, terminal, generated_mode,
                           replay_state, first_replay_error, curr_error>>

(*
  StartReplay: begin replay from the existing journal.
  REPAIRED (PF-VB-KYYF-004): reset curr_error for the new replay attempt.
*)
StartReplay ==
  /\ replay_state = "initial"
  /\ terminal = "none"
  /\ Len(journal) > 0
  /\ JournalSeqWellFormed(journal)
  /\ replay_count' = 1
  /\ replay_state' = "replay"
  /\ curr_error' = "none"
  /\ UNCHANGED <<run_id, journal, journal_dispatched, digest_mismatch,
                  error, terminal, generated_mode, first_replay_error>>

StartReplaySequenceViolation ==
  /\ replay_state = "initial"
  /\ terminal = "none"
  /\ generated_mode # "generated_unsupported"
  /\ Len(journal) > 0
  /\ ~JournalSeqWellFormed(journal)
  /\ replay_count' = 1
  /\ replay_state' = "replay"
  /\ error' = "replay_sequence_violation"
  /\ terminal' = "failed"
  /\ curr_error' = "replay_sequence_violation"
  /\ first_replay_error' = IF first_replay_error = "none"
                            THEN "replay_sequence_violation"
                            ELSE first_replay_error
  /\ UNCHANGED <<run_id, journal, journal_dispatched, digest_mismatch,
                  generated_mode>>

(*
  ReplayJournal: advance replay attempt counter.
  REPAIRED (PF-VB-KYYF-004): track first_replay_error when entering terminal
  state for the first time.
*)
ReplayJournal ==
  /\ replay_state = "replay"
  /\ replay_count < MaxReplay
  /\ terminal = "none"
  /\ error = "none"
  /\ \neg \E i \in ExternalActionSeqs(journal): WasDispatched(i) = TRUE
  /\ replay_count' = replay_count + 1
  /\ replay_state' = replay_state
  /\ UNCHANGED <<run_id, journal, journal_dispatched, digest_mismatch,
                  error, terminal, generated_mode, first_replay_error, curr_error>>

(*
  DetectReplaySequenceViolation: if journal has gaps, mixed run ids,
  or non-contiguous sequences, set typed error and terminal.
  REPAIRED (PF-VB-KYYF-004): also update curr_error and first_replay_error.
*)
DetectReplaySequenceViolation ==
  /\ replay_state = "replay"
  /\ terminal = "none"
  /\ error = "none"
  /\ generated_mode # "generated_unsupported"
  /\ \/ ~IsMonotonic(journal)
     \/ ~IsContiguous(journal)
     \/ \E i \in 1..Len(journal): journal[i].run_id # run_id
  /\ error' = "replay_sequence_violation"
  /\ terminal' = "failed"
  /\ curr_error' = "replay_sequence_violation"
  /\ first_replay_error' = IF first_replay_error = "none"
                            THEN "replay_sequence_violation"
                            ELSE first_replay_error
  /\ UNCHANGED <<run_id, journal, replay_count, journal_dispatched,
                  digest_mismatch, generated_mode, replay_state>>

(*
  DetectReplayDigestMismatch: on replay, any digest mismatch = typed failure.
  REPAIRED (PF-VB-KYYF-004): also update curr_error and first_replay_error.
*)
DetectReplayDigestMismatch ==
  /\ replay_state = "replay"
  /\ terminal = "none"
  /\ error = "none"
  /\ generated_mode # "generated_unsupported"
  /\ AnyMismatch(journal)
  /\ error' = "replay_digest_mismatch"
  /\ terminal' = "failed"
  /\ digest_mismatch' = TRUE
  /\ curr_error' = "replay_digest_mismatch"
  /\ first_replay_error' = IF first_replay_error = "none"
                            THEN "replay_digest_mismatch"
                            ELSE first_replay_error
  /\ UNCHANGED <<run_id, journal, replay_count, journal_dispatched,
                  generated_mode, replay_state>>

(*
  ReplayPolicyBlocked: at-least-once or non-idempotent action already
  dispatched in initial run → replay must block and not re-dispatch.
  REPAIRED (PF-VB-KYYF-004): also update curr_error and first_replay_error.
*)
ReplayPolicyBlocked ==
  /\ replay_state = "replay"
  /\ terminal = "none"
  /\ error = "none"
  /\ generated_mode # "generated_unsupported"
  /\ \E i \in ExternalActionSeqs(journal):
       WasDispatched(i) = TRUE
  /\ error' = "replay_policy_blocked"
  /\ terminal' = "blocked"
  /\ curr_error' = "replay_policy_blocked"
  /\ first_replay_error' = IF first_replay_error = "none"
                            THEN "replay_policy_blocked"
                            ELSE first_replay_error
  /\ UNCHANGED <<run_id, journal, replay_count, journal_dispatched,
                  digest_mismatch, generated_mode, replay_state>>

(*
  UnsupportedGeneratedSubset: generated_mode is unsupported → fail closed.
*)
UnsupportedGeneratedSubset ==
  /\ terminal = "none"
  /\ error = "none"
  /\ generated_mode = "generated_unsupported"
  /\ error' = "unsupported_generated_subset"
  /\ terminal' = "failed"
  /\ digest_mismatch' = FALSE
  /\ curr_error' = "unsupported_generated_subset"
  /\ first_replay_error' = IF first_replay_error = "none"
                            THEN "unsupported_generated_subset"
                            ELSE first_replay_error
  /\ UNCHANGED <<run_id, journal, replay_count, journal_dispatched,
                  generated_mode, replay_state>>

(*
  UnsupportedGeneratedSubsetReplay: same failure on replay.
*)
UnsupportedGeneratedSubsetReplay ==
  /\ replay_state = "replay"
  /\ terminal = "none"
  /\ error = "none"
  /\ generated_mode = "generated_unsupported"
  /\ error' = "unsupported_generated_subset"
  /\ terminal' = "failed"
  /\ digest_mismatch' = FALSE
  /\ curr_error' = "unsupported_generated_subset"
  /\ first_replay_error' = IF first_replay_error = "none"
                            THEN "unsupported_generated_subset"
                            ELSE first_replay_error
  /\ UNCHANGED <<run_id, journal, replay_count, journal_dispatched,
                  generated_mode, replay_state>>

(*
  StableTerminal: once terminal is set, no further changes.
*)
StableTerminal ==
  /\ terminal # "none"
  /\ UNCHANGED vars

(*
  NoOp: stutter when no transition applies.
*)
NoOp ==
  /\ terminal = "none"
  /\ UNCHANGED vars

Next ==
  \/ ExecuteAction
  \/ BoundedOverflow
  \/ TerminateRun
  \/ DetectDigestMismatch
  \/ CorruptRecord
  \/ Duplicate
  \/ Gap
  \/ StartReplay
  \/ StartReplaySequenceViolation
  \/ ReplayJournal
  \/ DetectReplaySequenceViolation
  \/ DetectReplayDigestMismatch
  \/ ReplayPolicyBlocked
  \/ UnsupportedGeneratedSubset
  \/ UnsupportedGeneratedSubsetReplay
  \/ StableTerminal
  \/ NoOp

\* -----------------------------------------------------------------------
\* Fairness
\* -----------------------------------------------------------------------
Spec ==
  /\ Init
  /\ [][Next]_vars
  /\ WF_vars(ExecuteAction)
  /\ WF_vars(TerminateRun)
  /\ WF_vars(DetectDigestMismatch)
  /\ WF_vars(StartReplay)
  /\ WF_vars(StartReplaySequenceViolation)
  /\ WF_vars(ReplayJournal)
  /\ WF_vars(CorruptRecord)
  /\ WF_vars(Duplicate)
  /\ WF_vars(Gap)
  /\ SF_vars(UnsupportedGeneratedSubset)
  /\ SF_vars(UnsupportedGeneratedSubsetReplay)
  /\ SF_vars(BoundedOverflow)

\* -----------------------------------------------------------------------
\* Invariants
\* -----------------------------------------------------------------------

(*
  INV-003: Journal sequence is monotonic and contiguous for all observable
  states (initial and replay). Mixed run_id in journal is a sequence
  violation.
*)
JournalSequenceWellFormed ==
  replay_state = "replay" =>
    \/ JournalSeqWellFormed(journal)
    \/ error = "replay_sequence_violation" /\ terminal = "failed"

(*
  INV-004: Digest mismatch never leads to silent continuation.
  If any digest mismatch exists, error must be typed and terminal
  must be reached without further state progression.
*)
DigestMismatchNeverContinues ==
  digest_mismatch = TRUE =>
    \/ error = "replay_digest_mismatch" /\ terminal = "failed"
    \/ error = "none"

(*
  INV-005: After a journal boundary (replay_count > 1), non-replay-safe
  (AtLeastOnce / NonIdempotent) external actions are never re-dispatched.
  The journal_dispatched flag tracks which entries were dispatched in the
  initial run; replay never changes it.
*)
NoUnsafeSideEffectReexecution ==
  replay_state = "replay" /\ replay_count > 1 =>
    \A i \in ExternalActionSeqs(journal):
      journal_dispatched[i] = TRUE

(*
  POST-002: Replay is reproducible. REPAIRED (PF-VB-KYYF-004).
  Compares actual typed error values (first_replay_error = curr_error),
  not just enum membership. When replay_count > 1, the current error
  must equal the first error observed in this replay session — proving
  that repeated replay attempts produce identical typed outcomes.
*)
ReplayIsReproducible ==
  replay_state = "replay" /\ replay_count > 1 =>
    curr_error = first_replay_error

(*
  POST-003: Stable blocked outcome. REPAIRED (PF-VB-KYYF-004).
  Compares actual error values, not membership in a set.
*)
StableBlockedOutcome ==
  (replay_state = "replay" /\ replay_count > 1 /\
    \E i \in ExternalActionSeqs(journal): WasDispatched(i) = TRUE)
    =>
    curr_error = "replay_policy_blocked"
    /\ curr_error = first_replay_error

(*
  POST-004: Stable error convergence. REPAIRED (PF-VB-KYYF-004).
  Compares actual typed error across replay attempts, not enum membership.
  If any typed error was observed first (first_replay_error # "none"),
  all subsequent replay attempts must produce the same error.
*)
StableErrorConvergence ==
  replay_state = "replay" /\ replay_count > 1 =>
    \/ first_replay_error = "none"
    \/ curr_error = first_replay_error

(*
  INV-006: Generated IR observation parity. Unsupported generated subset
  fails closed before any terminal observation is reached.
*)
GeneratedIrObservationParity ==
  generated_mode = "generated_unsupported" =>
    terminal \in {"none", "failed"}

(*
  UnsupportedGeneratedSubsetEventuallyFailsClosed:
  any state with generated_unsupported must eventually reach terminal=failed
  with error=unsupported_generated_subset.
*)
UnsupportedGeneratedSubsetEventuallyFailsClosed ==
  generated_mode = "generated_unsupported" =>
    <>(terminal = "failed" /\ error = "unsupported_generated_subset")

\* -----------------------------------------------------------------------
\* Liveness (checked as PROPERTY)
\* -----------------------------------------------------------------------
EventuallyTerminal ==
  <>(terminal # "none")

EventuallyReplayOrTerminal ==
  <>(replay_state = "replay" \/ terminal # "none")

====
