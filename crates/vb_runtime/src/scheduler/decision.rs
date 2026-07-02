#![forbid(unsafe_code)]
//! Boundary decision logic for [`SeededScheduler`].
//!
//! This file is part of the `scheduler` module. It contains the
//! public scheduler API: `tick_shard`, `tick_all`, `decide_boundary`,
//! and `run_to_completion`. The free-function helpers that select
//! boundary variants (e.g. `select_decision`, `materialize_free_variant`)
//! and translate [`BoundaryDecision`] → [`ShardDirective`] live in
//! the sibling `decision_select.rs` to keep this file under the
//! 300-line production ceiling.
//!
//! All control flow is statically bounded by the configured
//! `SchedulerConfig::max_steps` and `max_ticks` budgets.
//!
//! # Runtime wiring
//!
//! `tick_shard` translates each [`BoundaryDecision`] into a
//! [`ShardDirective`] and dispatches it to
//! `Runtime::tick_shard`; `tick_all` calls `Runtime::tick_all` and
//! observes its `Ok(false)` return value (which indicates that at
//! least one shard has shut down). `run_to_completion` advances the
//! runtime via `tick_all` until the runtime reports natural
//! completion or a budget trips.

use crate::scheduler::config::SeededScheduler;
use crate::scheduler::decision_select::{
    TickOutcome, select_decision, translate_decision_to_directive,
};
use crate::scheduler::error::SchedulerError;
use crate::scheduler::transcript::BoundaryTranscriptEntry;
use crate::scheduler::types::{
    BoundaryChoice, BoundaryDecision, BoundaryPolicy, RunEndReason, RunResult,
};

impl SeededScheduler {
    /// Ticks the runtime on the given shard and returns a
    /// [`BoundaryDecision`].
    ///
    /// Enforces the `max_steps` budget; returns
    /// [`SchedulerError::StepBudgetExhausted`] if the budget is reached.
    /// Enforces the shard-index invariant; returns
    /// [`SchedulerError::ShardOutOfRange`] if `shard_idx >= shard_count`.
    ///
    /// Translates the chosen [`BoundaryDecision`] into a
    /// [`ShardDirective`] and dispatches it to
    /// `Runtime::tick_shard`. The directive mapping is:
    ///
    /// - [`BoundaryDecision::Advance`] → `ShardDirective::Continue`
    /// - [`BoundaryDecision::Yield`] → `ShardDirective::Migrate`
    ///   (the `to_step` index is reduced modulo `shard_count` to
    ///   produce a valid in-range target; if it collapses to the
    ///   source, the next index in cyclic order is used instead)
    /// - [`BoundaryDecision::Fail`] → `ShardDirective::Shutdown`
    /// - [`BoundaryDecision::Retry`] → `ShardDirective::Suspend`
    pub fn tick_shard(
        &mut self,
        shard_idx: u32,
        choice: BoundaryChoice,
    ) -> Result<BoundaryDecision, SchedulerError> {
        self.enforce_step_budget()?;
        if shard_idx >= self.shard_count {
            return Err(SchedulerError::ShardOutOfRange {
                requested: shard_idx,
                shard_count: self.shard_count,
            });
        }
        let decision = self.decide_boundary(choice)?;
        let directive = translate_decision_to_directive(&decision, shard_idx, self.shard_count);
        self.runtime
            .tick_shard(shard_idx, directive)
            .map_err(|runtime_err| SchedulerError::Runtime {
                code: runtime_err.runtime_code().unwrap_or("unknown"),
                detail: runtime_err.to_string(),
            })?;
        // Bump step counter exactly once per tick, after the decision
        // is recorded AND the runtime has accepted the directive, so
        // a `decide_boundary` failure or runtime failure does not
        // silently consume a step.
        self.step_count =
            self.step_count
                .checked_add(1)
                .ok_or(SchedulerError::StepBudgetExhausted {
                    budget: self.max_steps,
                })?;
        Ok(decision)
    }

    /// Ticks the runtime on every shard in shard-index order and
    /// returns a single aggregated [`BoundaryDecision`].
    ///
    /// Enforces the `max_steps` budget. Calls `Runtime::tick_all`
    /// and observes its return: `Ok(false)` means at least one shard
    /// has shut down, which is propagated up via
    /// [`BoundaryDecision::Fail`] with
    /// [`crate::RuntimeError::ShutdownInProgress`] so that
    /// `run_to_completion` can detect natural completion via the
    /// failed-decision path.
    pub fn tick_all(&mut self) -> Result<BoundaryDecision, SchedulerError> {
        self.enforce_step_budget()?;
        // Use a transient decision: advance is the natural default
        // for batch ticks; the caller can override via
        // `decide_boundary` after observing the result.
        let choice = BoundaryChoice::Free;
        let decision = self.decide_boundary(choice)?;
        let all_alive = self
            .runtime
            .tick_all()
            .map_err(|runtime_err| SchedulerError::Runtime {
                code: runtime_err.runtime_code().unwrap_or("unknown"),
                detail: runtime_err.to_string(),
            })?;
        // Bump step counter exactly once per tick, after the runtime
        // has accepted the tick, so a runtime failure does not
        // silently consume a step.
        self.step_count =
            self.step_count
                .checked_add(1)
                .ok_or(SchedulerError::StepBudgetExhausted {
                    budget: self.max_steps,
                })?;
        if !all_alive {
            // At least one shard has shut down. Surface this as a
            // Fail decision so callers (e.g. tests) can observe the
            // completion signal via the typed return value.
            return Ok(BoundaryDecision::Fail {
                variant: crate::RuntimeError::ShutdownInProgress,
            });
        }
        Ok(decision)
    }

    /// Returns a [`BoundaryDecision`] for the supplied
    /// [`BoundaryChoice`], selected by the configured policy and PRNG
    /// state.
    ///
    /// This is the canonical Antithesis-style decision point: same
    /// seed + same choice sequence → same decision sequence.
    pub fn decide_boundary(
        &mut self,
        choice: BoundaryChoice,
    ) -> Result<BoundaryDecision, SchedulerError> {
        let rng_pick = self.rng_state.next_bounded(4);
        let (decision, next_cursor) = select_decision(
            &choice,
            self.boundary_policy,
            self.decision_count,
            rng_pick,
            self.round_robin_cursor,
        );
        if matches!(
            (&choice, self.boundary_policy),
            (BoundaryChoice::Free, BoundaryPolicy::RoundRobin)
        ) {
            self.round_robin_cursor = next_cursor;
        }
        // Bump decision counter; surface overflow as a typed error.
        self.decision_count = self
            .decision_count
            .checked_add(1)
            .ok_or(SchedulerError::DecisionCounterOverflow)?;
        let post_rng_state = self.rng_state.raw_state();
        self.transcript.record(
            self.decision_count.saturating_sub(1),
            choice,
            decision.clone(),
            post_rng_state,
            None,
        );
        Ok(decision)
    }

    /// Helper: enforce the `max_steps` budget up front, before any
    /// work is performed.
    const fn enforce_step_budget(&self) -> Result<(), SchedulerError> {
        if self.step_count >= self.max_steps {
            return Err(SchedulerError::StepBudgetExhausted {
                budget: self.max_steps,
            });
        }
        Ok(())
    }

    /// Returns the current transcript entry count via a public
    /// accessor (delegates to
    /// [`crate::scheduler::transcript::BoundaryTranscript::len`]).
    #[must_use]
    pub fn transcript_len(&self) -> usize {
        self.transcript.len()
    }

    /// Returns the last recorded transcript entry, if any.
    #[must_use]
    pub fn last_transcript_entry(&self) -> Option<&BoundaryTranscriptEntry> {
        self.transcript.last()
    }
}

impl SeededScheduler {
    /// Performs one scheduler tick via `Runtime::tick_all`, records
    /// the decision in the transcript, and reports whether the run
    /// should continue, complete, or fail.
    ///
    /// `try_tick` is the inner driver used by
    /// [`run_to_completion`](Self::run_to_completion). It exposes the
    /// runtime's `all_alive` signal directly so the caller can
    /// distinguish a scheduler-chosen `Fail` (which terminates with
    /// [`RunEndReason::FailedDecision`]) from runtime shutdown
    /// (which terminates with [`RunEndReason::Completed`]).
    fn try_tick(&mut self) -> Result<TickOutcome, SchedulerError> {
        self.enforce_step_budget()?;
        let choice = BoundaryChoice::Free;
        let decision = self.decide_boundary(choice)?;
        let all_alive = self
            .runtime
            .tick_all()
            .map_err(|runtime_err| SchedulerError::Runtime {
                code: runtime_err.runtime_code().unwrap_or("unknown"),
                detail: runtime_err.to_string(),
            })?;
        // Bump step counter after the runtime accepted the tick.
        self.step_count =
            self.step_count
                .checked_add(1)
                .ok_or(SchedulerError::StepBudgetExhausted {
                    budget: self.max_steps,
                })?;
        if !all_alive {
            // Runtime reported natural completion (a shard has shut
            // down via `Runtime::tick_all` returning `Ok(false)`).
            // This supersedes whatever decision the scheduler
            // emitted for this tick and terminates the run.
            return Ok(TickOutcome::Complete);
        }
        if matches!(decision, BoundaryDecision::Fail { .. }) {
            Ok(TickOutcome::Fail)
        } else {
            Ok(TickOutcome::Continue)
        }
    }

    /// Drives the scheduler until natural completion (a shard
    /// reports shutdown via `Runtime::tick_all` returning
    /// `Ok(false)`), a fail decision, or a budget exhaustion.
    pub fn run_to_completion(&mut self) -> Result<RunResult, SchedulerError> {
        let mut ticks_executed: u32 = 0;
        loop {
            if self.step_count >= self.max_steps {
                return self.finish(
                    ticks_executed,
                    RunEndReason::StepBudgetExhausted {
                        budget: self.max_steps,
                    },
                );
            }
            if ticks_executed >= self.max_ticks {
                return self.finish(
                    ticks_executed,
                    RunEndReason::TickBudgetExhausted {
                        budget: self.max_ticks,
                    },
                );
            }
            match self.try_tick()? {
                TickOutcome::Continue => {}
                TickOutcome::Complete => {
                    return self.finish(ticks_executed, RunEndReason::Completed);
                }
                TickOutcome::Fail => {
                    return self.finish(ticks_executed, RunEndReason::FailedDecision);
                }
            }
            ticks_executed =
                ticks_executed
                    .checked_add(1)
                    .ok_or(SchedulerError::TickBudgetExhausted {
                        budget: self.max_ticks,
                    })?;
        }
    }

    /// Builds a `RunResult` for the supplied reason. `Completed` is
    /// emitted only when `try_tick` observed natural completion;
    /// `FailedDecision` is emitted when the scheduler chose a `Fail`
    /// boundary decision.
    fn finish(
        &self,
        ticks_executed: u32,
        reason: RunEndReason,
    ) -> Result<RunResult, SchedulerError> {
        Ok(RunResult {
            completed: matches!(reason, RunEndReason::Completed),
            ticks_executed,
            steps_executed: self.step_count,
            reason,
        })
    }
}
