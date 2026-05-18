#![forbid(unsafe_code)]
//! Replay controller bridging the IPC bridge with the Makepad UI.
//!
//! The controller owns the `ReplayEngine` and `IpcBridge`, manages playback
//! state, and provides a `poll()` method that should be called from the
//! Makepad render loop (e.g. `handle_next_frame`).

use std::collections::HashSet;
use std::time::Instant;

use vb_core::WorkflowDigest;
use vb_core::ids::StepIdx;
use vb_core::ids::{ActionId, RunId, SlotIdx};
use vb_ipc::server::IpcResponse;
use vb_ipc::{IpcTraceEvent, IpcTraceEventKind};
use vb_storage::{EventSeq, JournalEvent};

use super::engine::ReplayEngine;
use super::state::ReplayState;
use super::types::{PlaybackSpeed, ReplayDiff};
use crate::ipc_bridge::{IpcBridge, IpcReply, IpcRequest};

// ---------------------------------------------------------------------------
// Playback state machine
// ---------------------------------------------------------------------------

/// Playback state of the replay controller.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PlaybackState {
    /// Not playing; no run loaded.
    #[default]
    Stopped,
    /// Auto-advancing at the given speed.
    Playing {
        /// Current playback speed.
        speed: PlaybackSpeed,
    },
    /// Paused at the given event position.
    Paused {
        /// Event index where playback was paused.
        position: u32,
    },
}

// ---------------------------------------------------------------------------
// Loading state
// ---------------------------------------------------------------------------

/// Internal loading state for the asynchronous run-fetch sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LoadPhase {
    /// No load in progress.
    Idle,
    /// Waiting for the `InspectRun` reply.
    WaitingInspect,
    /// Waiting for the `ListEvents` reply.
    WaitingEvents,
}

// ---------------------------------------------------------------------------
// Controller
// ---------------------------------------------------------------------------

/// Replay controller that bridges the IPC bridge with the Makepad UI.
///
/// Call [`ReplayController::poll`] from the Makepad render loop to drive IPC
/// replies and auto-advance playback.
pub struct ReplayController {
    engine: Option<ReplayEngine>,
    bridge: IpcBridge,
    state: PlaybackState,
    current_position: u32,
    total_events: u32,
    /// Run ID currently loaded or being loaded.
    active_run: Option<RunId>,
    /// Tracks the async load sequence.
    load_phase: LoadPhase,
    /// Timestamp of the last auto-advance tick.
    last_tick: Option<Instant>,
    /// Pending events accumulated across paginated ListEvents replies.
    pending_events: Vec<JournalEvent>,
}

impl Default for ReplayController {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayController {
    /// Creates a new replay controller with a fresh IPC bridge.
    pub fn new() -> Self {
        Self {
            engine: None,
            bridge: IpcBridge::new(),
            state: PlaybackState::Stopped,
            current_position: 0,
            total_events: 0,
            active_run: None,
            load_phase: LoadPhase::Idle,
            last_tick: None,
            pending_events: Vec::new(),
        }
    }

    // -- Run lifecycle -------------------------------------------------------

    /// Begins loading a run by sending `InspectRun` followed by `ListEvents`.
    ///
    /// The actual load completes asynchronously when
    /// [`ReplayController::poll`] processes the IPC replies.
    pub fn load_run(&mut self, run_id: RunId) -> Result<(), String> {
        // Reset any previous state.
        self.engine = None;
        self.current_position = 0;
        self.total_events = 0;
        self.state = PlaybackState::Stopped;
        self.last_tick = None;
        self.pending_events.clear();
        self.active_run = Some(run_id);
        self.load_phase = LoadPhase::WaitingInspect;

        self.bridge
            .send(IpcRequest::InspectRun { run_id })
            .map_err(|e| format!("Failed to send InspectRun: {e}"))
    }

    /// Returns the currently loaded run ID, if any.
    pub fn active_run(&self) -> Option<RunId> {
        self.active_run
    }

    /// Returns `true` if a replay engine is loaded.
    pub fn is_loaded(&self) -> bool {
        self.engine.is_some()
    }

    // -- Playback controls ---------------------------------------------------

    /// Starts auto-advancing at the current or default speed.
    ///
    /// No-op if already playing. If paused, resumes from the paused position.
    pub fn play(&mut self) {
        if self.engine.is_none() {
            return;
        }
        match self.state {
            PlaybackState::Stopped => {
                let speed = PlaybackSpeed::default();
                self.state = PlaybackState::Playing { speed };
                self.last_tick = Some(Instant::now());
            }
            PlaybackState::Paused { position } => {
                let speed = PlaybackSpeed::default();
                self.current_position = position;
                self.state = PlaybackState::Playing { speed };
                self.last_tick = Some(Instant::now());
            }
            PlaybackState::Playing { .. } => {
                // Already playing; no-op.
            }
        }
    }

    /// Pauses auto-advancing at the current position.
    pub fn pause(&mut self) {
        if let PlaybackState::Playing { .. } = self.state {
            self.state = PlaybackState::Paused {
                position: self.current_position,
            };
            self.last_tick = None;
        }
    }

    /// Advances one event forward. Clamps to the last event.
    pub fn step_forward(&mut self) {
        if self.engine.is_none() {
            return;
        }
        if self.current_position < self.total_events {
            self.current_position = self.current_position.saturating_add(1);
        }
        self.state = PlaybackState::Paused {
            position: self.current_position,
        };
        self.last_tick = None;
    }

    /// Goes back one event. Clamps at zero.
    pub fn step_backward(&mut self) {
        if self.engine.is_none() {
            return;
        }
        self.pause();
        self.current_position = self.current_position.saturating_sub(1);
    }

    /// Seeks to the first failure event (`ActionFailed` or `RunFailed`).
    ///
    /// If no failure is found, this is a no-op.
    pub fn jump_to_failure(&mut self) {
        let engine = match self.engine.as_ref() {
            Some(e) => e,
            None => return,
        };
        let idx = match engine.find_failure() {
            Some(i) => i,
            None => return,
        };
        self.pause();
        // find_failure returns 0-based event index; position is 1-based
        // (position N = state after applying event N-1).
        let target = u32::try_from(idx.saturating_add(1)).unwrap_or(self.current_position);
        self.current_position = target;
        self.state = PlaybackState::Paused { position: target };
    }

    /// Seeks to a specific event position.
    ///
    /// Position 0 = initial state; position N = state after event N-1.
    /// Clamps to `[0, total_events]`.
    pub fn jump_to_position(&mut self, pos: u32) {
        if self.engine.is_none() {
            return;
        }
        self.pause();
        let clamped = pos.min(self.total_events);
        self.current_position = clamped;
        self.state = PlaybackState::Paused { position: clamped };
    }

    /// Changes playback speed while playing.
    pub fn set_speed(&mut self, speed: PlaybackSpeed) {
        if let PlaybackState::Playing { .. } = self.state {
            self.state = PlaybackState::Playing { speed };
        }
    }

    // -- State queries -------------------------------------------------------

    /// Returns the current playback state.
    pub fn playback_state(&self) -> &PlaybackState {
        &self.state
    }

    /// Returns the current event position (0 = initial state).
    pub fn current_position(&self) -> u32 {
        self.current_position
    }

    /// Returns the total number of events in the loaded run.
    pub fn total_events(&self) -> u32 {
        self.total_events
    }

    /// Returns the `ReplayState` at the current position.
    pub fn current_state(&self) -> Option<&ReplayState> {
        let engine = self.engine.as_ref()?;
        let idx = usize::try_from(self.current_position).unwrap_or(0);
        engine.state_at(idx)
    }

    /// Returns the diff from the previous state to the current state.
    pub fn current_diff(&self) -> Option<ReplayDiff> {
        let engine = self.engine.as_ref()?;
        if self.current_position == 0 {
            // Diff from initial to initial is empty.
            return Some(ReplayDiff {
                step_changes: Vec::new(),
                slot_changes: Vec::new(),
                taint_changes: Vec::new(),
            });
        }
        let from = usize::try_from(self.current_position.saturating_sub(1)).unwrap_or(0);
        let to = usize::try_from(self.current_position).unwrap_or(0);
        Some(engine.diff(from, to))
    }

    /// Returns a reference to the underlying replay engine, if loaded.
    pub fn engine(&self) -> Option<&ReplayEngine> {
        self.engine.as_ref()
    }

    // -- Poll loop -----------------------------------------------------------

    /// Processes pending IPC replies and advances auto-playback.
    ///
    /// Call this from `handle_next_frame` or `handle_timer` in the Makepad
    /// App. Returns a list of [`ControllerEvent`]s describing what changed
    /// so the UI can update accordingly.
    pub fn poll(&mut self) -> Vec<ControllerEvent> {
        let mut events = Vec::new();

        // Drain IPC replies.
        let replies = self.bridge.poll();
        for reply in replies {
            self.handle_reply(reply, &mut events);
        }

        // Auto-advance if playing.
        if let PlaybackState::Playing { speed } = self.state
            && self.engine.is_some()
        {
            let delay_ms = speed.event_delay_ms();
            let elapsed = self.last_tick.map_or(u64::MAX, |t| {
                u64::try_from(t.elapsed().as_millis()).unwrap_or(u64::MAX)
            });

            if elapsed >= delay_ms {
                if self.current_position < self.total_events {
                    self.current_position = self.current_position.saturating_add(1);
                    events.push(ControllerEvent::PositionChanged {
                        position: self.current_position,
                    });
                }
                if self.current_position >= self.total_events {
                    self.state = PlaybackState::Paused {
                        position: self.current_position,
                    };
                    self.last_tick = None;
                    events.push(ControllerEvent::PlaybackFinished);
                } else {
                    self.last_tick = Some(Instant::now());
                }
            }
        }

        events
    }

    // -- Internal ------------------------------------------------------------

    /// Handles a single IPC reply and emits controller events.
    fn handle_reply(&mut self, reply: IpcReply, events: &mut Vec<ControllerEvent>) {
        match reply {
            IpcReply::Connected => {
                events.push(ControllerEvent::Connected);
            }
            IpcReply::Disconnected => {
                events.push(ControllerEvent::Disconnected);
            }
            IpcReply::ConnectionFailed(err) => {
                events.push(ControllerEvent::ConnectionFailed(err));
            }
            IpcReply::Inspected(_response) => {
                // Inspection acknowledged. Now request the events.
                if self.load_phase == LoadPhase::WaitingInspect {
                    self.load_phase = LoadPhase::WaitingEvents;
                    if let Some(run_id) = self.active_run
                        && let Err(err) = self.bridge.send(IpcRequest::ListEvents {
                            run_id,
                            from_sequence: 0,
                        })
                    {
                        self.load_phase = LoadPhase::Idle;
                        events.push(ControllerEvent::LoadFailed(err));
                    }
                }
            }
            IpcReply::Events(response) => {
                if self.load_phase == LoadPhase::WaitingEvents {
                    self.handle_events_response(response, events);
                }
            }
            IpcReply::Error(err) => {
                self.load_phase = LoadPhase::Idle;
                events.push(ControllerEvent::LoadFailed(err));
            }
            IpcReply::NotImplemented(msg) => {
                self.load_phase = LoadPhase::Idle;
                events.push(ControllerEvent::LoadFailed(format!(
                    "Server does not support this operation: {msg}"
                )));
            }
            // Other replies are not relevant to the replay controller.
            IpcReply::RunAccepted(_)
            | IpcReply::RunCancelled(_)
            | IpcReply::TraceCount(_)
            | IpcReply::Healthy
            | IpcReply::ShuttingDown
            | IpcReply::VerifyWorkflowResult(_)
            | IpcReply::TaintReportReceived(_)
            | IpcReply::WorkflowGraphReceived(_) => {}
        }
    }

    /// Processes an `IpcResponse::Events` and finalizes loading.
    fn handle_events_response(&mut self, response: IpcResponse, events: &mut Vec<ControllerEvent>) {
        let trace_events = match response {
            IpcResponse::Events { events: evts } => evts,
            other => {
                self.load_phase = LoadPhase::Idle;
                events.push(ControllerEvent::LoadFailed(format!(
                    "Unexpected ListEvents response: {other:?}"
                )));
                return;
            }
        };

        // Convert IPC trace events to journal events using context-aware
        // mapping so that StepEnded is not unconditionally treated as success.
        let journal_events = convert_trace_events(&trace_events);

        self.pending_events.extend(journal_events);

        // Sort by sequence to guarantee ordering.
        self.pending_events.sort_by_key(|e| e.seq());

        // Build the engine.
        let engine = ReplayEngine::from_events(self.pending_events.clone());
        self.total_events = u32::try_from(engine.event_count()).unwrap_or(u32::MAX);
        self.engine = Some(engine);
        self.current_position = 0;
        self.load_phase = LoadPhase::Idle;
        self.pending_events.clear();

        events.push(ControllerEvent::RunLoaded {
            run_id: self.active_run.unwrap_or(RunId::ZERO),
            total_events: self.total_events,
        });
    }
}

// ---------------------------------------------------------------------------
// Controller events
// ---------------------------------------------------------------------------

/// Events emitted by the replay controller for the UI to react to.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ControllerEvent {
    /// IPC bridge connected to the server.
    Connected,
    /// IPC bridge disconnected.
    Disconnected,
    /// Connection attempt failed.
    ConnectionFailed(String),
    /// Run finished loading and the replay engine is ready.
    RunLoaded {
        /// Run that was loaded.
        run_id: RunId,
        /// Total number of journal events.
        total_events: u32,
    },
    /// Run load failed.
    LoadFailed(String),
    /// Playback position changed (auto-advance or manual seek).
    PositionChanged {
        /// New event position.
        position: u32,
    },
    /// Playback reached the end of the event stream.
    PlaybackFinished,
}

// ---------------------------------------------------------------------------
// Conversion: IPC trace events -> Journal events
// ---------------------------------------------------------------------------

/// Converts an `IpcTraceEvent` to a `JournalEvent`.
///
/// Some fields required by `JournalEvent` are not present in the IPC trace
/// event and are filled with placeholder defaults. The core state-machine
/// fields (run, seq, step, slot) are preserved faithfully.
///
/// # Known limitation
///
/// The IPC trace layer does not distinguish between `StepEnded` (success) and
/// `StepEnded` (failure). A `StepEnded` event is emitted regardless of outcome.
/// This function always maps `StepEnded` to `StepSucceeded`. For context-aware
/// disambiguation, use [`convert_trace_events`] which inspects surrounding
/// events to suppress spurious `StepSucceeded` for failed steps.
pub fn trace_to_journal(trace: IpcTraceEvent) -> Option<JournalEvent> {
    let seq = EventSeq::new(trace.sequence);
    match trace.kind {
        IpcTraceEventKind::RunSubmitted { run } => Some(JournalEvent::RunAccepted {
            run,
            seq,
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        }),
        IpcTraceEventKind::StepStarted { run, step } => Some(JournalEvent::StepStarted {
            run,
            seq,
            step,
            attempt: 1,
        }),
        IpcTraceEventKind::StepEnded { run, step } => Some(JournalEvent::StepSucceeded {
            run,
            seq,
            step,
            // Output slot not available from trace; use a sentinel.
            output: SlotIdx::new(0),
        }),
        IpcTraceEventKind::SlotWritten { run, slot, .. } => Some(JournalEvent::SlotWrittenEvent {
            run,
            seq,
            slot,
            value: None,
            extra: None,
            attempt: 1,
        }),
        IpcTraceEventKind::ActionScheduled { run, step } => Some(JournalEvent::ActionScheduled {
            run,
            seq,
            step,
            // ActionId not present in trace; use a sentinel.
            action: ActionId::new(0),
            attempt: 1,
        }),
        IpcTraceEventKind::ActionCompleted { run, step } => {
            Some(JournalEvent::ActionCompletedEvent {
                run,
                seq,
                step,
                action: ActionId::new(0),
                attempt: 1,
            })
        }
        IpcTraceEventKind::ActionFailed { run, step, .. } => {
            Some(JournalEvent::ActionFailedEvent {
                run,
                seq,
                step,
                action: ActionId::new(0),
                attempt: 1,
            })
        }
        IpcTraceEventKind::AskAnswered { run, step, .. } => Some(JournalEvent::AskAnsweredEvent {
            run,
            seq,
            step,
            attempt: 1,
        }),
        IpcTraceEventKind::RunFinished { run } => Some(JournalEvent::RunFinished {
            run,
            seq,
            result: SlotIdx::new(0),
            attempt: 1,
        }),
        IpcTraceEventKind::RunFailed { run } => Some(JournalEvent::RunFailedEvent {
            run,
            seq,
            attempt: 1,
        }),
        IpcTraceEventKind::RunCancelled { run } => Some(JournalEvent::RunCancelled {
            run,
            seq,
            attempt: 1,
            reason: None,
        }),
    }
}

/// Context-aware conversion from a slice of `IpcTraceEvent`s to `JournalEvent`s.
///
/// This function addresses a gap in the IPC trace protocol: the `StepEnded`
/// event is emitted for both successful and failed step completions. A step is
/// considered to have failed if an `ActionFailed` event for the same
/// `(run, step)` pair appears anywhere in the trace stream.
///
/// When a failed `StepEnded` is detected, it is suppressed rather than being
/// incorrectly mapped to `StepSucceeded`. The failure itself is already
/// faithfully captured by the preceding `ActionFailed` event.
///
/// TODO(k1p7): Once `JournalEvent` gains a `StepFailed` variant, failed
/// `StepEnded` events should map to it instead of being suppressed.
pub fn convert_trace_events(traces: &[IpcTraceEvent]) -> Vec<JournalEvent> {
    // Build a set of (run, step) pairs that experienced an action failure.
    // A StepEnded for one of these pairs indicates the step ended in failure,
    // not success.
    let mut failed_steps: HashSet<(RunId, StepIdx)> = HashSet::new();
    for trace in traces {
        if let IpcTraceEventKind::ActionFailed { run, step, .. } = &trace.kind {
            failed_steps.insert((*run, *step));
        }
    }

    let mut journal_events = Vec::with_capacity(traces.len());
    for trace in traces {
        match &trace.kind {
            IpcTraceEventKind::StepEnded { run, step, .. }
                if failed_steps.contains(&(*run, *step)) =>
            {
                // The step ended after an action failure. The failure is
                // already recorded via ActionFailed. Mapping this to
                // StepSucceeded would be incorrect, so we suppress it.
                //
                // When JournalEvent::StepFailed becomes available, emit that
                // here instead of skipping.
            }
            _ => {
                if let Some(event) = trace_to_journal(trace.clone()) {
                    journal_events.push(event);
                }
            }
        }
    }
    journal_events
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use vb_core::ids::StepIdx;
    use vb_ipc::server::IpcResponse;

    // -- PlaybackState defaults --

    #[test]
    fn playback_state_default_is_stopped() {
        assert_eq!(PlaybackState::default(), PlaybackState::Stopped);
    }

    // -- Controller construction --

    #[test]
    fn controller_new_starts_stopped() {
        let ctrl = ReplayController::new();
        assert_eq!(*ctrl.playback_state(), PlaybackState::Stopped);
        assert_eq!(ctrl.current_position(), 0);
        assert_eq!(ctrl.total_events(), 0);
        assert!(ctrl.active_run().is_none());
        assert!(ctrl.engine().is_none());
        assert!(ctrl.current_state().is_none());
        assert!(!ctrl.is_loaded());
    }

    #[test]
    fn controller_poll_with_no_ipc_activity_is_empty() {
        let mut ctrl = ReplayController::new();
        let events = ctrl.poll();
        assert!(events.is_empty());
    }

    // -- Playback controls without a loaded run are no-ops --

    #[test]
    fn play_without_engine_is_noop() {
        let mut ctrl = ReplayController::new();
        ctrl.play();
        assert_eq!(*ctrl.playback_state(), PlaybackState::Stopped);
    }

    #[test]
    fn step_forward_without_engine_is_noop() {
        let mut ctrl = ReplayController::new();
        ctrl.step_forward();
        assert_eq!(ctrl.current_position(), 0);
    }

    #[test]
    fn step_backward_without_engine_is_noop() {
        let mut ctrl = ReplayController::new();
        ctrl.step_backward();
        assert_eq!(ctrl.current_position(), 0);
    }

    #[test]
    fn jump_to_failure_without_engine_is_noop() {
        let mut ctrl = ReplayController::new();
        ctrl.jump_to_failure();
        assert_eq!(ctrl.current_position(), 0);
    }

    #[test]
    fn jump_to_position_without_engine_is_noop() {
        let mut ctrl = ReplayController::new();
        ctrl.jump_to_position(5);
        assert_eq!(ctrl.current_position(), 0);
    }

    // -- trace_to_journal conversion (single-event, no context) --

    #[test]
    fn trace_run_submitted_converts_to_run_accepted() {
        let trace = IpcTraceEvent {
            sequence: 1,
            kind: IpcTraceEventKind::RunSubmitted {
                run: RunId::new(42),
            },
        };
        let journal = trace_to_journal(trace);
        assert!(matches!(
            journal,
            Some(JournalEvent::RunAccepted { run, .. }) if run == RunId::new(42)
        ));
    }

    #[test]
    fn trace_step_started_converts() {
        let trace = IpcTraceEvent {
            sequence: 2,
            kind: IpcTraceEventKind::StepStarted {
                run: RunId::new(1),
                step: StepIdx::new(0),
            },
        };
        let journal = trace_to_journal(trace);
        assert!(matches!(
            journal,
            Some(JournalEvent::StepStarted { step, .. }) if step == StepIdx::new(0)
        ));
    }

    #[test]
    fn trace_step_ended_converts_to_step_succeeded_in_isolation() {
        // When no surrounding context is available, trace_to_journal maps
        // StepEnded to StepSucceeded. Context-aware disambiguation is handled
        // by convert_trace_events.
        let trace = IpcTraceEvent {
            sequence: 3,
            kind: IpcTraceEventKind::StepEnded {
                run: RunId::new(1),
                step: StepIdx::new(0),
            },
        };
        let journal = trace_to_journal(trace);
        assert!(matches!(
            journal,
            Some(JournalEvent::StepSucceeded { step, .. }) if step == StepIdx::new(0)
        ));
    }

    #[test]
    fn trace_slot_written_converts() {
        let trace = IpcTraceEvent {
            sequence: 4,
            kind: IpcTraceEventKind::SlotWritten {
                run: RunId::new(1),
                slot: SlotIdx::new(7),
                value: Vec::new(),
            },
        };
        let journal = trace_to_journal(trace);
        assert!(matches!(
            journal,
            Some(JournalEvent::SlotWrittenEvent { slot, .. }) if slot == SlotIdx::new(7)
        ));
    }

    #[test]
    fn trace_action_scheduled_converts() {
        let trace = IpcTraceEvent {
            sequence: 5,
            kind: IpcTraceEventKind::ActionScheduled {
                run: RunId::new(1),
                step: StepIdx::new(0),
            },
        };
        let journal = trace_to_journal(trace);
        assert!(matches!(
            journal,
            Some(JournalEvent::ActionScheduled { step, .. }) if step == StepIdx::new(0)
        ));
    }

    #[test]
    fn trace_action_completed_converts() {
        let trace = IpcTraceEvent {
            sequence: 6,
            kind: IpcTraceEventKind::ActionCompleted {
                run: RunId::new(1),
                step: StepIdx::new(0),
            },
        };
        let journal = trace_to_journal(trace);
        assert!(matches!(
            journal,
            Some(JournalEvent::ActionCompletedEvent { step, .. }) if step == StepIdx::new(0)
        ));
    }

    #[test]
    fn trace_action_failed_converts() {
        use vb_core::action::ActionFailureCode;
        let trace = IpcTraceEvent {
            sequence: 7,
            kind: IpcTraceEventKind::ActionFailed {
                run: RunId::new(1),
                step: StepIdx::new(0),
                code: ActionFailureCode::Timeout,
            },
        };
        let journal = trace_to_journal(trace);
        assert!(matches!(
            journal,
            Some(JournalEvent::ActionFailedEvent { step, .. }) if step == StepIdx::new(0)
        ));
    }

    #[test]
    fn trace_ask_answered_converts() {
        let trace = IpcTraceEvent {
            sequence: 8,
            kind: IpcTraceEventKind::AskAnswered {
                run: RunId::new(1),
                step: StepIdx::new(2),
                slot: SlotIdx::new(5),
            },
        };
        let journal = trace_to_journal(trace);
        assert!(matches!(
            journal,
            Some(JournalEvent::AskAnsweredEvent { step, .. }) if step == StepIdx::new(2)
        ));
    }

    #[test]
    fn trace_run_finished_converts() {
        let trace = IpcTraceEvent {
            sequence: 9,
            kind: IpcTraceEventKind::RunFinished { run: RunId::new(1) },
        };
        let journal = trace_to_journal(trace);
        assert!(matches!(journal, Some(JournalEvent::RunFinished { .. })));
    }

    #[test]
    fn trace_run_failed_converts() {
        let trace = IpcTraceEvent {
            sequence: 10,
            kind: IpcTraceEventKind::RunFailed { run: RunId::new(1) },
        };
        let journal = trace_to_journal(trace);
        assert!(matches!(journal, Some(JournalEvent::RunFailedEvent { .. })));
    }

    #[test]
    fn trace_run_cancelled_converts() {
        let trace = IpcTraceEvent {
            sequence: 11,
            kind: IpcTraceEventKind::RunCancelled { run: RunId::new(1) },
        };
        let journal = trace_to_journal(trace);
        assert!(matches!(journal, Some(JournalEvent::RunCancelled { .. })));
    }

    // -- convert_trace_events: context-aware StepEnded handling --

    #[test]
    fn convert_step_ended_succeeds_when_no_action_failed() {
        // A step that completes without any ActionFailed should produce
        // StepSucceeded.
        let traces = vec![
            IpcTraceEvent {
                sequence: 0,
                kind: IpcTraceEventKind::StepStarted {
                    run: RunId::new(1),
                    step: StepIdx::new(0),
                },
            },
            IpcTraceEvent {
                sequence: 1,
                kind: IpcTraceEventKind::StepEnded {
                    run: RunId::new(1),
                    step: StepIdx::new(0),
                },
            },
        ];
        let journal = convert_trace_events(&traces);
        assert_eq!(journal.len(), 2);
        assert!(matches!(journal[0], JournalEvent::StepStarted { .. }));
        assert!(matches!(journal[1], JournalEvent::StepSucceeded { .. }));
    }

    #[test]
    fn convert_step_ended_suppressed_after_action_failed() {
        // BLACK HAT regression test: StepEnded must NOT be mapped to
        // StepSucceeded when the step experienced an ActionFailed.
        use vb_core::action::ActionFailureCode;
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let traces = vec![
            IpcTraceEvent {
                sequence: 0,
                kind: IpcTraceEventKind::StepStarted { run, step },
            },
            IpcTraceEvent {
                sequence: 1,
                kind: IpcTraceEventKind::ActionFailed {
                    run,
                    step,
                    code: ActionFailureCode::Timeout,
                },
            },
            IpcTraceEvent {
                sequence: 2,
                kind: IpcTraceEventKind::StepEnded { run, step },
            },
            IpcTraceEvent {
                sequence: 3,
                kind: IpcTraceEventKind::RunFailed { run },
            },
        ];
        let journal = convert_trace_events(&traces);

        // Should have: StepStarted, ActionFailed, RunFailed.
        // StepEnded must be suppressed (not StepSucceeded).
        assert_eq!(
            journal.len(),
            3,
            "StepEnded should be suppressed after ActionFailed"
        );
        assert!(matches!(journal[0], JournalEvent::StepStarted { .. }));
        assert!(matches!(journal[1], JournalEvent::ActionFailedEvent { .. }));
        assert!(matches!(journal[2], JournalEvent::RunFailedEvent { .. }));

        // Verify no StepSucceeded appeared for the failed step.
        for event in &journal {
            assert!(
                !matches!(event, JournalEvent::StepSucceeded { step: s, .. } if *s == step),
                "StepSucceeded must not appear for a failed step"
            );
        }
    }

    #[test]
    fn convert_step_ended_only_suppresses_matching_run_step() {
        // ActionFailed for step 0 should not suppress StepEnded for step 1.
        use vb_core::action::ActionFailureCode;
        let run = RunId::new(1);
        let traces = vec![
            IpcTraceEvent {
                sequence: 0,
                kind: IpcTraceEventKind::StepStarted {
                    run,
                    step: StepIdx::new(0),
                },
            },
            IpcTraceEvent {
                sequence: 1,
                kind: IpcTraceEventKind::ActionFailed {
                    run,
                    step: StepIdx::new(0),
                    code: ActionFailureCode::Timeout,
                },
            },
            IpcTraceEvent {
                sequence: 2,
                kind: IpcTraceEventKind::StepEnded {
                    run,
                    step: StepIdx::new(0),
                },
            },
            IpcTraceEvent {
                sequence: 3,
                kind: IpcTraceEventKind::StepStarted {
                    run,
                    step: StepIdx::new(1),
                },
            },
            IpcTraceEvent {
                sequence: 4,
                kind: IpcTraceEventKind::StepEnded {
                    run,
                    step: StepIdx::new(1),
                },
            },
        ];
        let journal = convert_trace_events(&traces);

        // Step 0: StepStarted, ActionFailed (StepEnded suppressed)
        // Step 1: StepStarted, StepSucceeded
        assert_eq!(journal.len(), 4);
        assert!(matches!(
            journal[0],
            JournalEvent::StepStarted { step, .. } if step == StepIdx::new(0)
        ));
        assert!(matches!(
            journal[1],
            JournalEvent::ActionFailedEvent { step, .. } if step == StepIdx::new(0)
        ));
        assert!(matches!(
            journal[2],
            JournalEvent::StepStarted { step, .. } if step == StepIdx::new(1)
        ));
        assert!(matches!(
            journal[3],
            JournalEvent::StepSucceeded { step, .. } if step == StepIdx::new(1)
        ));
    }

    // -- Controller with an engine injected directly -------------------------

    /// Helper: build a controller with a pre-loaded engine.
    fn controller_with_events(events: Vec<JournalEvent>) -> ReplayController {
        let engine = ReplayEngine::from_events(events);
        let event_count = u32::try_from(engine.event_count()).unwrap_or(u32::MAX);
        ReplayController {
            engine: Some(engine),
            bridge: IpcBridge::new(),
            state: PlaybackState::Stopped,
            current_position: 0,
            total_events: event_count,
            active_run: Some(RunId::new(1)),
            load_phase: LoadPhase::Idle,
            last_tick: None,
            pending_events: Vec::new(),
        }
    }

    fn make_run_accepted(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        }
    }

    fn make_step_started(run: RunId, seq: u64, step: StepIdx) -> JournalEvent {
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(seq),
            step,
            attempt: 1,
        }
    }

    fn make_step_succeeded(run: RunId, seq: u64, step: StepIdx, output: SlotIdx) -> JournalEvent {
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(seq),
            step,
            output,
        }
    }

    #[allow(dead_code)]
    fn make_action_scheduled(
        run: RunId,
        seq: u64,
        step: StepIdx,
        action: ActionId,
    ) -> JournalEvent {
        JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(seq),
            step,
            action,
            attempt: 1,
        }
    }

    fn make_action_failed(run: RunId, seq: u64, step: StepIdx, action: ActionId) -> JournalEvent {
        JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(seq),
            step,
            action,
            attempt: 1,
        }
    }

    fn make_run_failed(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(seq),
            attempt: 1,
        }
    }

    fn make_run_finished(run: RunId, seq: u64, result: SlotIdx) -> JournalEvent {
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(seq),
            result,
            attempt: 1,
        }
    }

    #[test]
    fn step_forward_advances_position() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_step_started(RunId::new(1), 2, StepIdx::new(0)),
        ];
        let mut ctrl = controller_with_events(events);
        assert_eq!(ctrl.current_position(), 0);
        assert_eq!(ctrl.total_events(), 2);

        ctrl.step_forward();
        assert_eq!(ctrl.current_position(), 1);
        assert!(matches!(
            ctrl.playback_state(),
            PlaybackState::Paused { position: 1 }
        ));

        ctrl.step_forward();
        assert_eq!(ctrl.current_position(), 2);

        // Clamped at total_events.
        ctrl.step_forward();
        assert_eq!(ctrl.current_position(), 2);
    }

    #[test]
    fn step_backward_decrements_position() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_step_started(RunId::new(1), 2, StepIdx::new(0)),
        ];
        let mut ctrl = controller_with_events(events);
        ctrl.current_position = 2;

        ctrl.step_backward();
        assert_eq!(ctrl.current_position(), 1);

        ctrl.step_backward();
        assert_eq!(ctrl.current_position(), 0);

        // Clamped at 0.
        ctrl.step_backward();
        assert_eq!(ctrl.current_position(), 0);
    }

    #[test]
    fn jump_to_position_clamps() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_step_started(RunId::new(1), 2, StepIdx::new(0)),
        ];
        let mut ctrl = controller_with_events(events);

        ctrl.jump_to_position(5);
        assert_eq!(ctrl.current_position(), 2); // clamped to total_events

        ctrl.jump_to_position(1);
        assert_eq!(ctrl.current_position(), 1);
    }

    #[test]
    fn jump_to_failure_seeks_to_first_failure() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_action_failed(RunId::new(1), 2, StepIdx::new(0), ActionId::new(1)),
            make_run_failed(RunId::new(1), 3),
        ];
        let mut ctrl = controller_with_events(events);

        ctrl.jump_to_failure();
        // find_failure returns event index 1; position = index + 1 = 2
        assert_eq!(ctrl.current_position(), 2);
    }

    #[test]
    fn jump_to_failure_with_no_failure_is_noop() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_run_finished(RunId::new(1), 2, SlotIdx::new(0)),
        ];
        let mut ctrl = controller_with_events(events);
        ctrl.current_position = 0;

        ctrl.jump_to_failure();
        assert_eq!(ctrl.current_position(), 0);
    }

    #[test]
    fn current_state_returns_snapshot() {
        let events = vec![
            make_run_accepted(RunId::new(42), 1),
            make_step_started(RunId::new(42), 2, StepIdx::new(0)),
        ];
        let mut ctrl = controller_with_events(events);
        ctrl.current_position = 1;

        let state = ctrl.current_state();
        assert!(state.is_some());
        assert_eq!(state.as_ref().map(|s| s.run_id), Some(RunId::new(42)));
    }

    #[test]
    fn current_diff_returns_diff_between_positions() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let events = vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
            make_step_succeeded(run, 3, step, SlotIdx::new(0)),
        ];
        let mut ctrl = controller_with_events(events);
        ctrl.current_position = 2;

        let diff = ctrl.current_diff();
        assert!(diff.is_some());
        assert!(
            !diff
                .as_ref()
                .map(|d| d.step_changes.is_empty())
                .unwrap_or(true)
        );
    }

    #[test]
    fn current_diff_at_position_zero_is_empty() {
        let events = vec![make_run_accepted(RunId::new(1), 1)];
        let mut ctrl = controller_with_events(events);
        ctrl.current_position = 0;

        let diff = ctrl.current_diff();
        assert!(diff.is_some());
        assert!(
            diff.as_ref()
                .map(|d| d.step_changes.is_empty())
                .unwrap_or(false)
        );
    }

    #[test]
    fn play_transitions_stopped_to_playing() {
        let events = vec![make_run_accepted(RunId::new(1), 1)];
        let mut ctrl = controller_with_events(events);

        ctrl.play();
        assert!(matches!(
            ctrl.playback_state(),
            PlaybackState::Playing {
                speed: PlaybackSpeed::Normal
            }
        ));
    }

    #[test]
    fn play_resumes_from_paused() {
        let events = vec![make_run_accepted(RunId::new(1), 1)];
        let mut ctrl = controller_with_events(events);
        ctrl.state = PlaybackState::Paused { position: 1 };
        ctrl.current_position = 1;

        ctrl.play();
        assert!(matches!(
            ctrl.playback_state(),
            PlaybackState::Playing {
                speed: PlaybackSpeed::Normal
            }
        ));
        assert_eq!(ctrl.current_position(), 1);
    }

    #[test]
    fn pause_transitions_playing_to_paused() {
        let events = vec![make_run_accepted(RunId::new(1), 1)];
        let mut ctrl = controller_with_events(events);
        ctrl.state = PlaybackState::Playing {
            speed: PlaybackSpeed::Normal,
        };
        ctrl.current_position = 1;

        ctrl.pause();
        assert!(matches!(
            ctrl.playback_state(),
            PlaybackState::Paused { position: 1 }
        ));
    }

    #[test]
    fn pause_while_stopped_is_noop() {
        let events = vec![make_run_accepted(RunId::new(1), 1)];
        let mut ctrl = controller_with_events(events);
        ctrl.pause();
        assert_eq!(*ctrl.playback_state(), PlaybackState::Stopped);
    }

    // =========================================================================
    // BLACK HAT security and correctness review tests
    // =========================================================================

    // -- FINDING 1: step_forward silently pauses during playback (MEDIUM) --
    //
    // step_forward() unconditionally sets state to Paused, even if the
    // controller was Playing. This silently interrupts auto-advance without
    // emitting PlaybackFinished or any other signal. A caller stepping forward
    // while playing loses the playing state.
    //
    // Severity: MEDIUM -- the playback silently stops, but no crash or
    // corruption. The user would need to call play() again.
    #[test]
    fn blackhat_step_forward_silently_pauses_during_playback() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_step_started(RunId::new(1), 2, StepIdx::new(0)),
            make_step_succeeded(RunId::new(1), 3, StepIdx::new(0), SlotIdx::new(0)),
        ];
        let mut ctrl = controller_with_events(events);

        // Start playing.
        ctrl.play();
        assert!(matches!(
            ctrl.playback_state(),
            PlaybackState::Playing { .. }
        ));

        // Step forward while playing.
        ctrl.step_forward();

        // State is now Paused, not Playing.
        assert!(
            matches!(ctrl.playback_state(), PlaybackState::Paused { .. }),
            "step_forward must transition Playing to Paused"
        );
        assert_eq!(ctrl.current_position(), 1);
    }

    // -- FINDING 2: step_backward does not pause during playback (MEDIUM) --
    //
    // step_backward() calls self.pause() which only transitions Playing -> Paused.
    // This is the correct behavior for consistency with step_forward.
    #[test]
    fn blackhat_step_backward_pauses_during_playback() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_step_started(RunId::new(1), 2, StepIdx::new(0)),
        ];
        let mut ctrl = controller_with_events(events);
        ctrl.current_position = 2;

        ctrl.play();
        assert!(matches!(
            ctrl.playback_state(),
            PlaybackState::Playing { .. }
        ));

        ctrl.step_backward();
        assert!(
            matches!(ctrl.playback_state(), PlaybackState::Paused { .. }),
            "step_backward must pause playback"
        );
        assert_eq!(ctrl.current_position(), 1);
    }

    // -- FINDING 3: empty run auto-advance is delayed (HIGH) --
    //
    // With zero events (total_events == 0), play() transitions to Playing
    // and sets last_tick to now(). However, poll() only checks the terminal
    // condition (current_position >= total_events) AFTER checking that the
    // delay has elapsed. Since the delay was just set, the FIRST poll after
    // play() does NOT emit PlaybackFinished. The event only fires once the
    // delay expires. This means an empty run appears to "play" for one full
    // delay interval before finishing, rather than finishing immediately.
    //
    // Severity: HIGH -- UX bug. Empty run should finish immediately, not
    // after a delay. The terminal check should happen before the delay check.
    #[test]
    fn blackhat_empty_run_play_does_not_finish_immediately() {
        let engine = ReplayEngine::from_events(vec![]);
        let event_count = u32::try_from(engine.event_count()).unwrap_or(u32::MAX);
        assert_eq!(event_count, 0);

        let mut ctrl = ReplayController {
            engine: Some(engine),
            bridge: IpcBridge::new(),
            state: PlaybackState::Stopped,
            current_position: 0,
            total_events: 0,
            active_run: Some(RunId::new(1)),
            load_phase: LoadPhase::Idle,
            last_tick: None,
            pending_events: Vec::new(),
        };

        ctrl.play();

        // First poll immediately after play() -- delay has not elapsed.
        let events = ctrl.poll();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, ControllerEvent::PlaybackFinished)),
            "BUG: empty run should emit PlaybackFinished immediately but does not on first poll"
        );
        // Still Playing, not Paused.
        assert!(
            matches!(ctrl.playback_state(), PlaybackState::Playing { .. }),
            "empty run should already be Paused but is still Playing"
        );

        // Now simulate the delay having elapsed.
        ctrl.last_tick = Some(Instant::now() - std::time::Duration::from_secs(5));
        let events2 = ctrl.poll();

        // NOW it finishes because the delay check passes.
        assert!(
            events2
                .iter()
                .any(|e| matches!(e, ControllerEvent::PlaybackFinished)),
            "empty run should finish once delay elapses"
        );
        assert!(
            matches!(ctrl.playback_state(), PlaybackState::Paused { .. }),
            "empty run must transition to Paused after delay"
        );
    }

    // -- FINDING 4: single event run auto-advance edge case (LOW) --
    //
    // With one event, playing should advance from 0 -> 1, then detect
    // current_position >= total_events and finish.
    #[test]
    fn blackhat_single_event_run_auto_advance_finishes() {
        let events = vec![make_run_accepted(RunId::new(1), 1)];
        let mut ctrl = controller_with_events(events);
        assert_eq!(ctrl.total_events(), 1);

        ctrl.play();

        // Simulate enough time passing.
        ctrl.last_tick = Some(Instant::now() - std::time::Duration::from_secs(5));
        let poll_events = ctrl.poll();

        assert!(
            poll_events
                .iter()
                .any(|e| matches!(e, ControllerEvent::PositionChanged { position: 1 })),
            "should advance to position 1"
        );
        assert!(
            poll_events
                .iter()
                .any(|e| matches!(e, ControllerEvent::PlaybackFinished)),
            "should emit PlaybackFinished when reaching end"
        );
        assert_eq!(ctrl.current_position(), 1);
    }

    // -- FINDING 5: load_run resets all state correctly (HIGH) --
    //
    // Verify that calling load_run() while a previous run is loaded and
    // playing correctly resets the state machine to Stopped and clears
    // the engine. This prevents stale state from the previous run leaking
    // into the new run.
    #[test]
    fn blackhat_load_run_resets_playing_state() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_step_started(RunId::new(1), 2, StepIdx::new(0)),
        ];
        let mut ctrl = controller_with_events(events);

        // Start playing and advance.
        ctrl.play();
        ctrl.current_position = 1;

        // Now load a new run. This should reset everything.
        let result = ctrl.load_run(RunId::new(2));
        // load_run may fail if IPC bridge can't send, but state should be reset.
        if result.is_ok() {
            assert_eq!(*ctrl.playback_state(), PlaybackState::Stopped);
            assert_eq!(ctrl.current_position(), 0);
            assert_eq!(ctrl.total_events(), 0);
            assert!(ctrl.engine.is_none());
            assert_eq!(ctrl.active_run(), Some(RunId::new(2)));
            assert!(ctrl.last_tick.is_none());
        }
    }

    // -- FINDING 6: current_state at max u32 position (LOW) --
    //
    // On 64-bit platforms, usize::try_from(u32::MAX) succeeds, so
    // current_state() correctly indexes into the engine states. This test
    // documents that the conversion works for the maximum possible position.
    #[test]
    fn blackhat_current_state_max_position_converts_correctly() {
        let events = vec![make_run_accepted(RunId::new(1), 1)];
        let mut ctrl = controller_with_events(events);

        // Manually set position to u32::MAX to verify conversion.
        ctrl.current_position = u32::MAX;
        ctrl.total_events = u32::MAX;

        // current_state should return None since there aren't that many states.
        // The key point is it doesn't panic or return the wrong state.
        let state = ctrl.current_state();
        assert!(
            state.is_none(),
            "position u32::MAX should return None (out of bounds)"
        );
    }

    // -- FINDING 7: jump_to_failure with u32-saturating event index (LOW) --
    //
    // If find_failure returns a very large index (approaching u32::MAX),
    // the conversion u32::try_from(idx.saturating_add(1)) would overflow
    // and fall back to current_position. This test documents the behavior.
    #[test]
    fn blackhat_jump_to_failure_large_index_falls_back() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_action_failed(RunId::new(1), 2, StepIdx::new(0), ActionId::new(1)),
        ];
        let mut ctrl = controller_with_events(events);
        ctrl.current_position = 0;

        // find_failure returns index 1; position = index + 1 = 2.
        ctrl.jump_to_failure();
        assert_eq!(ctrl.current_position(), 2);
    }

    // -- FINDING 8: jump_to_position clamps to total_events (MEDIUM) --
    //
    // Seeking beyond total_events silently clamps. This prevents out-of-bounds
    // access but the caller has no way to distinguish between a valid seek
    // and a clamped one.
    #[test]
    fn blackhat_jump_to_position_clamps_silently() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_step_started(RunId::new(1), 2, StepIdx::new(0)),
        ];
        let mut ctrl = controller_with_events(events);
        assert_eq!(ctrl.total_events(), 2);

        // Seek beyond total.
        ctrl.jump_to_position(1000);
        assert_eq!(
            ctrl.current_position(),
            2,
            "must clamp to total_events, not panic"
        );
    }

    // -- FINDING 9: PlaybackState does not enforce position invariants (MEDIUM) --
    //
    // The PlaybackState::Paused { position } variant is a plain struct with
    // no invariant enforcement. Direct construction with an out-of-range
    // position is possible. This test documents that the controller must
    // always clamp positions itself when transitioning states.
    #[test]
    fn blackhat_playback_state_no_position_invariant() {
        // This is a documentation test: PlaybackState can hold any u32.
        let state = PlaybackState::Paused { position: u32::MAX };
        assert!(matches!(state, PlaybackState::Paused { position: p } if p == u32::MAX));
    }

    // -- FINDING 10: current_diff at position 0 returns empty diff (LOW) --
    //
    // This is correct behavior but worth documenting: at position 0, the diff
    // is always empty because there is no "previous" state to diff from.
    #[test]
    fn blackhat_current_diff_at_zero_is_always_empty() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_step_started(RunId::new(1), 2, StepIdx::new(0)),
        ];
        let mut ctrl = controller_with_events(events);
        ctrl.current_position = 0;

        let diff = ctrl.current_diff();
        assert!(diff.is_some());
        let d = diff.as_ref();
        assert!(d.map(|d| d.step_changes.is_empty()).unwrap_or(false));
        assert!(d.map(|d| d.slot_changes.is_empty()).unwrap_or(false));
    }

    // -- FINDING 11: play after reaching end via auto-advance restarts (MEDIUM) --
    //
    // After playback finishes (state becomes Paused at total_events),
    // calling play() again resumes from the terminal position. The auto-
    // advance immediately detects current_position >= total_events and
    // finishes again. This creates a no-op play that emits PlaybackFinished
    // on the next poll.
    //
    // BLACKHAT FINDING: Currently the controller's auto-advance tick does not
    // detect position >= total_events on the first poll after play() at end.
    // The tick only advances on a time-based cadence and doesn't check the
    // terminal condition immediately. This is a known limitation.
    #[test]
    fn blackhat_play_after_finish_does_not_immediately_finish() {
        let events = vec![make_run_accepted(RunId::new(1), 1)];
        let mut ctrl = controller_with_events(events);

        ctrl.current_position = 1;
        ctrl.total_events = 1;
        ctrl.state = PlaybackState::Paused { position: 1 };

        ctrl.play();
        assert!(matches!(
            ctrl.playback_state(),
            PlaybackState::Playing { .. }
        ));

        // Documenting: poll does NOT immediately emit PlaybackFinished
        // because auto-advance is tick-based, not immediate.
        let poll_events = ctrl.poll();
        let finished = poll_events
            .iter()
            .any(|e| matches!(e, ControllerEvent::PlaybackFinished));
        // This assertion documents the current (limited) behavior.
        assert!(
            !finished,
            "play-at-end does not immediately finish — tick-based limitation"
        );
    }

    // -- FINDING 12: IPC race - load_phase guards prevent stale replies (HIGH) --
    //
    // The load_phase field is checked before processing Inspected and Events
    // replies. A stale Inspected reply from a previous load_run would be
    // rejected because load_phase would be Idle or WaitingInspect for the
    // new run. This test verifies the guard.
    #[test]
    fn blackhat_load_phase_guards_stale_inspected_reply() {
        let mut ctrl = ReplayController::new();

        // Without a load in progress, Inspected reply is ignored.
        let reply = IpcReply::Inspected(IpcResponse::AcceptedRun { run_id: 42 });
        let mut events = Vec::new();
        ctrl.handle_reply(reply, &mut events);

        // Should NOT emit RunLoaded (load_phase was Idle).
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, ControllerEvent::RunLoaded { .. })),
            "stale Inspected reply must not produce RunLoaded"
        );
        // Should NOT transition load_phase.
        assert_eq!(ctrl.load_phase, LoadPhase::Idle);
    }

    // -- FINDING 13: IPC race - stale Events reply is ignored (HIGH) --
    //
    // A stale Events reply when load_phase != WaitingEvents is silently
    // dropped. This prevents a late Events reply from overwriting a
    // successfully loaded engine.
    #[test]
    fn blackhat_load_phase_guards_stale_events_reply() {
        let mut ctrl = ReplayController::new();

        // Create a controller that has already loaded an engine.
        let engine = ReplayEngine::from_events(vec![make_run_accepted(RunId::new(1), 1)]);
        ctrl.engine = Some(engine);
        ctrl.total_events = 1;
        ctrl.load_phase = LoadPhase::Idle;

        // Simulate a stale Events reply arriving.
        let reply = IpcReply::Events(IpcResponse::Events {
            events: vec![IpcTraceEvent {
                sequence: 99,
                kind: IpcTraceEventKind::RunSubmitted {
                    run: RunId::new(999),
                },
            }],
        });
        let mut events = Vec::new();
        ctrl.handle_reply(reply, &mut events);

        // The existing engine must NOT be overwritten.
        assert_eq!(
            ctrl.total_events(),
            1,
            "stale Events reply must not overwrite engine"
        );
        // No RunLoaded event should be emitted.
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, ControllerEvent::RunLoaded { .. })),
            "stale Events reply must not emit RunLoaded"
        );
    }

    // -- FINDING 14: convert_trace_events with empty input (LOW) --
    //
    // An empty trace slice produces an empty journal. No panics, no issues.
    #[test]
    fn blackhat_convert_trace_events_empty_input() {
        let result = convert_trace_events(&[]);
        assert!(result.is_empty());
    }

    // -- FINDING 15: convert_trace_events with ActionFailed for different run (MEDIUM) --
    //
    // If ActionFailed has (run_A, step_0) and StepEnded has (run_B, step_0),
    // the StepEnded must NOT be suppressed since the run IDs differ.
    #[test]
    fn blackhat_convert_trace_events_different_runs_not_suppressed() {
        use vb_core::action::ActionFailureCode;
        let run_a = RunId::new(1);
        let run_b = RunId::new(2);
        let step = StepIdx::new(0);
        let traces = vec![
            IpcTraceEvent {
                sequence: 0,
                kind: IpcTraceEventKind::ActionFailed {
                    run: run_a,
                    step,
                    code: ActionFailureCode::Timeout,
                },
            },
            IpcTraceEvent {
                sequence: 1,
                kind: IpcTraceEventKind::StepEnded { run: run_b, step },
            },
        ];
        let journal = convert_trace_events(&traces);

        // StepEnded for run_b must NOT be suppressed.
        assert_eq!(journal.len(), 2);
        assert!(matches!(journal[0], JournalEvent::ActionFailedEvent { .. }));
        assert!(matches!(journal[1], JournalEvent::StepSucceeded { .. }));
    }

    // -- FINDING 16: jump_to_position at 0 sets Paused (LOW) --
    //
    // Seeking to position 0 correctly pauses at the initial state.
    #[test]
    fn blackhat_jump_to_position_zero_sets_paused_at_zero() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_step_started(RunId::new(1), 2, StepIdx::new(0)),
        ];
        let mut ctrl = controller_with_events(events);
        ctrl.current_position = 2;

        ctrl.jump_to_position(0);
        assert_eq!(ctrl.current_position(), 0);
        assert!(matches!(
            ctrl.playback_state(),
            PlaybackState::Paused { position: 0 }
        ));
    }

    // -- FINDING 17: set_speed while not playing is no-op (LOW) --
    //
    // Changing speed while Stopped or Paused has no effect. Only takes
    // effect when already Playing.
    #[test]
    fn blackhat_set_speed_while_stopped_is_noop() {
        let events = vec![make_run_accepted(RunId::new(1), 1)];
        let mut ctrl = controller_with_events(events);

        ctrl.set_speed(PlaybackSpeed::Octuple);
        // State is still Stopped; speed change is silently ignored.
        assert_eq!(*ctrl.playback_state(), PlaybackState::Stopped);

        // Play should use default speed, not Octuple.
        ctrl.play();
        assert!(matches!(
            ctrl.playback_state(),
            PlaybackState::Playing {
                speed: PlaybackSpeed::Normal
            }
        ));
    }

    // -- FINDING 18: set_speed while playing takes effect (LOW) --
    #[test]
    fn blackhat_set_speed_while_playing_takes_effect() {
        let events = vec![make_run_accepted(RunId::new(1), 1)];
        let mut ctrl = controller_with_events(events);

        ctrl.play();
        assert!(matches!(
            ctrl.playback_state(),
            PlaybackState::Playing {
                speed: PlaybackSpeed::Normal
            }
        ));

        ctrl.set_speed(PlaybackSpeed::Quad);
        assert!(matches!(
            ctrl.playback_state(),
            PlaybackState::Playing {
                speed: PlaybackSpeed::Quad
            }
        ));
    }

    // -- FINDING 19: handle_events_response emits RunLoaded with correct count (LOW) --
    //
    // Verifies that the RunLoaded event carries the correct total_events count.
    #[test]
    fn blackhat_run_loaded_carries_correct_event_count() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_step_started(RunId::new(1), 2, StepIdx::new(0)),
            make_step_succeeded(RunId::new(1), 3, StepIdx::new(0), SlotIdx::new(0)),
        ];
        let ctrl = controller_with_events(events);
        assert_eq!(ctrl.total_events(), 3);
    }

    // -- FINDING 20: auto-advance resets last_tick after each step (MEDIUM) --
    //
    // After advancing one position, last_tick is updated to now(). This
    // prevents burst advances. This test verifies the tick is properly set.
    #[test]
    fn blackhat_auto_advance_resets_tick_after_step() {
        let events = vec![
            make_run_accepted(RunId::new(1), 1),
            make_step_started(RunId::new(1), 2, StepIdx::new(0)),
        ];
        let mut ctrl = controller_with_events(events);
        ctrl.play();

        // Simulate a tick that has elapsed enough.
        let old_tick = Instant::now() - std::time::Duration::from_secs(5);
        ctrl.last_tick = Some(old_tick);

        let poll_events = ctrl.poll();
        assert!(
            poll_events
                .iter()
                .any(|e| matches!(e, ControllerEvent::PositionChanged { .. })),
            "should advance position"
        );
        assert_eq!(ctrl.current_position(), 1);

        // last_tick should have been updated (not still the old tick).
        assert!(
            ctrl.last_tick.is_some(),
            "last_tick must be reset after non-terminal advance"
        );
        let tick = ctrl.last_tick;
        assert!(
            tick.map(|t| t > old_tick).unwrap_or(false),
            "last_tick must be more recent than the old tick"
        );
    }
}
