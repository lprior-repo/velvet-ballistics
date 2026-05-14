//! vb_ui — Mission Control UI for Velvet Ballistics
//!
//! A Makepad 2.0 application using the Rust Widget trait pattern.

#![forbid(unsafe_code)]

mod domain;
mod draw_helpers;
mod event_handlers;

use makepad_widgets::*;
use vb_ui::app_state::AppState;
use vb_ui::ipc_wiring::IpcAppWiring;
use vb_ui::replay::transport::TransportState;
use vb_ui::workflow::WorkflowCanvas;

use crate::domain::IpcCleanCycles;
use crate::draw_helpers::{draw_background, draw_content, draw_header_bar, draw_nav_tabs};
use crate::event_handlers::{
    TransportControlKind, VbAction, handle_keyboard, handle_nav, handle_transport,
    poll_ipc_and_detect_changes,
};

app_main!(VbApp);

script_mod! {
    use mod.prelude.widgets_internal.*

    let state = {
        current_screen: "ExecutionOverview",
        transport_state: "Idle"
    }
    mod.state = state

    mod.widgets.VbAppBase = #(VbApp::register_widget(vm))
    mod.widgets.VbApp = set_type_default() do mod.widgets.VbAppBase{
        width: Fill
        height: Fill
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct VbApp {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[walk]
    walk: Walk,
    #[layout]
    layout: Layout,
    #[redraw]
    #[live]
    draw_bg: DrawColor,
    #[live]
    draw_header: DrawColor,
    #[live]
    draw_nav: DrawColor,
    #[live]
    draw_vector: DrawVector,
    #[live]
    draw_text: DrawText,
    #[rust]
    app_state: AppState,
    #[rust]
    ipc_wiring: IpcAppWiring,
    #[rust]
    ipc_clean_cycles: IpcCleanCycles,
    #[rust]
    workflow_canvas: Option<WorkflowCanvas>,
    #[rust]
    rect: Rect,
}

impl Widget for VbApp {
    #[allow(elided_lifetimes_in_paths)]
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, _scope: &mut Scope) {
        let hit = capture_hit(cx, event, self.draw_bg.area());
        process_ipc_changes(
            cx,
            &mut self.ipc_wiring,
            &mut self.app_state,
            &mut self.ipc_clean_cycles,
        );
        handle_nav(cx, self.uid, &self.rect, &hit);
        handle_transport(cx, self.uid, &self.rect, &hit, &self.app_state);
        handle_keyboard(
            cx,
            self.uid,
            event,
            &self.app_state,
            &mut self.workflow_canvas,
        );
        self.redraw(cx);
    }

    #[allow(elided_lifetimes_in_paths)]
    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        self.rect = cx.walk_turtle(walk);
        draw_background(&mut self.draw_bg, cx, self.rect);
        draw_header_bar(&mut self.draw_header, cx, self.rect);
        draw_nav_tabs(&mut self.draw_nav, cx, self.rect, &self.app_state);
        draw_content(
            &mut self.draw_bg,
            &mut self.draw_vector,
            &mut self.draw_text,
            cx,
            self.rect,
            &self.app_state,
            &self.workflow_canvas,
        );
        DrawStep::done()
    }
}

fn ingest_timeline_events_to_app(
    app_state: &mut AppState,
    responses: &[vb_ipc::server::IpcResponse],
) {
    use vb_ipc::server::IpcResponse;
    use vb_ui::replay::convert_trace_events;

    for response in responses {
        if let IpcResponse::Events { events } = response {
            let journal_events = convert_trace_events(events);
            app_state
                .replay
                .timeline_strip
                .extend_from_journal(&journal_events);
            let new_len = app_state.replay.timeline_strip.events().len();
            app_state.replay.total_events = u32::try_from(new_len).map_or(u32::MAX, |v| v);
        }
    }
}

fn capture_hit(cx: &mut Cx, event: &Event, area: Area) -> Hit {
    let hit = event.hits_with_capture_overload(cx, area, true);
    if matches!(&hit, Hit::FingerDown(fe) if fe.is_primary_hit()) {
        cx.set_key_focus(area);
    }
    hit
}

fn process_ipc_changes(
    _cx: &mut Cx,
    ipc_wiring: &mut IpcAppWiring,
    app_state: &mut AppState,
    ipc_clean_cycles: &mut IpcCleanCycles,
) {
    let changes = poll_ipc_and_detect_changes(ipc_wiring, app_state, ipc_clean_cycles);
    if changes.needs_system_sync() {
        // System sync handled here
    }
    if changes.needs_verify_sync() {
        // Verify sync handled here
    }
    if changes.needs_replay_sync() {
        if changes.events_arrived {
            let responses = ipc_wiring.drain_events();
            ingest_timeline_events_to_app(app_state, &responses);
        } else {
            let _ = ipc_wiring.drain_events();
        }
    }
    if changes.needs_workflow_sync() {
        // Workflow sync handled here
    }
}

impl MatchEvent for VbApp {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        for action in actions.filter_widget_actions(self.uid) {
            if let Some(app_action) = action.action.downcast_ref::<VbAction>() {
                match app_action {
                    VbAction::SwitchScreen(screen) => {
                        self.app_state.switch_screen(*screen);
                        let screen_name = screen.splash_name();
                        script_eval!(cx, {
                            mod.state.current_screen = #(screen_name)
                        });
                    }
                    VbAction::TransportControl(kind) => match kind {
                        TransportControlKind::JumpToStart => {
                            self.app_state.replay.playback_position = 0;
                            self.app_state.replay.transport_state = TransportState::Idle;
                            script_eval!(cx, {
                                mod.state.transport_state = "Idle"
                            });
                        }
                        TransportControlKind::StepBackward => {
                            self.app_state.replay.playback_position =
                                self.app_state.replay.playback_position.saturating_sub(1);
                            self.app_state.replay.transport_state = TransportState::Paused;
                            script_eval!(cx, {
                                mod.state.transport_state = "Paused"
                            });
                        }
                        TransportControlKind::TogglePlayPause => {
                            let is_playing = self.app_state.replay.transport_state.is_playing();
                            self.app_state.replay.transport_state = if is_playing {
                                script_eval!(cx, {
                                    mod.state.transport_state = "Paused"
                                });
                                TransportState::Paused
                            } else {
                                script_eval!(cx, {
                                    mod.state.transport_state = "Playing"
                                });
                                TransportState::Playing { next_tick_at: 0 }
                            };
                        }
                        TransportControlKind::StepForward => {
                            self.app_state.replay.playback_position =
                                self.app_state.replay.playback_position.saturating_add(1);
                            self.app_state.replay.transport_state = TransportState::Paused;
                            script_eval!(cx, {
                                mod.state.transport_state = "Paused"
                            });
                        }
                        TransportControlKind::JumpToEnd => {
                            self.app_state.replay.playback_position =
                                self.app_state.replay.total_events.saturating_sub(1);
                            self.app_state.replay.transport_state = TransportState::Idle;
                            script_eval!(cx, {
                                mod.state.transport_state = "Idle"
                            });
                        }
                    },
                    VbAction::Escape => {
                        self.app_state.show_shortcuts = false;
                        self.app_state.replay.transport_state = TransportState::Idle;
                        self.app_state
                            .switch_screen(vb_ui::app_state::Screen::ExecutionOverview);
                        script_eval!(cx, {
                            mod.state.current_screen = "ExecutionOverview"
                            mod.state.transport_state = "Idle"
                        });
                    }
                    VbAction::ToggleShortcuts => {
                        self.app_state.show_shortcuts = !self.app_state.show_shortcuts;
                    }
                    VbAction::NoOp => {}
                }
            }
        }
    }
}

impl AppMain for VbApp {
    fn script_mod(vm: &mut ScriptVm<'_>) -> ScriptValue {
        makepad_widgets::script_mod(vm);
        self::script_mod(vm)
    }

    #[allow(elided_lifetimes_in_paths)]
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        <VbApp as Widget>::handle_event(self, cx, event, &mut Scope::empty());
    }
}
