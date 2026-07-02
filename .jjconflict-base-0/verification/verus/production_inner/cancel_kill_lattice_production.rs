// SPDX-License-Identifier: MIT
//
// IN-TREE PRODUCTION-SOURCE MIRROR for cancel/kill lifecycle lattice.
//
// This file is the WEAK `production_inner/` binding surface for
// `verification/verus/cancel_kill_lattice.rs`. It mirrors the observable
// state predicates and method names used by
// `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:127-174`.
//
// DRIFT POLICY: This file MUST be reviewed against
// `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:127-174` whenever
// production cancel/kill lifecycle behavior changes.
//
// Production `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:127-174`

#![forbid(unsafe_code)]
#![allow(dead_code)]

#[derive(Clone, Copy)]
pub struct RunId(pub u64);

#[derive(Clone, Copy)]
pub struct String;

#[derive(Clone, Copy)]
pub enum RuntimeError {
    RunNotFound,
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

#[derive(Clone, Copy)]
pub enum RuntimeJournalEvent {
    RunCancelled { run: RunId },
    RunKilled { run: RunId },
}

#[derive(Clone, Copy)]
pub enum TraceEvent {
    RunCancelled { run: RunId },
    RunKilled { run: RunId },
}

#[derive(Clone, Copy)]
pub struct Frame(pub u64);

#[derive(Clone, Copy)]
pub struct RunState {
    pub frame: Frame,
}

#[derive(Clone, Copy)]
pub struct Counters {
    pub failed: u64,
}

impl Counters {
    pub fn inc_failed(&mut self) {
        self.failed = self.failed.saturating_add(1);
    }
}

#[derive(Clone, Copy)]
pub struct TraceRing {
    pub pushed: bool,
}

impl TraceRing {
    pub fn push(&mut self, _event: TraceEvent) {
        self.pushed = true;
    }
}

#[derive(Clone, Copy)]
pub struct Shard {
    pub run_state_present: bool,
    pub terminal_runs_present: bool,
    pub pending_timer_present: bool,
    pub terminal_event_emitted: bool,
    pub stale_authority_valid: bool,
    pub counters: Counters,
    pub trace_ring: TraceRing,
}

impl Shard {
    pub fn run_state_contains(&self, _run: RunId) -> bool {
        self.run_state_present
    }

    pub fn terminal_runs_contains(&self, _run: RunId) -> bool {
        self.terminal_runs_present
    }

    pub fn emit_action_abandoned_for_pending(&mut self, _run: RunId) -> RuntimeResult<()> {
        Ok(())
    }

    pub fn append_journal_event(&mut self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        match event {
            RuntimeJournalEvent::RunCancelled { .. } | RuntimeJournalEvent::RunKilled { .. } => {
                self.terminal_event_emitted = true;
                Ok(())
            }
        }
    }

    pub fn pending_timer_remove(&mut self, _run: RunId) {
        self.pending_timer_present = false;
    }

    pub fn run_state_remove(&mut self, _run: RunId) -> Option<RunState> {
        if self.run_state_present {
            self.run_state_present = false;
            Some(RunState { frame: Frame(0) })
        } else {
            None
        }
    }

    pub fn release_frame(&mut self, _frame: Frame) {}

    pub fn terminal_runs_insert(&mut self, _run: RunId) -> RuntimeResult<()> {
        self.terminal_runs_present = true;
        Ok(())
    }

    pub fn runtime_state_remove(&mut self, _run: RunId) {}

    pub fn clear_executed_step_accounting(&mut self, _run: RunId) {}

    pub fn discard_journal_sequence(&mut self, _run: RunId) {
        self.stale_authority_valid = false;
    }

    #[verifier::external]
    pub fn handle_cancel(&mut self, _run: RunId, _reason: Option<String>) -> RuntimeResult<()> {
        loop {}
    }

    #[verifier::external]
    pub fn handle_kill(&mut self, _run: RunId, _reason: Option<String>) -> RuntimeResult<()> {
        loop {}
    }
}
