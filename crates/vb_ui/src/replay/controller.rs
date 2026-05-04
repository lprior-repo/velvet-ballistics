//! Replay controller bridging the IPC bridge with the Makepad UI.
//!
//! The controller owns the `ReplayEngine` and `IpcBridge`, manages playback
//! state, and provides a `poll()` method that should be called from the
//! Makepad render loop (e.g. `handle_next_frame`).

use std::time::Instant;

use vb_core::WorkflowDigest;
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
                    if let Some(run_id) = self.active_run {
                        self.bridge
                            .send(IpcRequest::ListEvents {
                                run_id,
                                from_sequence: 0,
                            })
                            .ok();
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
            | IpcReply::ShuttingDown => {}
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

        // Convert IPC trace events to journal events.
        let journal_events: Vec<JournalEvent> = trace_events
            .into_iter()
            .filter_map(trace_to_journal)
            .collect();

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
fn trace_to_journal(trace: IpcTraceEvent) -> Option<JournalEvent> {
    let seq = EventSeq::new(trace.sequence);
    match trace.kind {
        IpcTraceEventKind::RunSubmitted { run } => Some(JournalEvent::RunAccepted {
            run,
            seq,
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        }),
        IpcTraceEventKind::StepStarted { run, step } => {
            Some(JournalEvent::StepStarted { run, seq, step })
        }
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
        }),
        IpcTraceEventKind::ActionScheduled { run, step } => Some(JournalEvent::ActionScheduled {
            run,
            seq,
            step,
            // ActionId not present in trace; use a sentinel.
            action: ActionId::new(0),
        }),
        IpcTraceEventKind::ActionCompleted { run, step } => {
            Some(JournalEvent::ActionCompletedEvent {
                run,
                seq,
                step,
                action: ActionId::new(0),
            })
        }
        IpcTraceEventKind::ActionFailed { run, step, .. } => {
            Some(JournalEvent::ActionFailedEvent {
                run,
                seq,
                step,
                action: ActionId::new(0),
            })
        }
        IpcTraceEventKind::AskAnswered { run, step, .. } => {
            Some(JournalEvent::AskAnsweredEvent { run, seq, step })
        }
        IpcTraceEventKind::RunFinished { run } => Some(JournalEvent::RunFinished {
            run,
            seq,
            result: SlotIdx::new(0),
        }),
        IpcTraceEventKind::RunFailed { run } => Some(JournalEvent::RunFailedEvent { run, seq }),
        IpcTraceEventKind::RunCancelled { run } => Some(JournalEvent::RunCancelled { run, seq }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::StepIdx;

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

    // -- trace_to_journal conversion --

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
    fn trace_step_ended_converts_to_step_succeeded() {
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
        }
    }

    fn make_action_failed(run: RunId, seq: u64, step: StepIdx, action: ActionId) -> JournalEvent {
        JournalEvent::ActionFailedEvent {
            run,
            seq: EventSeq::new(seq),
            step,
            action,
        }
    }

    fn make_run_failed(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(seq),
        }
    }

    fn make_run_finished(run: RunId, seq: u64, result: SlotIdx) -> JournalEvent {
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(seq),
            result,
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
}
