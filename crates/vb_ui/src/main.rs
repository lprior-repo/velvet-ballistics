pub mod ipc_bridge;

pub use makepad_widgets;

use makepad_widgets::*;

app_main!(VbApp);

script_mod! {
    use mod.prelude.widgets.*

    startup() do #(VbApp::script_component(vm)){
        ui: Root{
            on_startup: ||{
                ui.main_view.render()
            }
            main_window := Window{
                window.inner_size: vec2(1600, 900)
                window.title: "vb — Mission Control"
                body +: {
                    main_view := View{
                        width: Fill height: Fill
                        flow: Down
                        new_batch: true
                        draw_bg +: {
                            color: #0a0a12
                        }

                        nav_bar := View{
                            width: Fill height: 48
                            flow: Right spacing: 12
                            padding: Inset{left: 16 right: 16 top: 8 bottom: 8}
                            new_batch: true
                            draw_bg +: {color: #12121f}

                            app_title := Label{
                                text: "vb"
                                draw_text +: {
                                    color: #00f5ff
                                    text_style +: {font_size: 20}
                                }
                            }
                            nav_sep := Label{
                                text: " — "
                                draw_text +: {color: #555577}
                            }
                            nav_subtitle := Label{
                                text: "Mission Control"
                                draw_text +: {color: #x2e2e2e}
                            }
                        }

                        content := View{
                            width: Fill height: Fill
                            flow: Right
                            new_batch: true
                            draw_bg +: {color: #0a0a12}

                            sidebar := View{
                                width: 240 height: Fill
                                flow: Down spacing: 8
                                padding: 12
                                new_batch: true
                                draw_bg +: {color: #12121f}

                                sidebar_title := Label{
                                    text: "RUNS"
                                    draw_text +: {
                                        color: #00f5ff
                                        text_style +: {font_size: 12}
                                    }
                                }

                                run_item_1 := View{
                                    width: Fill height: Fit
                                    flow: Down spacing: 2
                                    padding: 8
                                    new_batch: true
                                    draw_bg +: {color: #16162a border_radius: 4.0}

                                    run_id_1 := Label{
                                        text: "Run 8172"
                                        draw_text +: {
                                            color: #39ff14
                                            text_style +: {font_size: 13}
                                        }
                                    }
                                    run_status_1 := Label{
                                        text: "● succeeded"
                                        draw_text +: {
                                            color: #39ff14
                                            text_style +: {font_size: 11}
                                        }
                                    }
                                }
                            }

                            canvas_area := View{
                                width: Fill height: Fill
                                flow: Down
                                align: Align{x: 0.5 y: 0.5}
                                new_batch: true
                                draw_bg +: {color: #0a0a12}

                                placeholder := Label{
                                    text: "Replay Theater"
                                    draw_text +: {
                                        color: #2a2a4a
                                        text_style +: {font_size: 32}
                                    }
                                }
                            }

                            inspector := View{
                                width: 320 height: Fill
                                flow: Down spacing: 8
                                padding: 12
                                new_batch: true
                                draw_bg +: {color: #12121f}

                                inspector_title := Label{
                                    text: "INSPECTOR"
                                    draw_text +: {
                                        color: #00f5ff
                                        text_style +: {font_size: 12}
                                    }
                                }
                                inspector_hint := Label{
                                    text: "Select a run to inspect"
                                    draw_text +: {
                                        color: #555577
                                        text_style +: {font_size: 12}
                                    }
                                }
                            }
                        }

                        timeline_bar := View{
                            width: Fill height: 80
                            flow: Right spacing: 8
                            padding: 12
                            new_batch: true
                            draw_bg +: {color: #12121f}

                            timeline_hint := Label{
                                text: "TIMELINE — event playback"
                                draw_text +: {
                                    color: #2a2a4a
                                    text_style +: {font_size: 12}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Script, ScriptHook)]
pub struct VbApp {
    #[live]
    ui: WidgetRef,
}

impl MatchEvent for VbApp {
    fn handle_actions(&mut self, _cx: &mut Cx, _actions: &Actions) {
        // Future: handle widget actions for run selection, timeline scrub, etc.
    }
}

impl AppMain for VbApp {
    fn script_mod(vm: &mut ScriptVm<'_>) -> ScriptValue {
        crate::makepad_widgets::script_mod(vm);
        self::script_mod(vm)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}
