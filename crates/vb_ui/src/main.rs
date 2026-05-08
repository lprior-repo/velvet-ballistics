//! vb_ui — Mission Control UI for Velvet Ballistics
//!
//! A Makepad 2.0 application using the Rust Widget trait pattern.

pub mod ipc_bridge;

use makepad_widgets::*;
use vb_ui::app_state::{AppState, Screen};
use vb_ui::ipc_wiring::{IpcAppWiring, WiringError};

app_main!(VbApp);

script_mod! {
    use mod.prelude.widgets_internal.*

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
    #[rust]
    app_state: AppState,
    #[rust]
    ipc_wiring: IpcAppWiring,
    #[rust]
    ipc_clean_cycles: u8,
    #[rust]
    rect: Rect,
}

impl Widget for VbApp {
    #[allow(elided_lifetimes_in_paths)]
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, _scope: &mut Scope) {
        match event.hits_with_capture_overload(cx, self.draw_bg.area(), true) {
            Hit::FingerDown(fe) if fe.is_primary_hit() => {
                cx.set_key_focus(self.draw_bg.area());
            }
            _ => {}
        }

        let wiring_events = self.ipc_wiring.poll(&mut self.app_state);

        let mut has_errors = !wiring_events.errors.is_empty();
        if has_errors {
            self.ipc_clean_cycles = 0;
            if let Some(err) = wiring_events.errors.first() {
                let msg = match err {
                    WiringError::ConnectionFailed(detail) => {
                        format!("IPC connection failed: {detail}")
                    }
                    WiringError::IpcError(detail) => {
                        format!("IPC error: {detail}")
                    }
                };
                self.app_state.last_ipc_error = Some(msg);
            }
        } else if self.app_state.last_ipc_error.is_some() {
            self.ipc_clean_cycles = self.ipc_clean_cycles.saturating_add(1);
            if self.ipc_clean_cycles >= 3 {
                self.app_state.last_ipc_error = None;
                self.ipc_clean_cycles = 0;
                has_errors = true;
            }
        }

        if wiring_events.metrics_updated
            || wiring_events.connection_changed
            || wiring_events.health_checked
            || wiring_events.run_list_updated
            || has_errors
        {
            self.sync_system_state(cx);
        }
        if wiring_events.verification_updated || wiring_events.taint_report_updated {
            self.sync_verify_state(cx);
        }
        if wiring_events.run_accepted
            || wiring_events.run_cancelled
            || wiring_events.events_arrived
            || wiring_events.trace_drained
            || wiring_events.inspected
        {
            if wiring_events.events_arrived {
                let responses = self.ipc_wiring.drain_events();
                self.ingest_timeline_events(&responses);
            } else {
                let _ = self.ipc_wiring.drain_events();
            }
            self.sync_replay_state(cx);
            if wiring_events.inspected {
                let title = self.app_state.screen_title().to_string();
                self.sync_nav(cx, title);
            }
        }
        if wiring_events.workflow_graph_updated {
            self.sync_workflow_state(cx);
        }

        self.handle_nav(cx);
        self.handle_transport(cx);
        self.redraw(cx);
    }

    #[allow(elided_lifetimes_in_paths)]
    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        self.rect = cx.walk_turtle(walk);
        self.draw_background(cx);
        self.draw_header_bar(cx);
        self.draw_nav_tabs(cx);
        self.draw_content(cx);
        DrawStep::done()
    }
}

impl VbApp {
    #[allow(elided_lifetimes_in_paths)]
    fn draw_background(&mut self, cx: &mut Cx2d) {
        self.draw_bg.color = Vec4f {
            x: 0.039,
            y: 0.039,
            z: 0.071,
            w: 1.0,
        };
        self.draw_bg.draw_abs(cx, self.rect);
    }

    #[allow(elided_lifetimes_in_paths)]
    fn draw_header_bar(&mut self, cx: &mut Cx2d) {
        let header_height = 44.0;
        let header_rect = Rect {
            pos: DVec2 {
                x: self.rect.pos.x,
                y: self.rect.pos.y,
            },
            size: DVec2 {
                x: self.rect.size.x,
                y: header_height,
            },
        };
        self.draw_header.color = Vec4f {
            x: 0.071,
            y: 0.078,
            z: 0.122,
            w: 1.0,
        };
        self.draw_header.draw_abs(cx, header_rect);

        let title_rect = Rect {
            pos: DVec2 {
                x: self.rect.pos.x + 16.0,
                y: self.rect.pos.y + 8.0,
            },
            size: DVec2 { x: 40.0, y: 28.0 },
        };
        self.draw_header.color = Vec4f {
            x: 0.0,
            y: 0.96,
            z: 1.0,
            w: 1.0,
        };
        self.draw_header.draw_abs(cx, title_rect);

        let separator_rect = Rect {
            pos: DVec2 {
                x: self.rect.pos.x,
                y: self.rect.pos.y + header_height,
            },
            size: DVec2 {
                x: self.rect.size.x,
                y: 1.0,
            },
        };
        self.draw_header.color = Vec4f {
            x: 0.165,
            y: 0.165,
            z: 0.290,
            w: 1.0,
        };
        self.draw_header.draw_abs(cx, separator_rect);
    }

    #[allow(elided_lifetimes_in_paths)]
    fn draw_nav_tabs(&mut self, cx: &mut Cx2d) {
        let header_height = 45.0;
        let y = self.rect.pos.y + header_height;
        let tab_height = 28.0;

        let tab_x_offsets = [0.0, 80.0, 160.0, 240.0, 330.0];

        for (i, &x_offset) in tab_x_offsets.iter().enumerate() {
            let is_active = match self.app_state.current_screen() {
                Screen::RunReplay => i == 0,
                Screen::Verification => i == 1,
                Screen::SystemOverview => i == 2,
                Screen::WorkflowGraph => i == 3,
                Screen::IncidentConsole => i == 4,
            };

            let (bg_r, bg_g, bg_b) = if is_active {
                (0.10, 0.165, 0.165)
            } else {
                (0.102, 0.102, 0.180)
            };

            let (accent_r, accent_g, accent_b) = match i {
                0 => (0.0, 0.96, 1.0),
                1 => (0.22, 1.0, 0.08),
                2 => (0.18, 0.42, 1.0),
                3 => (0.69, 0.30, 1.0),
                4 => (1.0, 0.03, 0.23),
                _ => (0.5, 0.5, 0.5),
            };

            let tab_rect = Rect {
                pos: DVec2 {
                    x: self.rect.pos.x + x_offset,
                    y,
                },
                size: DVec2 {
                    x: 70.0,
                    y: tab_height,
                },
            };
            self.draw_nav.color = Vec4f {
                x: bg_r,
                y: bg_g,
                z: bg_b,
                w: 1.0,
            };
            self.draw_nav.draw_abs(cx, tab_rect);

            let accent_rect = Rect {
                pos: DVec2 {
                    x: self.rect.pos.x + x_offset,
                    y: y + tab_height - 3.0,
                },
                size: DVec2 { x: 70.0, y: 3.0 },
            };
            self.draw_nav.color = Vec4f {
                x: accent_r,
                y: accent_g,
                z: accent_b,
                w: 1.0,
            };
            self.draw_nav.draw_abs(cx, accent_rect);
        }
    }

    #[allow(elided_lifetimes_in_paths)]
    fn draw_content(&mut self, cx: &mut Cx2d) {
        let content_y = self.rect.pos.y + 73.0;
        let content_rect = Rect {
            pos: DVec2 {
                x: self.rect.pos.x,
                y: content_y,
            },
            size: DVec2 {
                x: self.rect.size.x,
                y: self.rect.size.y - 73.0,
            },
        };

        self.draw_bg.color = Vec4f {
            x: 0.039,
            y: 0.039,
            z: 0.071,
            w: 1.0,
        };
        self.draw_bg.draw_abs(cx, content_rect);

        let panel_rect = Rect {
            pos: DVec2 {
                x: self.rect.pos.x + 20.0,
                y: content_y + 20.0,
            },
            size: DVec2 {
                x: self.rect.size.x - 40.0,
                y: 150.0,
            },
        };
        self.draw_bg.color = Vec4f {
            x: 0.086,
            y: 0.086,
            z: 0.165,
            w: 1.0,
        };
        self.draw_bg.draw_abs(cx, panel_rect);

        let accent_rect = Rect {
            pos: DVec2 {
                x: self.rect.pos.x + 20.0,
                y: content_y + 20.0,
            },
            size: DVec2 { x: 4.0, y: 150.0 },
        };
        let (r, g, b) = match self.app_state.current_screen() {
            Screen::RunReplay => (0.0, 0.96, 1.0),
            Screen::Verification => (0.22, 1.0, 0.08),
            Screen::SystemOverview => (0.18, 0.42, 1.0),
            Screen::WorkflowGraph => (0.69, 0.30, 1.0),
            Screen::IncidentConsole => (1.0, 0.03, 0.23),
        };
        self.draw_bg.color = Vec4f {
            x: r,
            y: g,
            z: b,
            w: 1.0,
        };
        self.draw_bg.draw_abs(cx, accent_rect);
    }

    fn handle_nav(&mut self, _cx: &mut Cx) {}

    fn handle_transport(&mut self, _cx: &mut Cx) {}

    fn sync_nav(&mut self, _cx: &mut Cx, _title: String) {}

    fn sync_replay_state(&mut self, _cx: &mut Cx) {}

    fn ingest_timeline_events(&mut self, _responses: &[vb_ipc::server::IpcResponse]) {}

    fn sync_verify_state(&mut self, _cx: &mut Cx) {}

    fn sync_system_state(&mut self, _cx: &mut Cx) {}

    fn sync_workflow_state(&mut self, _cx: &mut Cx) {}
}

impl AppMain for VbApp {
    fn script_mod(vm: &mut ScriptVm<'_>) -> ScriptValue {
        makepad_widgets::script_mod(vm);
        self::script_mod(vm)
    }

    #[allow(elided_lifetimes_in_paths)]
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        <VbApp as Widget>::handle_event(self, cx, event, &mut Scope::empty());
    }
}
