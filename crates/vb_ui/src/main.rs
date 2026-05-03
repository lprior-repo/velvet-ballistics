pub mod app_state;
pub mod ipc_bridge;

pub use makepad_widgets;

use makepad_widgets::*;
use vb_ui::app_state::{AppState, ReplayData, Screen};

app_main!(VbApp);

script_mod! {
    use mod.prelude.widgets.*

    // ── Cyberpunk color constants ──────────────────────────────
    let canvas_bg     = #0a0a12
    let panel_bg      = #12121f
    let panel_bg_alt  = #1a1a2e
    let card_bg       = #16162a
    let border_color  = #2a2a4a
    let neon_cyan     = #00f5ff
    let neon_green    = #39ff14
    let neon_red      = #ff073a
    let neon_blue     = #2d6bff
    let neon_yellow   = #ffe600
    let neon_orange   = #ff6b00
    let neon_magenta  = #ff00ff
    let neon_purple   = #b14dff
    let text_primary  = #e8e8ff
    let text_secondary= #8888aa
    let text_dim      = #555577

    // ── Reusable card template ─────────────────────────────────
    let InfoCard = View{
        width: Fill height: Fit
        flow: Down spacing: 4
        padding: Inset{left: 10 right: 10 top: 8 bottom: 8}
        new_batch: true
        draw_bg +: {
            color: #16162a
            border_radius: 4.0
        }
    }

    // ── Workflow node card template ────────────────────────────
    let NodeCard = View{
        width: 140 height: Fit
        flow: Down spacing: 2
        padding: Inset{left: 8 right: 8 top: 6 bottom: 6}
        new_batch: true
        draw_bg +: {
            color: #16162a
            border_radius: 4.0
        }
    }

    // ── Event chip template ────────────────────────────────────
    let EventChip = View{
        width: Fit height: Fit
        flow: Right spacing: 4
        padding: Inset{left: 8 right: 8 top: 4 bottom: 4}
        new_batch: true
        draw_bg +: {
            color: #16162a
            border_radius: 3.0
        }
    }

    // ── Transport button helper ────────────────────────────────
    let TransportBtn = ButtonFlat{
        draw_bg +: {color: #2a2a4a}
        draw_text +: {color: #e8e8ff text_style +: {font_size: 14}}
    }

    // ── Jump chip helper ───────────────────────────────────────
    let JumpChip = ButtonFlatter{
        draw_bg +: {color: #1a1a2e border_radius: 3.0}
        draw_text +: {color: #00f5ff text_style +: {font_size: 10}}
    }

    // ── Certificate panel template ─────────────────────────────
    let CertPanel = View{
        width: Fill height: Fit
        flow: Down spacing: 4
        padding: Inset{left: 10 right: 10 top: 8 bottom: 8}
        new_batch: true
        draw_bg +: {
            color: #16162a
            border_radius: 4.0
        }
    }

    // ── Shard card template ────────────────────────────────────
    let ShardCard = View{
        width: Fill height: Fit
        flow: Down spacing: 3
        padding: Inset{left: 10 right: 10 top: 6 bottom: 6}
        new_batch: true
        draw_bg +: {
            color: #16162a
            border_radius: 4.0
        }
    }

    // ── Alert card template ────────────────────────────────────
    let AlertCard = View{
        width: Fill height: Fit
        flow: Right spacing: 8
        padding: Inset{left: 10 right: 10 top: 6 bottom: 6}
        align: Align{y: 0.5}
        new_batch: true
        draw_bg +: {
            color: #16162a
            border_radius: 4.0
        }
    }

    // ── Tab button for sub-panel tabs ──────────────────────────
    let SubTab = ButtonFlatter{
        draw_bg +: {color: #1a1a2e border_radius: 3.0}
        draw_text +: {color: #8888aa text_style +: {font_size: 10}}
    }

    // ── Active sub-tab ─────────────────────────────────────────
    let SubTabActive = ButtonFlatter{
        draw_bg +: {color: #2a2a4a border_radius: 3.0}
        draw_text +: {color: #00f5ff text_style +: {font_size: 10}}
    }

    // ── Main app ───────────────────────────────────────────────
    startup() do #(VbApp::script_component(vm)){
        ui: Root{
            on_startup: ||{
                ui.main_view.render()
            }
            main_window := Window{
                window.inner_size: vec2(1400, 900)
                window.title: "vb — Mission Control"
                body +: {
                    main_view := View{
                        width: Fill height: Fill
                        flow: Down
                        new_batch: true
                        draw_bg +: {
                            color: #0a0a12
                        }

                        // ════════════════════════════════════════
                        // TOP BAR (always visible)
                        // ════════════════════════════════════════
                        top_bar := View{
                            width: Fill height: 44
                            flow: Right spacing: 12
                            padding: Inset{left: 16 right: 16 top: 8 bottom: 8}
                            align: Align{y: 0.5}
                            new_batch: true
                            draw_bg +: {color: #12121f}

                            app_title := Label{
                                text: "vb"
                                draw_text +: {
                                    color: #00f5ff
                                    text_style +: {font_size: 18}
                                }
                            }
                            sep1 := Label{
                                text: " — "
                                draw_text +: {color: #555577}
                            }
                            page_title := Label{
                                text: "Replay Theater"
                                draw_text +: {
                                    color: #e8e8ff
                                    text_style +: {font_size: 14}
                                }
                            }
                            Filler{}

                            nav_tabs := View{
                                width: Fit height: Fit
                                flow: Right spacing: 2
                                align: Align{y: 0.5}

                                nav_replay := ButtonFlat{
                                    text: "Replay"
                                    draw_bg +: {color: #1a2a2a border_radius: 3.0}
                                    draw_text +: {color: #00f5ff text_style +: {font_size: 11}}
                                }
                                nav_verify := ButtonFlat{
                                    text: "Verify"
                                    draw_bg +: {color: #1a1a2e border_radius: 3.0}
                                    draw_text +: {color: #39ff14 text_style +: {font_size: 11}}
                                }
                                nav_system := ButtonFlat{
                                    text: "System"
                                    draw_bg +: {color: #1a1a2e border_radius: 3.0}
                                    draw_text +: {color: #2d6bff text_style +: {font_size: 11}}
                                }
                                nav_workflow := ButtonFlat{
                                    text: "Workflow"
                                    draw_bg +: {color: #1a1a2e border_radius: 3.0}
                                    draw_text +: {color: #b14dff text_style +: {font_size: 11}}
                                }
                                nav_incident := ButtonFlat{
                                    text: "Incidents"
                                    draw_bg +: {color: #1a1a2e border_radius: 3.0}
                                    draw_text +: {color: #ff073a text_style +: {font_size: 11}}
                                }
                            }

                            run_badge := View{
                                width: Fit height: Fit
                                flow: Right spacing: 6
                                padding: Inset{left: 10 right: 10 top: 3 bottom: 3}
                                new_batch: true
                                draw_bg +: {color: #1a1a2e border_radius: 3.0}
                                run_label := Label{
                                    text: "Run:"
                                    draw_text +: {color: #555577 text_style +: {font_size: 11}}
                                }
                                run_id := Label{
                                    text: "8172"
                                    draw_text +: {color: #00f5ff text_style +: {font_size: 11}}
                                }
                            }
                            wf_badge := View{
                                width: Fit height: Fit
                                flow: Right spacing: 6
                                padding: Inset{left: 10 right: 10 top: 3 bottom: 3}
                                new_batch: true
                                draw_bg +: {color: #1a1a2e border_radius: 3.0}
                                wf_label := Label{
                                    text: "Workflow:"
                                    draw_text +: {color: #555577 text_style +: {font_size: 11}}
                                }
                                wf_name := Label{
                                    text: "issue-triage"
                                    draw_text +: {color: #00f5ff text_style +: {font_size: 11}}
                                }
                            }
                        }

                        // ════════════════════════════════════════
                        // SCREEN SWITCHING VIA PAGE FLIP
                        // ════════════════════════════════════════
                        screens := PageFlip{
                            width: Fill height: Fill
                            active_page: replay_page

                            // ──────────────────────────────────
                            // SCREEN 1: REPLAY THEATER
                            // ──────────────────────────────────
                            replay_page := View{
                                width: Fill height: Fill
                                flow: Down

                                content_area := View{
                                    width: Fill height: Fill
                                    flow: Right

                                    // LEFT: Workflow Graph
                                    graph_panel := View{
                                        width: Fill height: Fill
                                        flow: Down spacing: 6
                                        padding: 12
                                        new_batch: true
                                        draw_bg +: {color: #0a0a12}

                                        graph_header := View{
                                            width: Fill height: Fit
                                            flow: Right spacing: 8
                                            align: Align{y: 0.5}
                                            graph_title := Label{
                                                text: "WORKFLOW GRAPH"
                                                draw_text +: {color: #00f5ff text_style +: {font_size: 11}}
                                            }
                                            Filler{}
                                            graph_hint := Label{
                                                text: "6 nodes"
                                                draw_text +: {color: #555577 text_style +: {font_size: 10}}
                                            }
                                        }

                                        graph_canvas := ScrollXYView{
                                            width: Fill height: Fill
                                            flow: Down spacing: 8
                                            padding: 4

                                            node_row1 := View{
                                                width: Fit height: Fit
                                                flow: Right spacing: 10
                                                align: Align{y: 0.5}

                                                node_setconst := NodeCard{
                                                    draw_bg +: {color: #0d1a0d border_radius: 4.0}
                                                    node_name := Label{text: "SetConst" draw_text +: {color: #39ff14 text_style +: {font_size: 11}}}
                                                    node_badge := Label{text: "succeeded" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                                }
                                                arrow1 := Label{text: "->" draw_text +: {color: #2a2a4a text_style +: {font_size: 12}}}
                                                node_do := NodeCard{
                                                    draw_bg +: {color: #0d1a0d border_radius: 4.0}
                                                    node_name := Label{text: "Do" draw_text +: {color: #39ff14 text_style +: {font_size: 11}}}
                                                    node_badge := Label{text: "github.issue.create" draw_text +: {color: #ff6b00 text_style +: {font_size: 9}}}
                                                    node_state := Label{text: "succeeded" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                                }
                                                arrow2 := Label{text: "->" draw_text +: {color: #2a2a4a text_style +: {font_size: 12}}}
                                                node_choose := NodeCard{
                                                    draw_bg +: {color: #0d1a0d border_radius: 4.0}
                                                    node_name := Label{text: "Choose" draw_text +: {color: #b14dff text_style +: {font_size: 11}}}
                                                    node_badge := Label{text: "succeeded" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                                }
                                            }

                                            node_row2 := View{
                                                width: Fit height: Fit
                                                flow: Right spacing: 10
                                                align: Align{y: 0.5}
                                                margin: Inset{left: 40}
                                                node_foreach := NodeCard{
                                                    draw_bg +: {color: #0d1a0d border_radius: 4.0}
                                                    node_name := Label{text: "ForEach" draw_text +: {color: #2d6bff text_style +: {font_size: 11}}}
                                                    node_badge := Label{text: "3 iterations" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                                    node_state := Label{text: "succeeded" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                                }
                                                arrow3 := Label{text: "->" draw_text +: {color: #2a2a4a text_style +: {font_size: 12}}}
                                                node_do_ext := NodeCard{
                                                    draw_bg +: {color: #1a0d00 border_radius: 4.0}
                                                    node_name := Label{text: "Do" draw_text +: {color: #ff6b00 text_style +: {font_size: 11}}}
                                                    node_badge := Label{text: "slack.notify" draw_text +: {color: #ff6b00 text_style +: {font_size: 9}}}
                                                    node_state := Label{text: "succeeded" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                                }
                                                arrow4 := Label{text: "->" draw_text +: {color: #2a2a4a text_style +: {font_size: 12}}}
                                                node_finish := NodeCard{
                                                    draw_bg +: {color: #0d1a0d border_radius: 4.0}
                                                    node_name := Label{text: "Finish" draw_text +: {color: #39ff14 text_style +: {font_size: 11}}}
                                                    node_badge := Label{text: "completed" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                                }
                                            }

                                            taint_row := View{
                                                width: Fit height: Fit
                                                flow: Right spacing: 8
                                                margin: Inset{top: 6}
                                                align: Align{y: 0.5}
                                                taint_dot := Label{text: "*" draw_text +: {color: #ff00ff text_style +: {font_size: 14}}}
                                                taint_label := Label{text: "taint path: slot 12 via Do/slack.notify" draw_text +: {color: #ff00ff text_style +: {font_size: 10}}}
                                            }
                                        }
                                    }

                                    vr1 := View{width: 1 height: Fill draw_bg +: {color: #2a2a4a}}

                                    // RIGHT: Detail Inspector
                                    inspector_panel := View{
                                        width: 380 height: Fill
                                        flow: Down spacing: 6
                                        padding: 12
                                        new_batch: true
                                        draw_bg +: {color: #12121f}

                                        inspector_header := Label{
                                            text: "DETAIL INSPECTOR"
                                            draw_text +: {color: #00f5ff text_style +: {font_size: 11}}
                                        }

                                        step_card := InfoCard{
                                            step_header := Label{text: "Step Inspector" draw_text +: {color: #e8e8ff text_style +: {font_size: 12}}}
                                            Hr{draw_bg +: {color: #2a2a4a}}
                                            step_fields := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 3
                                                sf1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    sf1k := Label{text: "Step:" width: 80 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    sf1v := Label{text: "github.issue.create" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                                }
                                                sf2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    sf2k := Label{text: "Kind:" width: 80 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    sf2v := Label{text: "Do" draw_text +: {color: #ff6b00 text_style +: {font_size: 10}}}
                                                }
                                                sf3 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    sf3k := Label{text: "State:" width: 80 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    sf3v := Label{text: "Succeeded" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                sf4 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    sf4k := Label{text: "ActionId:" width: 80 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    sf4v := Label{text: "17" draw_text +: {color: #00f5ff text_style +: {font_size: 10}}}
                                                }
                                            }
                                        }

                                        ticket_card := InfoCard{
                                            ticket_header := Label{text: "Action Ticket" draw_text +: {color: #e8e8ff text_style +: {font_size: 12}}}
                                            Hr{draw_bg +: {color: #2a2a4a}}
                                            ticket_fields := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 3
                                                tf1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    tf1k := Label{text: "Ticket:" width: 80 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    tf1v := Label{text: "#42" draw_text +: {color: #00f5ff text_style +: {font_size: 10}}}
                                                }
                                                tf2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    tf2k := Label{text: "Replay-safe:" width: 80 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    tf2v := Label{text: "YES" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                            }
                                        }

                                        slots_card := InfoCard{
                                            slots_header := Label{text: "Slot Diffs" draw_text +: {color: #e8e8ff text_style +: {font_size: 12}}}
                                            Hr{draw_bg +: {color: #2a2a4a}}
                                            slots_fields := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 3
                                                sd1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    sd1k := Label{text: "S12:" width: 40 draw_text +: {color: #ff00ff text_style +: {font_size: 10}}}
                                                    sd1arrow := Label{text: "null" draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    sd1sep := Label{text: "->" draw_text +: {color: #2a2a4a text_style +: {font_size: 10}}}
                                                    sd1v := Label{text: "ObjectId(0x3f7a..)" draw_text +: {color: #00f5ff text_style +: {font_size: 10}}}
                                                }
                                                sd2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    sd2k := Label{text: "S8:" width: 40 draw_text +: {color: #b14dff text_style +: {font_size: 10}}}
                                                    sd2arrow := Label{text: "null" draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    sd2sep := Label{text: "->" draw_text +: {color: #2a2a4a text_style +: {font_size: 10}}}
                                                    sd2v := Label{text: "Issue { title, body }" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                                }
                                                sd3 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    sd3k := Label{text: "S19:" width: 40 draw_text +: {color: #2d6bff text_style +: {font_size: 10}}}
                                                    sd3arrow := Label{text: "null" draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    sd3sep := Label{text: "->" draw_text +: {color: #2a2a4a text_style +: {font_size: 10}}}
                                                    sd3v := Label{text: "true" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                            }
                                        }

                                        legend := View{
                                            width: Fill height: Fit
                                            flow: Down spacing: 3
                                            margin: Inset{top: 4}
                                            legend_title := Label{text: "STATE LEGEND" draw_text +: {color: #555577 text_style +: {font_size: 9}}}
                                            leg_row1 := View{width: Fill height: Fit flow: Right spacing: 12
                                                lg1 := Label{text: "Succeeded" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                                lg2 := Label{text: "Running" draw_text +: {color: #00f5ff text_style +: {font_size: 9}}}
                                                lg3 := Label{text: "Failed" draw_text +: {color: #ff073a text_style +: {font_size: 9}}}
                                            }
                                            leg_row2 := View{width: Fill height: Fit flow: Right spacing: 12
                                                lg4 := Label{text: "Waiting" draw_text +: {color: #2d6bff text_style +: {font_size: 9}}}
                                                lg5 := Label{text: "Asking" draw_text +: {color: #ffe600 text_style +: {font_size: 9}}}
                                                lg6 := Label{text: "Taint" draw_text +: {color: #ff00ff text_style +: {font_size: 9}}}
                                            }
                                        }
                                    }
                                }

                                // Transport + Event Strip
                                bottom_bar := View{
                                    width: Fill height: Fit
                                    flow: Down
                                    new_batch: true
                                    draw_bg +: {color: #12121f}

                                    sep_line := View{width: Fill height: 1 draw_bg +: {color: #2a2a4a}}

                                    transport_row := View{
                                        width: Fill height: Fit
                                        flow: Right spacing: 6
                                        padding: Inset{left: 12 right: 12 top: 8 bottom: 4}
                                        align: Align{y: 0.5}

                                        btn_start := TransportBtn{text: "|<"}
                                        btn_prev := TransportBtn{text: "<"}
                                        btn_play := TransportBtn{text: ">"}
                                        btn_next := TransportBtn{text: ">>"}
                                        btn_end := TransportBtn{text: ">|"}

                                        speed_badge := View{
                                            width: Fit height: Fit
                                            padding: Inset{left: 6 right: 6 top: 2 bottom: 2}
                                            new_batch: true
                                            draw_bg +: {color: #1a1a2e border_radius: 3.0}
                                            speed_label := Label{text: "1x" draw_text +: {color: #8888aa text_style +: {font_size: 10}}}
                                        }
                                        transport_sep := View{width: 1 height: 20 margin: Inset{left: 6 right: 6} draw_bg +: {color: #2a2a4a}}
                                        jump_failure := JumpChip{text: "jump: failure"}
                                        jump_action := JumpChip{text: "action"}
                                        jump_done := JumpChip{text: "done"}
                                        Filler{}
                                        event_count := Label{text: "12 events" draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                    }

                                    event_strip := ScrollXView{
                                        width: Fill height: 44
                                        flow: Right spacing: 4
                                        padding: Inset{left: 12 right: 12 top: 4 bottom: 8}
                                        align: Align{y: 0.5}

                                        pos_dot := Label{text: "--*--" draw_text +: {color: #00f5ff text_style +: {font_size: 10}}}
                                        ev1 := EventChip{draw_bg +: {color: #0a1a1a border_radius: 3.0}
                                            ev_dot := Label{text: "*" draw_text +: {color: #00f5ff text_style +: {font_size: 10}}}
                                            ev_label := Label{text: "RunAccepted" draw_text +: {color: #00f5ff text_style +: {font_size: 9}}}
                                        }
                                        ev2 := EventChip{draw_bg +: {color: #0a1a0d border_radius: 3.0}
                                            ev_dot := Label{text: "*" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                            ev_label := Label{text: "Step:0" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                        }
                                        ev3 := EventChip{draw_bg +: {color: #0d0d1a border_radius: 3.0}
                                            ev_dot := Label{text: "*" draw_text +: {color: #2d6bff text_style +: {font_size: 10}}}
                                            ev_label := Label{text: "ActionScheduled" draw_text +: {color: #2d6bff text_style +: {font_size: 9}}}
                                        }
                                        ev4 := EventChip{draw_bg +: {color: #1a0d00 border_radius: 3.0}
                                            ev_dot := Label{text: "*" draw_text +: {color: #ff6b00 text_style +: {font_size: 10}}}
                                            ev_label := Label{text: "github.issue.create" draw_text +: {color: #ff6b00 text_style +: {font_size: 9}}}
                                        }
                                        ev5 := EventChip{draw_bg +: {color: #0a1a0d border_radius: 3.0}
                                            ev_dot := Label{text: "*" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                            ev_label := Label{text: "Succeeded" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                        }
                                        ev6 := EventChip{draw_bg +: {color: #12061a border_radius: 3.0}
                                            ev_dot := Label{text: "*" draw_text +: {color: #b14dff text_style +: {font_size: 10}}}
                                            ev_label := Label{text: "Choose[0]" draw_text +: {color: #b14dff text_style +: {font_size: 9}}}
                                        }
                                        ev7 := EventChip{draw_bg +: {color: #0d0d1a border_radius: 3.0}
                                            ev_dot := Label{text: "*" draw_text +: {color: #2d6bff text_style +: {font_size: 10}}}
                                            ev_label := Label{text: "ForEach[2/3]" draw_text +: {color: #2d6bff text_style +: {font_size: 9}}}
                                        }
                                        ev8 := EventChip{draw_bg +: {color: #1a001a border_radius: 3.0}
                                            ev_dot := Label{text: "!" draw_text +: {color: #ff00ff text_style +: {font_size: 10}}}
                                            ev_label := Label{text: "Taint(S12)" draw_text +: {color: #ff00ff text_style +: {font_size: 9}}}
                                        }
                                        ev9 := EventChip{draw_bg +: {color: #0a1a0d border_radius: 3.0}
                                            ev_dot := Label{text: "*" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                            ev_label := Label{text: "Completed" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                        }
                                        ev10 := EventChip{draw_bg +: {color: #0a1a1a border_radius: 3.0}
                                            ev_dot := Label{text: "*" draw_text +: {color: #00f5ff text_style +: {font_size: 10}}}
                                            ev_label := Label{text: "RunFinished" draw_text +: {color: #00f5ff text_style +: {font_size: 9}}}
                                        }
                                    }
                                }
                            }

                            // ──────────────────────────────────
                            // SCREEN 2: VERIFICATION / CERTIFICATES
                            // ──────────────────────────────────
                            verify_page := View{
                                width: Fill height: Fill
                                flow: Down
                                padding: 12
                                spacing: 8

                                verify_header := View{
                                    width: Fill height: Fit
                                    flow: Right spacing: 8
                                    align: Align{y: 0.5}
                                    verify_title := Label{
                                        text: "VERIFICATION REPORT"
                                        draw_text +: {color: #39ff14 text_style +: {font_size: 13}}
                                    }
                                    Filler{}
                                    verify_status := Label{
                                        text: "PASS (4/6 panels clean)"
                                        draw_text +: {color: #39ff14 text_style +: {font_size: 11}}
                                    }
                                }

                                verify_content := View{
                                    width: Fill height: Fill
                                    flow: Right spacing: 8

                                    // Left column: certificate panels
                                    verify_left := ScrollYView{
                                        width: Fill height: Fill
                                        flow: Down spacing: 6
                                        padding: Inset{right: 4}

                                        // Panel 1: Structure
                                        cert_structure := CertPanel{
                                            cs_header := View{
                                                width: Fill height: Fit
                                                flow: Right spacing: 8
                                                align: Align{y: 0.5}
                                                cs_title := Label{text: "Structure" draw_text +: {color: #e8e8ff text_style +: {font_size: 12}}}
                                                Filler{}
                                                cs_badge := Label{text: "PASS" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                            }
                                            Hr{draw_bg +: {color: #2a2a4a}}
                                            cs_fields := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 3
                                                cs1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cs1k := Label{text: "Unreachable steps:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cs1v := Label{text: "0" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                cs2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cs2k := Label{text: "Invalid transitions:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cs2v := Label{text: "0" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                cs3 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cs3k := Label{text: "Incorrect joins:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cs3v := Label{text: "0" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                cs4 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cs4k := Label{text: "Cycle analysis:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cs4v := Label{text: "no cycles" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                            }
                                        }

                                        // Panel 2: Boundedness
                                        cert_bounded := CertPanel{
                                            cb_header := View{
                                                width: Fill height: Fit
                                                flow: Right spacing: 8
                                                align: Align{y: 0.5}
                                                cb_title := Label{text: "Boundedness" draw_text +: {color: #e8e8ff text_style +: {font_size: 12}}}
                                                Filler{}
                                                cb_badge := Label{text: "PASS" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                            }
                                            Hr{draw_bg +: {color: #2a2a4a}}
                                            cb_fields := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 3
                                                cb1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cb1k := Label{text: "Max transitions:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cb1v := Label{text: "8 (limit: 128)" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                cb2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cb2k := Label{text: "Max retries:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cb2v := Label{text: "3 (limit: 16)" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                cb3 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cb3k := Label{text: "Fan-out:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cb3v := Label{text: "4 (limit: 64)" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                cb4 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cb4k := Label{text: "Timer waits:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cb4v := Label{text: "0 (limit: 32)" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                            }
                                        }

                                        // Panel 3: Resources
                                        cert_resources := CertPanel{
                                            cr_header := View{
                                                width: Fill height: Fit
                                                flow: Right spacing: 8
                                                align: Align{y: 0.5}
                                                cr_title := Label{text: "Resources" draw_text +: {color: #e8e8ff text_style +: {font_size: 12}}}
                                                Filler{}
                                                cr_badge := Label{text: "PASS" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                            }
                                            Hr{draw_bg +: {color: #2a2a4a}}
                                            cr_fields := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 3
                                                cr1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cr1k := Label{text: "Slot count:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cr1v := Label{text: "24 (limit: 256)" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                cr2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cr2k := Label{text: "Max frame size:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cr2v := Label{text: "1.2 KiB (limit: 4 KiB)" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                cr3 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cr3k := Label{text: "Max action payload:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cr3v := Label{text: "2.1 KiB (limit: 64 KiB)" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                            }
                                        }

                                        // Panel 4: Taint/Secret Flow
                                        cert_taint := CertPanel{
                                            draw_bg +: {color: #1a0d1a border_radius: 4.0}
                                            ct_header := View{
                                                width: Fill height: Fit
                                                flow: Right spacing: 8
                                                align: Align{y: 0.5}
                                                ct_title := Label{text: "Taint / Secret Flow" draw_text +: {color: #e8e8ff text_style +: {font_size: 12}}}
                                                Filler{}
                                                ct_badge := Label{text: "WARNING" draw_text +: {color: #ffe600 text_style +: {font_size: 10}}}
                                            }
                                            Hr{draw_bg +: {color: #2a2a4a}}
                                            ct_fields := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 3
                                                ct1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    ct1k := Label{text: "Secret sources:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    ct1v := Label{text: "2 (S3, S7)" draw_text +: {color: #ff00ff text_style +: {font_size: 10}}}
                                                }
                                                ct2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    ct2k := Label{text: "Tainted paths:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    ct2v := Label{text: "S3 -> S12 -> finish" draw_text +: {color: #ff00ff text_style +: {font_size: 10}}}
                                                }
                                                ct3 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    ct3k := Label{text: "Safe outputs:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    ct3v := Label{text: "5/6 finish signals clean" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                ct4 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    ct4k := Label{text: "Leak risk:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    ct4v := Label{text: "1 path to review" draw_text +: {color: #ffe600 text_style +: {font_size: 10}}}
                                                }
                                            }
                                        }

                                        // Panel 5: Action Policy
                                        cert_action := CertPanel{
                                            ca_header := View{
                                                width: Fill height: Fit
                                                flow: Right spacing: 8
                                                align: Align{y: 0.5}
                                                ca_title := Label{text: "Action Policy" draw_text +: {color: #e8e8ff text_style +: {font_size: 12}}}
                                                Filler{}
                                                ca_badge := Label{text: "PASS" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                            }
                                            Hr{draw_bg +: {color: #2a2a4a}}
                                            ca_fields := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 3
                                                ca1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    ca1k := Label{text: "Idempotency:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    ca1v := Label{text: "2/2 actions classified" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                ca2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    ca2k := Label{text: "Timeout coverage:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    ca2v := Label{text: "2/2 Do nodes covered" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                ca3 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    ca3k := Label{text: "Strict-durability:" width: 140 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    ca3v := Label{text: "eligible" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                            }
                                        }

                                        // Panel 6: Replay/Durability
                                        cert_durability := CertPanel{
                                            draw_bg +: {color: #1a0d00 border_radius: 4.0}
                                            cd_header := View{
                                                width: Fill height: Fit
                                                flow: Right spacing: 8
                                                align: Align{y: 0.5}
                                                cd_title := Label{text: "Replay / Durability" draw_text +: {color: #e8e8ff text_style +: {font_size: 12}}}
                                                Filler{}
                                                cd_badge := Label{text: "HIGH RISK" draw_text +: {color: #ff073a text_style +: {font_size: 10}}}
                                            }
                                            Hr{draw_bg +: {color: #2a2a4a}}
                                            cd_fields := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 3
                                                cd1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cd1k := Label{text: "Journal-before-dispatch:" width: 160 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cd1v := Label{text: "PASS" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                cd2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cd2k := Label{text: "Completion-before-mutation:" width: 160 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cd2v := Label{text: "PASS" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                cd3 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cd3k := Label{text: "Reconciliation risk:" width: 160 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cd3v := Label{text: "1 potential divergence" draw_text +: {color: #ff073a text_style +: {font_size: 10}}}
                                                }
                                                cd4 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cd4k := Label{text: "Timeout coverage:" width: 160 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cd4v := Label{text: "FAIL - Do/slack.notify missing" draw_text +: {color: #ff073a text_style +: {font_size: 10}}}
                                                }
                                            }
                                        }
                                    }

                                    vr_verify := View{width: 1 height: Fill draw_bg +: {color: #2a2a4a}}

                                    // Right column: summary + overlay controls
                                    verify_right := View{
                                        width: 320 height: Fill
                                        flow: Down spacing: 6
                                        padding: 8
                                        new_batch: true
                                        draw_bg +: {color: #12121f}

                                        verify_summary_title := Label{
                                            text: "SUMMARY"
                                            draw_text +: {color: #39ff14 text_style +: {font_size: 11}}
                                        }

                                        summary_card := InfoCard{
                                            sum_fields := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 4
                                                su1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    su1k := Label{text: "Total checks:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    su1v := Label{text: "18" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                                }
                                                su2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    su2k := Label{text: "Passed:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    su2v := Label{text: "16" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                su3 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    su3k := Label{text: "Warnings:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    su3v := Label{text: "1" draw_text +: {color: #ffe600 text_style +: {font_size: 10}}}
                                                }
                                                su4 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    su4k := Label{text: "Failures:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    su4v := Label{text: "1" draw_text +: {color: #ff073a text_style +: {font_size: 10}}}
                                                }
                                                su5 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    su5k := Label{text: "Worst risk:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    su5v := Label{text: "HIGH RISK" draw_text +: {color: #ff073a text_style +: {font_size: 10}}}
                                                }
                                            }
                                        }

                                        verify_actions_title := Label{
                                            text: "ACTIONS"
                                            draw_text +: {color: #39ff14 text_style +: {font_size: 11}}
                                            margin: Inset{top: 8}
                                        }
                                        btn_reverify := ButtonFlat{
                                            text: "Re-verify"
                                            draw_bg +: {color: #2a2a4a border_radius: 3.0}
                                            draw_text +: {color: #39ff14 text_style +: {font_size: 11}}
                                        }
                                        btn_export_cert := ButtonFlatter{
                                            text: "Export Certificate"
                                            draw_bg +: {color: #1a1a2e border_radius: 3.0}
                                            draw_text +: {color: #8888aa text_style +: {font_size: 10}}
                                            margin: Inset{top: 4}
                                        }
                                    }
                                }
                            }

                            // ──────────────────────────────────
                            // SCREEN 3: SYSTEM OVERVIEW / WORLD MAP
                            // ──────────────────────────────────
                            system_page := View{
                                width: Fill height: Fill
                                flow: Down

                                sys_content := View{
                                    width: Fill height: Fill
                                    flow: Right spacing: 8
                                    padding: 12

                                    // LEFT: Topology / Shard health
                                    sys_left := View{
                                        width: 280 height: Fill
                                        flow: Down spacing: 6
                                        padding: 8
                                        new_batch: true
                                        draw_bg +: {color: #12121f}

                                        sys_topo_title := Label{
                                            text: "TOPOLOGY"
                                            draw_text +: {color: #2d6bff text_style +: {font_size: 11}}
                                        }

                                        shard_0 := ShardCard{
                                            draw_bg +: {color: #0d1a0d border_radius: 4.0}
                                            sh_header := View{width: Fill height: Fit flow: Right
                                                sh_name := Label{text: "Shard 0" draw_text +: {color: #39ff14 text_style +: {font_size: 11}}}
                                                Filler{}
                                                sh_status := Label{text: "HEALTHY" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                            }
                                            sh_fields := View{width: Fill height: Fit flow: Down spacing: 2
                                                sh1 := Label{text: "active: 12 runs  queue: 3  frame: 78%" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                                sh2 := Label{text: "trace fill: 34%  throughput: 142 steps/s" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                            }
                                        }
                                        shard_1 := ShardCard{
                                            draw_bg +: {color: #0d1a0d border_radius: 4.0}
                                            sh_header := View{width: Fill height: Fit flow: Right
                                                sh_name := Label{text: "Shard 1" draw_text +: {color: #39ff14 text_style +: {font_size: 11}}}
                                                Filler{}
                                                sh_status := Label{text: "HEALTHY" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                            }
                                            sh_fields := View{width: Fill height: Fit flow: Down spacing: 2
                                                sh1 := Label{text: "active: 8 runs  queue: 1  frame: 45%" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                                sh2 := Label{text: "trace fill: 12%  throughput: 98 steps/s" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                            }
                                        }
                                        shard_2 := ShardCard{
                                            draw_bg +: {color: #1a1a0d border_radius: 4.0}
                                            sh_header := View{width: Fill height: Fit flow: Right
                                                sh_name := Label{text: "Shard 2" draw_text +: {color: #ffe600 text_style +: {font_size: 11}}}
                                                Filler{}
                                                sh_status := Label{text: "DEGRADED" draw_text +: {color: #ffe600 text_style +: {font_size: 9}}}
                                            }
                                            sh_fields := View{width: Fill height: Fit flow: Down spacing: 2
                                                sh1 := Label{text: "active: 24 runs  queue: 18  frame: 92%" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                                sh2 := Label{text: "trace fill: 78%  throughput: 34 steps/s" draw_text +: {color: #ffe600 text_style +: {font_size: 9}}}
                                            }
                                        }
                                        shard_3 := ShardCard{
                                            draw_bg +: {color: #0d1a0d border_radius: 4.0}
                                            sh_header := View{width: Fill height: Fit flow: Right
                                                sh_name := Label{text: "Shard 3" draw_text +: {color: #39ff14 text_style +: {font_size: 11}}}
                                                Filler{}
                                                sh_status := Label{text: "HEALTHY" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                            }
                                            sh_fields := View{width: Fill height: Fit flow: Down spacing: 2
                                                sh1 := Label{text: "active: 6 runs  queue: 0  frame: 32%" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                                sh2 := Label{text: "trace fill: 8%  throughput: 210 steps/s" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                            }
                                        }
                                    }

                                    vr_sys := View{width: 1 height: Fill draw_bg +: {color: #2a2a4a}}

                                    // CENTRE: Activity Lanes
                                    sys_center := View{
                                        width: Fill height: Fill
                                        flow: Down spacing: 6
                                        padding: 8
                                        new_batch: true
                                        draw_bg +: {color: #0a0a12}

                                        sys_lanes_title := View{
                                            width: Fill height: Fit
                                            flow: Right spacing: 8
                                            align: Align{y: 0.5}
                                            lanes_label := Label{
                                                text: "ACTIVITY LANES"
                                                draw_text +: {color: #2d6bff text_style +: {font_size: 11}}
                                            }
                                            Filler{}
                                            lanes_hint := Label{
                                                text: "50 active runs across 4 shards"
                                                draw_text +: {color: #555577 text_style +: {font_size: 10}}
                                            }
                                        }

                                        lanes_canvas := ScrollYView{
                                            width: Fill height: Fill
                                            flow: Down spacing: 4
                                            padding: 4

                                            // Lane 0: Shard 0
                                            lane_0 := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 2
                                                lane_0_label := Label{text: "Shard 0 — 12 runs" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                                lane_0_bar := View{
                                                    width: Fill height: 8
                                                    flow: Right spacing: 1
                                                    l0s1 := View{width: 60 height: 8 draw_bg +: {color: #39ff14}}
                                                    l0s2 := View{width: 40 height: 8 draw_bg +: {color: #2d6bff}}
                                                    l0s3 := View{width: 30 height: 8 draw_bg +: {color: #ffe600}}
                                                    l0s4 := View{width: 70 height: 8 draw_bg +: {color: #39ff14}}
                                                    l0s5 := View{width: 50 height: 8 draw_bg +: {color: #00f5ff}}
                                                }
                                            }
                                            // Lane 1: Shard 1
                                            lane_1 := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 2
                                                lane_1_label := Label{text: "Shard 1 — 8 runs" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                                lane_1_bar := View{
                                                    width: Fill height: 8
                                                    flow: Right spacing: 1
                                                    l1s1 := View{width: 45 height: 8 draw_bg +: {color: #39ff14}}
                                                    l1s2 := View{width: 55 height: 8 draw_bg +: {color: #2d6bff}}
                                                    l1s3 := View{width: 35 height: 8 draw_bg +: {color: #39ff14}}
                                                }
                                            }
                                            // Lane 2: Shard 2 (degraded)
                                            lane_2 := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 2
                                                lane_2_label := Label{text: "Shard 2 — 24 runs (DEGRADED)" draw_text +: {color: #ffe600 text_style +: {font_size: 9}}}
                                                lane_2_bar := View{
                                                    width: Fill height: 8
                                                    flow: Right spacing: 1
                                                    l2s1 := View{width: 25 height: 8 draw_bg +: {color: #ff073a}}
                                                    l2s2 := View{width: 20 height: 8 draw_bg +: {color: #ffe600}}
                                                    l2s3 := View{width: 15 height: 8 draw_bg +: {color: #ffe600}}
                                                    l2s4 := View{width: 30 height: 8 draw_bg +: {color: #ff073a}}
                                                    l2s5 := View{width: 18 height: 8 draw_bg +: {color: #2d6bff}}
                                                    l2s6 := View{width: 22 height: 8 draw_bg +: {color: #ffe600}}
                                                    l2s7 := View{width: 35 height: 8 draw_bg +: {color: #ff073a}}
                                                    l2s8 := View{width: 28 height: 8 draw_bg +: {color: #ffe600}}
                                                }
                                            }
                                            // Lane 3: Shard 3
                                            lane_3 := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 2
                                                lane_3_label := Label{text: "Shard 3 — 6 runs" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                                lane_3_bar := View{
                                                    width: Fill height: 8
                                                    flow: Right spacing: 1
                                                    l3s1 := View{width: 70 height: 8 draw_bg +: {color: #39ff14}}
                                                    l3s2 := View{width: 40 height: 8 draw_bg +: {color: #00f5ff}}
                                                }
                                            }

                                            lane_legend := View{
                                                width: Fill height: Fit
                                                flow: Right spacing: 16
                                                margin: Inset{top: 6}
                                                ll1 := Label{text: "* succeeded" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                                ll2 := Label{text: "* running" draw_text +: {color: #00f5ff text_style +: {font_size: 9}}}
                                                ll3 := Label{text: "* waiting" draw_text +: {color: #2d6bff text_style +: {font_size: 9}}}
                                                ll4 := Label{text: "* retrying" draw_text +: {color: #ffe600 text_style +: {font_size: 9}}}
                                                ll5 := Label{text: "! blocked" draw_text +: {color: #ff073a text_style +: {font_size: 9}}}
                                            }
                                        }
                                    }

                                    vr_sys2 := View{width: 1 height: Fill draw_bg +: {color: #2a2a4a}}

                                    // RIGHT: Alerts & pressure
                                    sys_right := View{
                                        width: 300 height: Fill
                                        flow: Down spacing: 6
                                        padding: 8
                                        new_batch: true
                                        draw_bg +: {color: #12121f}

                                        sys_alerts_title := Label{
                                            text: "ALERTS & PRESSURE"
                                            draw_text +: {color: #2d6bff text_style +: {font_size: 11}}
                                        }

                                        alert_1 := AlertCard{
                                            draw_bg +: {color: #1a0d00 border_radius: 4.0}
                                            alert_dot := Label{text: "!" draw_text +: {color: #ff073a text_style +: {font_size: 12}}}
                                            alert_body := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 2
                                                alert_title := Label{text: "Shard 2 queue pressure" draw_text +: {color: #ff073a text_style +: {font_size: 10}}}
                                                alert_detail := Label{text: "18 items in ready queue, 92% frame pool" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                            }
                                        }
                                        alert_2 := AlertCard{
                                            draw_bg +: {color: #1a0d00 border_radius: 4.0}
                                            alert_dot := Label{text: "!" draw_text +: {color: #ffe600 text_style +: {font_size: 12}}}
                                            alert_body := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 2
                                                alert_title := Label{text: "Replay divergence detected" draw_text +: {color: #ffe600 text_style +: {font_size: 10}}}
                                                alert_detail := Label{text: "Run #8144 step 3 diverged from journal" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                            }
                                        }
                                        alert_3 := AlertCard{
                                            draw_bg +: {color: #1a0d1a border_radius: 4.0}
                                            alert_dot := Label{text: "!" draw_text +: {color: #ff00ff text_style +: {font_size: 12}}}
                                            alert_body := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 2
                                                alert_title := Label{text: "Blocked reconciliation" draw_text +: {color: #ff00ff text_style +: {font_size: 10}}}
                                                alert_detail := Label{text: "Shard 2 has 3 runs awaiting reconciliation" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                            }
                                        }
                                        alert_4 := AlertCard{
                                            draw_bg +: {color: #0a1a0d border_radius: 4.0}
                                            alert_dot := Label{text: "*" draw_text +: {color: #39ff14 text_style +: {font_size: 12}}}
                                            alert_body := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 2
                                                alert_title := Label{text: "Action completed: github.issue.create" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                alert_detail := Label{text: "Run #8172 step 1, action #17" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                            }
                                        }
                                    }
                                }

                                // Bottom: Event ticker
                                sys_ticker := View{
                                    width: Fill height: 36
                                    flow: Right spacing: 6
                                    padding: Inset{left: 12 right: 12 top: 6 bottom: 6}
                                    align: Align{y: 0.5}
                                    new_batch: true
                                    draw_bg +: {color: #12121f}

                                    ticker_label := Label{
                                        text: "EVENTS:"
                                        draw_text +: {color: #555577 text_style +: {font_size: 9}}
                                    }
                                    ticker_scroll := ScrollXView{
                                        width: Fill height: Fit
                                        flow: Right spacing: 6
                                        align: Align{y: 0.5}
                                        tk1 := Label{text: "[14:32:01] RunAccepted #8172" draw_text +: {color: #00f5ff text_style +: {font_size: 9}}}
                                        tk2 := Label{text: "[14:32:01] StepStarted:0 #8172" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                        tk3 := Label{text: "[14:32:02] ActionScheduled #8172" draw_text +: {color: #2d6bff text_style +: {font_size: 9}}}
                                        tk4 := Label{text: "[14:32:03] RunAccepted #8144" draw_text +: {color: #00f5ff text_style +: {font_size: 9}}}
                                        tk5 := Label{text: "[14:32:04] TaintDetected S12 #8172" draw_text +: {color: #ff00ff text_style +: {font_size: 9}}}
                                    }
                                }
                            }

                            // ──────────────────────────────────
                            // SCREEN 4: WORKFLOW GRAPH / AUTHORING
                            // ──────────────────────────────────
                            workflow_page := View{
                                width: Fill height: Fill
                                flow: Right

                                // Main canvas area
                                wf_canvas := View{
                                    width: Fill height: Fill
                                    flow: Down spacing: 6
                                    padding: 12
                                    new_batch: true
                                    draw_bg +: {color: #0a0a12}

                                    wf_header := View{
                                        width: Fill height: Fit
                                        flow: Right spacing: 8
                                        align: Align{y: 0.5}
                                        wf_title := Label{
                                            text: "WORKFLOW: issue-triage"
                                            draw_text +: {color: #b14dff text_style +: {font_size: 13}}
                                        }
                                        Filler{}
                                        wf_digest := Label{
                                            text: "digest: 0x7f3a..c291"
                                            draw_text +: {color: #555577 text_style +: {font_size: 10}}
                                        }
                                    }

                                    wf_graph := ScrollXYView{
                                        width: Fill height: Fill
                                        flow: Down spacing: 12
                                        padding: 8

                                        wf_row1 := View{
                                            width: Fit height: Fit
                                            flow: Right spacing: 16
                                            align: Align{y: 0.5}
                                            wf_n1 := NodeCard{
                                                draw_bg +: {color: #16162a border_radius: 4.0}
                                                node_name := Label{text: "SetConst" draw_text +: {color: #b14dff text_style +: {font_size: 11}}}
                                                node_detail := Label{text: "step: 0 | slots: S0-S3" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                                node_badge := Label{text: "YAML source: line 4-12" draw_text +: {color: #555577 text_style +: {font_size: 8}}}
                                            }
                                            wf_arr1 := Label{text: "-->" draw_text +: {color: #2a2a4a text_style +: {font_size: 12}}}
                                            wf_n2 := NodeCard{
                                                draw_bg +: {color: #16162a border_radius: 4.0}
                                                node_name := Label{text: "Do" draw_text +: {color: #ff6b00 text_style +: {font_size: 11}}}
                                                node_detail := Label{text: "step: 1 | action: github.issue.create" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                                node_badge := Label{text: "retry: 3x backoff | timeout: 30s" draw_text +: {color: #555577 text_style +: {font_size: 8}}}
                                            }
                                            wf_arr2 := Label{text: "-->" draw_text +: {color: #2a2a4a text_style +: {font_size: 12}}}
                                            wf_n3 := NodeCard{
                                                draw_bg +: {color: #16162a border_radius: 4.0}
                                                node_name := Label{text: "Choose" draw_text +: {color: #b14dff text_style +: {font_size: 11}}}
                                                node_detail := Label{text: "step: 2 | branch on S4" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                                node_badge := Label{text: "branches: 2 (true/false)" draw_text +: {color: #555577 text_style +: {font_size: 8}}}
                                            }
                                        }
                                        wf_row2 := View{
                                            width: Fit height: Fit
                                            flow: Right spacing: 16
                                            align: Align{y: 0.5}
                                            margin: Inset{left: 60}
                                            wf_n4 := NodeCard{
                                                draw_bg +: {color: #16162a border_radius: 4.0}
                                                node_name := Label{text: "ForEach" draw_text +: {color: #2d6bff text_style +: {font_size: 11}}}
                                                node_detail := Label{text: "step: 3 | iterate S8" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                                node_badge := Label{text: "fan-out: 3 items" draw_text +: {color: #555577 text_style +: {font_size: 8}}}
                                            }
                                            wf_arr3 := Label{text: "-->" draw_text +: {color: #2a2a4a text_style +: {font_size: 12}}}
                                            wf_n5 := NodeCard{
                                                draw_bg +: {color: #16162a border_radius: 4.0}
                                                node_name := Label{text: "Do" draw_text +: {color: #ff6b00 text_style +: {font_size: 11}}}
                                                node_detail := Label{text: "step: 4 | action: slack.notify" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                                node_badge := Label{text: "retry: 2x fixed | timeout: 10s" draw_text +: {color: #555577 text_style +: {font_size: 8}}}
                                            }
                                            wf_arr4 := Label{text: "-->" draw_text +: {color: #2a2a4a text_style +: {font_size: 12}}}
                                            wf_n6 := NodeCard{
                                                draw_bg +: {color: #16162a border_radius: 4.0}
                                                node_name := Label{text: "Finish" draw_text +: {color: #39ff14 text_style +: {font_size: 11}}}
                                                node_detail := Label{text: "step: 5 | result: S19" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                                node_badge := Label{text: "taint: clean" draw_text +: {color: #39ff14 text_style +: {font_size: 8}}}
                                            }
                                        }
                                    }
                                }

                                vr_wf := View{width: 1 height: Fill draw_bg +: {color: #2a2a4a}}

                                // Inspector pane
                                wf_inspector := View{
                                    width: 380 height: Fill
                                    flow: Down spacing: 6
                                    padding: 12
                                    new_batch: true
                                    draw_bg +: {color: #12121f}

                                    wf_insp_title := Label{
                                        text: "NODE INSPECTOR"
                                        draw_text +: {color: #b14dff text_style +: {font_size: 11}}
                                    }

                                    wf_node_card := InfoCard{
                                        wf_nc_header := Label{text: "Do — github.issue.create" draw_text +: {color: #e8e8ff text_style +: {font_size: 12}}}
                                        Hr{draw_bg +: {color: #2a2a4a}}
                                        wf_nc_fields := View{
                                            width: Fill height: Fit
                                            flow: Down spacing: 3
                                            wnf1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                wnf1k := Label{text: "Step ID:" width: 100 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                wnf1v := Label{text: "1" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                            }
                                            wnf2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                wnf2k := Label{text: "Primitive:" width: 100 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                wnf2v := Label{text: "Do" draw_text +: {color: #ff6b00 text_style +: {font_size: 10}}}
                                            }
                                            wnf3 := View{width: Fill height: Fit flow: Right spacing: 6
                                                wnf3k := Label{text: "Retry policy:" width: 100 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                wnf3v := Label{text: "3x exponential backoff" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                            }
                                            wnf4 := View{width: Fill height: Fit flow: Right spacing: 6
                                                wnf4k := Label{text: "Timeout:" width: 100 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                wnf4v := Label{text: "30s" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                            }
                                            wnf5 := View{width: Fill height: Fit flow: Right spacing: 6
                                                wnf5k := Label{text: "Taint:" width: 100 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                wnf5v := Label{text: "clean" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                            }
                                            wnf6 := View{width: Fill height: Fit flow: Right spacing: 6
                                                wnf6k := Label{text: "Input slots:" width: 100 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                wnf6v := Label{text: "S0-S3" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                            }
                                            wnf7 := View{width: Fill height: Fit flow: Right spacing: 6
                                                wnf7k := Label{text: "Output slots:" width: 100 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                wnf7v := Label{text: "S8 (issue object)" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                            }
                                            wnf8 := View{width: Fill height: Fit flow: Right spacing: 6
                                                wnf8k := Label{text: "Last run:" width: 100 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                wnf8v := Label{text: "succeeded (attempt 1/3)" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                            }
                                        }
                                    }

                                    wf_yaml_title := Label{
                                        text: "YAML SOURCE"
                                        draw_text +: {color: #b14dff text_style +: {font_size: 11}}
                                        margin: Inset{top: 8}
                                    }
                                    wf_yaml_card := InfoCard{
                                        wf_yaml := View{
                                            width: Fill height: Fit
                                            flow: Down spacing: 2
                                            yl1 := Label{text: "- do:" draw_text +: {color: #8888aa text_style +: {font_size: 10}}}
                                            yl2 := Label{text: "    action: github.issue.create" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                            yl3 := Label{text: "    inputs:" draw_text +: {color: #8888aa text_style +: {font_size: 10}}}
                                            yl4 := Label{text: "      title: {$.slots.S0}" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                            yl5 := Label{text: "      body: {$.slots.S1}" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                            yl6 := Label{text: "    retry:" draw_text +: {color: #8888aa text_style +: {font_size: 10}}}
                                            yl7 := Label{text: "      max_attempts: 3" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                            yl8 := Label{text: "      backoff: exponential" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                        }
                                    }
                                }
                            }

                            // ──────────────────────────────────
                            // SCREEN 5: INCIDENT / FAILURE CONSOLE
                            // ──────────────────────────────────
                            incident_page := View{
                                width: Fill height: Fill
                                flow: Right

                                // Left: Incident list
                                inc_list := View{
                                    width: 320 height: Fill
                                    flow: Down spacing: 6
                                    padding: 12
                                    new_batch: true
                                    draw_bg +: {color: #12121f}

                                    inc_list_title := Label{
                                        text: "ACTIVE INCIDENTS"
                                        draw_text +: {color: #ff073a text_style +: {font_size: 11}}
                                    }

                                    inc_1 := InfoCard{
                                        draw_bg +: {color: #1a0d00 border_radius: 4.0}
                                        inc1_header := View{width: Fill height: Fit flow: Right spacing: 8 align: Align{y: 0.5}
                                            inc1_sev := Label{text: "CRITICAL" draw_text +: {color: #ff073a text_style +: {font_size: 10}}}
                                            Filler{}
                                            inc1_type := Label{text: "ActionFailure" draw_text +: {color: #ff6b00 text_style +: {font_size: 9}}}
                                        }
                                        Hr{draw_bg +: {color: #2a2a4a}}
                                        inc1_body := View{width: Fill height: Fit flow: Down spacing: 2
                                            inc1_code := Label{text: "ActionCode::ExternalTimeout" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                            inc1_step := Label{text: "Step: Do/slack.notify (#4)" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                            inc1_run := Label{text: "Run: #8144 | Workflow: issue-triage" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                            inc1_safety := View{width: Fill height: Fit flow: Right spacing: 6
                                                inc1_rl := Label{text: "Replay-safe:" draw_text +: {color: #555577 text_style +: {font_size: 9}}}
                                                inc1_rv := Label{text: "UNKNOWN" draw_text +: {color: #ffe600 text_style +: {font_size: 9}}}
                                            }
                                            inc1_cert := View{width: Fill height: Fit flow: Right spacing: 6
                                                inc1_cl := Label{text: "Side-effect:" draw_text +: {color: #555577 text_style +: {font_size: 9}}}
                                                inc1_cv := Label{text: "POSSIBLE" draw_text +: {color: #ff073a text_style +: {font_size: 9}}}
                                            }
                                        }
                                    }

                                    inc_2 := InfoCard{
                                        draw_bg +: {color: #1a0d1a border_radius: 4.0}
                                        inc2_header := View{width: Fill height: Fit flow: Right spacing: 8 align: Align{y: 0.5}
                                            inc2_sev := Label{text: "WARNING" draw_text +: {color: #ff00ff text_style +: {font_size: 10}}}
                                            Filler{}
                                            inc2_type := Label{text: "SecretLeak" draw_text +: {color: #ff00ff text_style +: {font_size: 9}}}
                                        }
                                        Hr{draw_bg +: {color: #2a2a4a}}
                                        inc2_body := View{width: Fill height: Fit flow: Down spacing: 2
                                            inc2_code := Label{text: "Taint leak in finish signal" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                            inc2_step := Label{text: "Step: Finish (#5)" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                            inc2_run := Label{text: "Run: #8172 | Workflow: issue-triage" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                        }
                                    }

                                    inc_3 := InfoCard{
                                        draw_bg +: {color: #1a1a0d border_radius: 4.0}
                                        inc3_header := View{width: Fill height: Fit flow: Right spacing: 8 align: Align{y: 0.5}
                                            inc3_sev := Label{text: "WARNING" draw_text +: {color: #ffe600 text_style +: {font_size: 10}}}
                                            Filler{}
                                            inc3_type := Label{text: "ReplayDivergence" draw_text +: {color: #ffe600 text_style +: {font_size: 9}}}
                                        }
                                        Hr{draw_bg +: {color: #2a2a4a}}
                                        inc3_body := View{width: Fill height: Fit flow: Down spacing: 2
                                            inc3_code := Label{text: "Slot S12 diverged at event 7" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                            inc3_step := Label{text: "Run: #8144 | Shard 2" draw_text +: {color: #8888aa text_style +: {font_size: 9}}}
                                        }
                                    }

                                    inc_summary := View{
                                        width: Fill height: Fit
                                        flow: Right spacing: 12
                                        margin: Inset{top: 8}
                                        inc_sum1 := Label{text: "3 active" draw_text +: {color: #ff073a text_style +: {font_size: 10}}}
                                        inc_sum2 := Label{text: "1 critical" draw_text +: {color: #ff073a text_style +: {font_size: 10}}}
                                        inc_sum3 := Label{text: "2 warnings" draw_text +: {color: #ffe600 text_style +: {font_size: 10}}}
                                    }
                                }

                                vr_inc := View{width: 1 height: Fill draw_bg +: {color: #2a2a4a}}

                                // Right: Detail panel with tabs
                                inc_detail := View{
                                    width: Fill height: Fill
                                    flow: Down spacing: 6
                                    padding: 12
                                    new_batch: true
                                    draw_bg +: {color: #0a0a12}

                                    inc_detail_header := View{
                                        width: Fill height: Fit
                                        flow: Right spacing: 8
                                        align: Align{y: 0.5}
                                        inc_detail_title := Label{
                                            text: "INCIDENT #1 — ExternalTimeout"
                                            draw_text +: {color: #ff073a text_style +: {font_size: 13}}
                                        }
                                        Filler{}
                                        inc_dismiss := ButtonFlatter{
                                            text: "Dismiss"
                                            draw_bg +: {color: #1a1a2e border_radius: 3.0}
                                            draw_text +: {color: #8888aa text_style +: {font_size: 10}}
                                        }
                                    }

                                    // Sub-tabs: Cause | Timeline | State Diff | Replay | Repair
                                    inc_tabs := View{
                                        width: Fill height: Fit
                                        flow: Right spacing: 2
                                        inc_tab_cause := SubTabActive{text: "Cause"}
                                        inc_tab_timeline := SubTab{text: "Timeline"}
                                        inc_tab_state := SubTab{text: "State Diff"}
                                        inc_tab_replay := SubTab{text: "Replay"}
                                        inc_tab_repair := SubTab{text: "Repair"}
                                    }

                                    // Cause panel content
                                    inc_cause := ScrollYView{
                                        width: Fill height: Fill
                                        flow: Down spacing: 6
                                        padding: 4

                                        cause_card := InfoCard{
                                            cause_header := Label{text: "Failure Analysis" draw_text +: {color: #e8e8ff text_style +: {font_size: 12}}}
                                            Hr{draw_bg +: {color: #2a2a4a}}
                                            cause_fields := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 4
                                                cf1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cf1k := Label{text: "Error code:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cf1v := Label{text: "ExternalTimeout" draw_text +: {color: #ff073a text_style +: {font_size: 10}}}
                                                }
                                                cf2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cf2k := Label{text: "Step:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cf2v := Label{text: "Do/slack.notify (step 4)" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                                }
                                                cf3 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cf3k := Label{text: "Attempt:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cf3v := Label{text: "3/3 (max retries exhausted)" draw_text +: {color: #ff073a text_style +: {font_size: 10}}}
                                                }
                                                cf4 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cf4k := Label{text: "Timeout config:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cf4v := Label{text: "10s (action took >10s)" draw_text +: {color: #ffe600 text_style +: {font_size: 10}}}
                                                }
                                                cf5 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    cf5k := Label{text: "Side-effect certainty:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    cf5v := Label{text: "POSSIBLE" draw_text +: {color: #ff073a text_style +: {font_size: 10}}}
                                                }
                                            }
                                        }

                                        repair_card := InfoCard{
                                            repair_header := Label{text: "Repair Suggestions" draw_text +: {color: #39ff14 text_style +: {font_size: 12}}}
                                            Hr{draw_bg +: {color: #2a2a4a}}
                                            repair_fields := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 4
                                                rp1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    rp1k := Label{text: "Kind:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    rp1v := Label{text: "IncreaseTimeout" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                rp2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    rp2k := Label{text: "Action:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    rp2v := Label{text: "Increase timeout to 30s for slack.notify" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                                }
                                                rp3 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    rp3k := Label{text: "Kind:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    rp3v := Label{text: "AddRetryBackoff" draw_text +: {color: #39ff14 text_style +: {font_size: 10}}}
                                                }
                                                rp4 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    rp4k := Label{text: "Action:" width: 120 draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                    rp4v := Label{text: "Add exponential backoff between retries" draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                                }
                                            }
                                        }

                                        slot_diff_card := InfoCard{
                                            slot_header := Label{text: "Slot State Before Failure" draw_text +: {color: #e8e8ff text_style +: {font_size: 12}}}
                                            Hr{draw_bg +: {color: #2a2a4a}}
                                            slot_fields := View{
                                                width: Fill height: Fit
                                                flow: Down spacing: 3
                                                sl1 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    sl1k := Label{text: "S8:" width: 40 draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}}
                                                    sl1v := Label{text: "Issue { title: \"Bug #421\", body: \"...\" }" draw_text +: {color: #00f5ff text_style +: {font_size: 10}}}
                                                }
                                                sl2 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    sl2k := Label{text: "S12:" width: 40 draw_text +: {color: #ff00ff text_style +: {font_size: 10}}}
                                                    sl2v := Label{text: "ObjectId(0x3f7a..) [TAINTED]" draw_text +: {color: #ff00ff text_style +: {font_size: 10}}}
                                                }
                                                sl3 := View{width: Fill height: Fit flow: Right spacing: 6
                                                    sl3k := Label{text: "S19:" width: 40 draw_text +: {color: #8888aa text_style +: {font_size: 10}}}
                                                    sl3v := Label{text: "<empty>" draw_text +: {color: #555577 text_style +: {font_size: 10}}}
                                                }
                                            }
                                        }
                                    }
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
    #[rust]
    app_state: AppState,
}

impl MatchEvent for VbApp {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // Screen navigation — switches PageFlip active_page
        if self.ui.button(cx, ids!(nav_replay)).clicked(actions) {
            self.app_state
                .switch_screen(Screen::RunReplay);
            self.sync_nav(cx, String::from("Replay Theater"), String::from("#00f5ff"));
            self.sync_replay_state(cx);
        }
        if self.ui.button(cx, ids!(nav_verify)).clicked(actions) {
            self.app_state
                .switch_screen(Screen::Verification);
            self.sync_nav(cx, String::from("Verification"), String::from("#39ff14"));
            self.sync_verify_state(cx);
        }
        if self.ui.button(cx, ids!(nav_system)).clicked(actions) {
            self.app_state
                .switch_screen(Screen::SystemOverview);
            self.sync_nav(cx, String::from("System Overview"), String::from("#2d6bff"));
            self.sync_system_state(cx);
        }
        if self.ui.button(cx, ids!(nav_workflow)).clicked(actions) {
            self.app_state
                .switch_screen(Screen::WorkflowGraph);
            self.sync_nav(cx, String::from("Workflow Graph"), String::from("#b14dff"));
            self.sync_workflow_state(cx);
        }
        if self.ui.button(cx, ids!(nav_incident)).clicked(actions) {
            self.app_state
                .switch_screen(Screen::IncidentConsole);
            self.sync_nav(cx, String::from("Incident Console"), String::from("#ff073a"));
            self.sync_incident_state(cx);
        }

        // Transport controls (Replay Theater)
        if self.ui.button(cx, ids!(btn_start)).clicked(actions) {
            self.app_state.replay.playback_position = 0;
            self.sync_replay_state(cx);
        }
        if self.ui.button(cx, ids!(btn_prev)).clicked(actions) {
            self.app_state.replay.playback_position =
                self.app_state.replay.playback_position.saturating_sub(1);
            self.sync_replay_state(cx);
        }
        if self.ui.button(cx, ids!(btn_play)).clicked(actions) {
            self.app_state.replay.is_playing = !self.app_state.replay.is_playing;
            self.sync_replay_state(cx);
        }
        if self.ui.button(cx, ids!(btn_next)).clicked(actions) {
            self.app_state.replay.playback_position =
                self.app_state.replay.playback_position.saturating_add(1);
            if self.app_state.replay.playback_position > self.app_state.replay.total_events {
                self.app_state.replay.playback_position = self.app_state.replay.total_events;
            }
            self.sync_replay_state(cx);
        }
        if self.ui.button(cx, ids!(btn_end)).clicked(actions) {
            self.app_state.replay.playback_position = self.app_state.replay.total_events;
            self.sync_replay_state(cx);
        }

        // Jump chips
        if self.ui.button(cx, ids!(jump_failure)).clicked(actions) {
            // TODO: seek to first failure event when timeline data is wired
            self.sync_replay_state(cx);
        }
        if self.ui.button(cx, ids!(jump_action)).clicked(actions) {
            // TODO: seek to next action boundary when timeline data is wired
            self.sync_replay_state(cx);
        }
        if self.ui.button(cx, ids!(jump_done)).clicked(actions) {
            self.app_state.replay.playback_position = self.app_state.replay.total_events;
            self.sync_replay_state(cx);
        }

        // Verify screen actions
        if self.ui.button(cx, ids!(btn_reverify)).clicked(actions) {
            script_eval!(cx, { std.println("verify: re-running verification") });
        }
        if self.ui.button(cx, ids!(btn_export_cert)).clicked(actions) {
            script_eval!(cx, { std.println("verify: exporting certificate") });
        }

        // Incident actions
        if self.ui.button(cx, ids!(inc_dismiss)).clicked(actions) {
            // TODO: dismiss selected incident and refresh list
        }
    }
}

// ---------------------------------------------------------------------------
// Dynamic state binding methods
// ---------------------------------------------------------------------------

impl VbApp {
    /// Synchronizes navigation chrome (page flip, title, top-bar badges).
    fn sync_nav(&mut self, cx: &mut Cx, title: String, _title_color: String) {
        let run_id_text = ReplayData::run_id_text(self.app_state.selected_run_id);
        let wf_name = self
            .app_state
            .selected_workflow_name
            .clone()
            .unwrap_or_else(|| String::from("issue-triage"));

        match self.app_state.current_screen() {
            Screen::RunReplay => {
                script_apply_eval!(cx, self.ui, {
                    screens.active_page: replay_page
                    page_title.text: #(title)
                    page_title.draw_text.color: #00f5ff
                    run_id.text: #(run_id_text)
                    wf_name.text: #(wf_name)
                });
            }
            Screen::Verification => {
                script_apply_eval!(cx, self.ui, {
                    screens.active_page: verify_page
                    page_title.text: #(title)
                    page_title.draw_text.color: #39ff14
                    run_id.text: #(run_id_text)
                    wf_name.text: #(wf_name)
                });
            }
            Screen::SystemOverview => {
                script_apply_eval!(cx, self.ui, {
                    screens.active_page: system_page
                    page_title.text: #(title)
                    page_title.draw_text.color: #2d6bff
                    run_id.text: #(run_id_text)
                    wf_name.text: #(wf_name)
                });
            }
            Screen::WorkflowGraph => {
                script_apply_eval!(cx, self.ui, {
                    screens.active_page: workflow_page
                    page_title.text: #(title)
                    page_title.draw_text.color: #b14dff
                    run_id.text: #(run_id_text)
                    wf_name.text: #(wf_name)
                });
            }
            Screen::IncidentConsole => {
                script_apply_eval!(cx, self.ui, {
                    screens.active_page: incident_page
                    page_title.text: #(title)
                    page_title.draw_text.color: #ff073a
                    run_id.text: #(run_id_text)
                    wf_name.text: #(wf_name)
                });
            }
        }
    }

    /// Synchronizes Replay Theater state to UI labels.
    fn sync_replay_state(&mut self, cx: &mut Cx) {
        let event_count = self.app_state.replay.event_count_text();
        let speed_text = self.app_state.replay.speed_text();

        script_apply_eval!(cx, self.ui, {
            event_count.text: #(event_count)
            speed_label.text: #(speed_text)
        });

        // Update play button label based on is_playing state
        let play_label = if self.app_state.replay.is_playing {
            "||"
        } else {
            ">"
        };
        script_apply_eval!(cx, self.ui, {
            btn_play.text: #(play_label)
        });
    }

    /// Synchronizes Verification screen state to UI labels.
    fn sync_verify_state(&mut self, cx: &mut Cx) {
        let status_text = self.app_state.verification.status_badge_text();
        let total_str = self.app_state.verification.total_checks.to_string();
        let pass_str = self.app_state.verification.pass_count.to_string();
        let warn_str = self.app_state.verification.warn_count.to_string();
        let fail_str = self.app_state.verification.fail_count.to_string();
        let risk_str = self.app_state.verification.worst_risk_text();

        script_apply_eval!(cx, self.ui, {
            verify_status.text: #(status_text)
            su1v.text: #(total_str)
            su2v.text: #(pass_str)
            su3v.text: #(warn_str)
            su4v.text: #(fail_str)
            su5v.text: #(risk_str)
        });

        // Color the risk label based on severity
        if self.app_state.verification.fail_count > 0 {
            script_apply_eval!(cx, self.ui, {
                su5v.draw_text.color: #ff073a
            });
        } else if self.app_state.verification.warn_count > 0 {
            script_apply_eval!(cx, self.ui, {
                su5v.draw_text.color: #ffe600
            });
        } else {
            script_apply_eval!(cx, self.ui, {
                su5v.draw_text.color: #39ff14
            });
        }
    }

    /// Synchronizes System Overview screen state to UI labels.
    fn sync_system_state(&mut self, cx: &mut Cx) {
        let lanes_hint = self.app_state.system.lanes_hint_text();

        script_apply_eval!(cx, self.ui, {
            lanes_hint.text: #(lanes_hint)
        });
    }

    /// Synchronizes Workflow Graph screen state to UI labels.
    fn sync_workflow_state(&mut self, cx: &mut Cx) {
        let name = self.app_state.workflow.display_name();
        let hint = self.app_state.workflow.node_hint();

        script_apply_eval!(cx, self.ui, {
            graph_hint.text: #(hint)
        });

        // Update workflow title with name
        let wf_title = format!("WORKFLOW: {name}");
        script_apply_eval!(cx, self.ui, {
            wf_title.text: #(wf_title)
        });
    }

    /// Synchronizes Incident Console screen state to UI labels.
    fn sync_incident_state(&mut self, cx: &mut Cx) {
        let active_str = self.app_state.incident.active_incidents.to_string();
        let critical_str = self.app_state.incident.critical_count.to_string();
        let warning_str = self.app_state.incident.warning_count.to_string();

        let active_text = format!("{active_str} active");
        let critical_text = format!("{critical_str} critical");
        let warning_text = format!("{warning_str} warnings");

        script_apply_eval!(cx, self.ui, {
            inc_sum1.text: #(active_text)
            inc_sum2.text: #(critical_text)
            inc_sum3.text: #(warning_text)
        });
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
