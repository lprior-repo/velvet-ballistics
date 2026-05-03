pub mod ipc_bridge;

pub use makepad_widgets;

use makepad_widgets::*;

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

    // ── Main app ───────────────────────────────────────────────
    startup() do #(VbApp::script_component(vm)){
        ui: Root{
            on_startup: ||{
                ui.main_view.render()
            }
            main_window := Window{
                window.inner_size: vec2(1400, 900)
                window.title: "vb — Replay Theater"
                body +: {
                    main_view := View{
                        width: Fill height: Fill
                        flow: Down
                        new_batch: true
                        draw_bg +: {
                            color: #0a0a12
                        }

                        // ════════════════════════════════════════
                        // TOP BAR
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
                            run_badge := View{
                                width: Fit height: Fit
                                flow: Right spacing: 6
                                padding: Inset{left: 10 right: 10 top: 3 bottom: 3}
                                new_batch: true
                                draw_bg +: {color: #1a1a2e border_radius: 3.0}
                                run_label := Label{
                                    text: "Run:"
                                    draw_text +: {
                                        color: #555577
                                        text_style +: {font_size: 11}
                                    }
                                }
                                run_id := Label{
                                    text: "8172"
                                    draw_text +: {
                                        color: #00f5ff
                                        text_style +: {font_size: 11}
                                    }
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
                                    draw_text +: {
                                        color: #555577
                                        text_style +: {font_size: 11}
                                    }
                                }
                                wf_name := Label{
                                    text: "issue-triage"
                                    draw_text +: {
                                        color: #00f5ff
                                        text_style +: {font_size: 11}
                                    }
                                }
                            }
                        }

                        // ════════════════════════════════════════
                        // CONTENT AREA (left 60% + right 40%)
                        // ════════════════════════════════════════
                        content_area := View{
                            width: Fill height: Fill
                            flow: Right

                            // ────────────────────────────────────
                            // LEFT PANEL: Workflow Graph
                            // ────────────────────────────────────
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
                                        draw_text +: {
                                            color: #00f5ff
                                            text_style +: {font_size: 11}
                                        }
                                    }
                                    Filler{}
                                    graph_hint := Label{
                                        text: "6 nodes"
                                        draw_text +: {
                                            color: #555577
                                            text_style +: {font_size: 10}
                                        }
                                    }
                                }

                                // Scrollable node canvas
                                graph_canvas := ScrollXYView{
                                    width: Fill height: Fill
                                    flow: Down spacing: 8
                                    padding: 4

                                    // Row 1: SetConst -> Do -> Choose
                                    node_row1 := View{
                                        width: Fit height: Fit
                                        flow: Right spacing: 10
                                        align: Align{y: 0.5}

                                        // Node 1: SetConst (succeeded)
                                        node_setconst := NodeCard{
                                            draw_bg +: {color: #0d1a0d border_radius: 4.0}
                                            node_name := Label{
                                                text: "SetConst"
                                                draw_text +: {
                                                    color: #39ff14
                                                    text_style +: {font_size: 11}
                                                }
                                            }
                                            node_badge := Label{
                                                text: "succeeded"
                                                draw_text +: {
                                                    color: #39ff14
                                                    text_style +: {font_size: 9}
                                                }
                                            }
                                        }

                                        // Connector arrow
                                        arrow1 := Label{
                                            text: "->"
                                            draw_text +: {
                                                color: #2a2a4a
                                                text_style +: {font_size: 12}
                                            }
                                        }

                                        // Node 2: Do (succeeded)
                                        node_do := NodeCard{
                                            draw_bg +: {color: #0d1a0d border_radius: 4.0}
                                            node_name := Label{
                                                text: "Do"
                                                draw_text +: {
                                                    color: #39ff14
                                                    text_style +: {font_size: 11}
                                                }
                                            }
                                            node_badge := Label{
                                                text: "github.issue.create"
                                                draw_text +: {
                                                    color: #ff6b00
                                                    text_style +: {font_size: 9}
                                                }
                                            }
                                            node_state := Label{
                                                text: "succeeded"
                                                draw_text +: {
                                                    color: #39ff14
                                                    text_style +: {font_size: 9}
                                                }
                                            }
                                        }

                                        // Connector
                                        arrow2 := Label{
                                            text: "->"
                                            draw_text +: {color: #2a2a4a text_style +: {font_size: 12}}
                                        }

                                        // Node 3: Choose (succeeded)
                                        node_choose := NodeCard{
                                            draw_bg +: {color: #0d1a0d border_radius: 4.0}
                                            node_name := Label{
                                                text: "Choose"
                                                draw_text +: {
                                                    color: #b14dff
                                                    text_style +: {font_size: 11}
                                                }
                                            }
                                            node_badge := Label{
                                                text: "succeeded"
                                                draw_text +: {
                                                    color: #39ff14
                                                    text_style +: {font_size: 9}
                                                }
                                            }
                                        }
                                    }

                                    // Row 2: ForEach -> Do (external) -> Finish
                                    node_row2 := View{
                                        width: Fit height: Fit
                                        flow: Right spacing: 10
                                        align: Align{y: 0.5}
                                        margin: Inset{left: 40}

                                        // Node 4: ForEach (succeeded)
                                        node_foreach := NodeCard{
                                            draw_bg +: {color: #0d1a0d border_radius: 4.0}
                                            node_name := Label{
                                                text: "ForEach"
                                                draw_text +: {
                                                    color: #2d6bff
                                                    text_style +: {font_size: 11}
                                                }
                                            }
                                            node_badge := Label{
                                                text: "3 iterations"
                                                draw_text +: {
                                                    color: #8888aa
                                                    text_style +: {font_size: 9}
                                                }
                                            }
                                            node_state := Label{
                                                text: "succeeded"
                                                draw_text +: {
                                                    color: #39ff14
                                                    text_style +: {font_size: 9}
                                                }
                                            }
                                        }

                                        arrow3 := Label{
                                            text: "->"
                                            draw_text +: {color: #2a2a4a text_style +: {font_size: 12}}
                                        }

                                        // Node 5: Do (external action, orange)
                                        node_do_ext := NodeCard{
                                            draw_bg +: {color: #1a0d00 border_radius: 4.0}
                                            node_name := Label{
                                                text: "Do"
                                                draw_text +: {
                                                    color: #ff6b00
                                                    text_style +: {font_size: 11}
                                                }
                                            }
                                            node_badge := Label{
                                                text: "slack.notify"
                                                draw_text +: {
                                                    color: #ff6b00
                                                    text_style +: {font_size: 9}
                                                }
                                            }
                                            node_state := Label{
                                                text: "succeeded"
                                                draw_text +: {
                                                    color: #39ff14
                                                    text_style +: {font_size: 9}
                                                }
                                            }
                                        }

                                        arrow4 := Label{
                                            text: "->"
                                            draw_text +: {color: #2a2a4a text_style +: {font_size: 12}}
                                        }

                                        // Node 6: Finish
                                        node_finish := NodeCard{
                                            draw_bg +: {color: #0d1a0d border_radius: 4.0}
                                            node_name := Label{
                                                text: "Finish"
                                                draw_text +: {
                                                    color: #39ff14
                                                    text_style +: {font_size: 11}
                                                }
                                            }
                                            node_badge := Label{
                                                text: "completed"
                                                draw_text +: {
                                                    color: #39ff14
                                                    text_style +: {font_size: 9}
                                                }
                                            }
                                        }
                                    }

                                    // Taint path indicator
                                    taint_row := View{
                                        width: Fit height: Fit
                                        flow: Right spacing: 8
                                        margin: Inset{top: 6}
                                        align: Align{y: 0.5}
                                        taint_dot := Label{
                                            text: "*"
                                            draw_text +: {
                                                color: #ff00ff
                                                text_style +: {font_size: 14}
                                            }
                                        }
                                        taint_label := Label{
                                            text: "taint path detected: slot 12 via Do/slack.notify"
                                            draw_text +: {
                                                color: #ff00ff
                                                text_style +: {font_size: 10}
                                            }
                                        }
                                    }
                                }
                            }

                            // ─── Vertical separator ──────────────
                            vr1 := View{
                                width: 1 height: Fill
                                draw_bg +: {color: #2a2a4a}
                            }

                            // ────────────────────────────────────
                            // RIGHT PANEL: Detail Inspector
                            // ────────────────────────────────────
                            inspector_panel := View{
                                width: 380 height: Fill
                                flow: Down spacing: 6
                                padding: 12
                                new_batch: true
                                draw_bg +: {color: #12121f}

                                inspector_header := Label{
                                    text: "DETAIL INSPECTOR"
                                    draw_text +: {
                                        color: #00f5ff
                                        text_style +: {font_size: 11}
                                    }
                                }

                                // ── Step Inspector card ──────────
                                step_card := InfoCard{
                                    step_header := Label{
                                        text: "Step Inspector"
                                        draw_text +: {
                                            color: #e8e8ff
                                            text_style +: {font_size: 12}
                                        }
                                    }
                                    Hr{draw_bg +: {color: #2a2a4a}}
                                    step_fields := View{
                                        width: Fill height: Fit
                                        flow: Down spacing: 3
                                        sf1 := View{
                                            width: Fill height: Fit
                                            flow: Right spacing: 6
                                            sf1k := Label{
                                                text: "Step:"
                                                width: 80
                                                draw_text +: {color: #555577 text_style +: {font_size: 10}}
                                            }
                                            sf1v := Label{
                                                text: "github.issue.create"
                                                draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}
                                            }
                                        }
                                        sf2 := View{
                                            width: Fill height: Fit
                                            flow: Right spacing: 6
                                            sf2k := Label{
                                                text: "Kind:"
                                                width: 80
                                                draw_text +: {color: #555577 text_style +: {font_size: 10}}
                                            }
                                            sf2v := Label{
                                                text: "Do"
                                                draw_text +: {color: #ff6b00 text_style +: {font_size: 10}}
                                            }
                                        }
                                        sf3 := View{
                                            width: Fill height: Fit
                                            flow: Right spacing: 6
                                            sf3k := Label{
                                                text: "State:"
                                                width: 80
                                                draw_text +: {color: #555577 text_style +: {font_size: 10}}
                                            }
                                            sf3v := Label{
                                                text: "Succeeded"
                                                draw_text +: {color: #39ff14 text_style +: {font_size: 10}}
                                            }
                                        }
                                        sf4 := View{
                                            width: Fill height: Fit
                                            flow: Right spacing: 6
                                            sf4k := Label{
                                                text: "ActionId:"
                                                width: 80
                                                draw_text +: {color: #555577 text_style +: {font_size: 10}}
                                            }
                                            sf4v := Label{
                                                text: "17"
                                                draw_text +: {color: #00f5ff text_style +: {font_size: 10}}
                                            }
                                        }
                                    }
                                }

                                // ── Action Ticket card ───────────
                                ticket_card := InfoCard{
                                    ticket_header := Label{
                                        text: "Action Ticket"
                                        draw_text +: {
                                            color: #e8e8ff
                                            text_style +: {font_size: 12}
                                        }
                                    }
                                    Hr{draw_bg +: {color: #2a2a4a}}
                                    ticket_fields := View{
                                        width: Fill height: Fit
                                        flow: Down spacing: 3
                                        tf1 := View{
                                            width: Fill height: Fit
                                            flow: Right spacing: 6
                                            tf1k := Label{
                                                text: "Ticket:"
                                                width: 80
                                                draw_text +: {color: #555577 text_style +: {font_size: 10}}
                                            }
                                            tf1v := Label{
                                                text: "#42"
                                                draw_text +: {color: #00f5ff text_style +: {font_size: 10}}
                                            }
                                        }
                                        tf2 := View{
                                            width: Fill height: Fit
                                            flow: Right spacing: 6
                                            tf2k := Label{
                                                text: "Replay-safe:"
                                                width: 80
                                                draw_text +: {color: #555577 text_style +: {font_size: 10}}
                                            }
                                            tf2v := Label{
                                                text: "YES"
                                                draw_text +: {color: #39ff14 text_style +: {font_size: 10}}
                                            }
                                        }
                                    }
                                }

                                // ── Slot Diffs card ──────────────
                                slots_card := InfoCard{
                                    slots_header := Label{
                                        text: "Slot Diffs"
                                        draw_text +: {
                                            color: #e8e8ff
                                            text_style +: {font_size: 12}
                                        }
                                    }
                                    Hr{draw_bg +: {color: #2a2a4a}}
                                    slots_fields := View{
                                        width: Fill height: Fit
                                        flow: Down spacing: 3
                                        sd1 := View{
                                            width: Fill height: Fit
                                            flow: Right spacing: 6
                                            sd1k := Label{
                                                text: "S12:"
                                                width: 40
                                                draw_text +: {color: #ff00ff text_style +: {font_size: 10}}
                                            }
                                            sd1arrow := Label{
                                                text: "null"
                                                draw_text +: {color: #555577 text_style +: {font_size: 10}}
                                            }
                                            sd1sep := Label{
                                                text: "->"
                                                draw_text +: {color: #2a2a4a text_style +: {font_size: 10}}
                                            }
                                            sd1v := Label{
                                                text: "ObjectId(0x3f7a..)"
                                                draw_text +: {color: #00f5ff text_style +: {font_size: 10}}
                                            }
                                        }
                                        sd2 := View{
                                            width: Fill height: Fit
                                            flow: Right spacing: 6
                                            sd2k := Label{
                                                text: "S8:"
                                                width: 40
                                                draw_text +: {color: #b14dff text_style +: {font_size: 10}}
                                            }
                                            sd2arrow := Label{
                                                text: "null"
                                                draw_text +: {color: #555577 text_style +: {font_size: 10}}
                                            }
                                            sd2sep := Label{
                                                text: "->"
                                                draw_text +: {color: #2a2a4a text_style +: {font_size: 10}}
                                            }
                                            sd2v := Label{
                                                text: "Issue { title, body }"
                                                draw_text +: {color: #e8e8ff text_style +: {font_size: 10}}
                                            }
                                        }
                                        sd3 := View{
                                            width: Fill height: Fit
                                            flow: Right spacing: 6
                                            sd3k := Label{
                                                text: "S19:"
                                                width: 40
                                                draw_text +: {color: #2d6bff text_style +: {font_size: 10}}
                                            }
                                            sd3arrow := Label{
                                                text: "null"
                                                draw_text +: {color: #555577 text_style +: {font_size: 10}}
                                            }
                                            sd3sep := Label{
                                                text: "->"
                                                draw_text +: {color: #2a2a4a text_style +: {font_size: 10}}
                                            }
                                            sd3v := Label{
                                                text: "true"
                                                draw_text +: {color: #39ff14 text_style +: {font_size: 10}}
                                            }
                                        }
                                    }
                                }

                                // ── State legend ─────────────────
                                legend := View{
                                    width: Fill height: Fit
                                    flow: Down spacing: 3
                                    margin: Inset{top: 4}
                                    legend_title := Label{
                                        text: "STATE LEGEND"
                                        draw_text +: {color: #555577 text_style +: {font_size: 9}}
                                    }
                                    leg_row1 := View{
                                        width: Fill height: Fit
                                        flow: Right spacing: 12
                                        lg1 := Label{text: "Succeeded" draw_text +: {color: #39ff14 text_style +: {font_size: 9}}}
                                        lg2 := Label{text: "Running" draw_text +: {color: #00f5ff text_style +: {font_size: 9}}}
                                        lg3 := Label{text: "Failed" draw_text +: {color: #ff073a text_style +: {font_size: 9}}}
                                    }
                                    leg_row2 := View{
                                        width: Fill height: Fit
                                        flow: Right spacing: 12
                                        lg4 := Label{text: "Waiting" draw_text +: {color: #2d6bff text_style +: {font_size: 9}}}
                                        lg5 := Label{text: "Asking" draw_text +: {color: #ffe600 text_style +: {font_size: 9}}}
                                        lg6 := Label{text: "Taint" draw_text +: {color: #ff00ff text_style +: {font_size: 9}}}
                                    }
                                }
                            }
                        }

                        // ════════════════════════════════════════
                        // BOTTOM BAR: Transport + Event Strip
                        // ════════════════════════════════════════
                        bottom_bar := View{
                            width: Fill height: Fit
                            flow: Down
                            new_batch: true
                            draw_bg +: {color: #12121f}

                            // Separator line
                            sep_line := View{
                                width: Fill height: 1
                                draw_bg +: {color: #2a2a4a}
                            }

                            // Transport controls row
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
                                    speed_label := Label{
                                        text: "1x"
                                        draw_text +: {color: #8888aa text_style +: {font_size: 10}}
                                    }
                                }

                                // Thin vertical separator
                                transport_sep := View{
                                    width: 1 height: 20
                                    margin: Inset{left: 6 right: 6}
                                    draw_bg +: {color: #2a2a4a}
                                }

                                jump_failure := JumpChip{text: "jump: failure"}
                                jump_action := JumpChip{text: "action"}
                                jump_done := JumpChip{text: "done"}

                                Filler{}

                                event_count := Label{
                                    text: "12 events"
                                    draw_text +: {color: #555577 text_style +: {font_size: 10}}
                                }
                            }

                            // Event strip row
                            event_strip := ScrollXView{
                                width: Fill height: 44
                                flow: Right spacing: 4
                                padding: Inset{left: 12 right: 12 top: 4 bottom: 8}
                                align: Align{y: 0.5}

                                // Playback position indicator
                                pos_dot := Label{
                                    text: "--*--"
                                    draw_text +: {color: #00f5ff text_style +: {font_size: 10}}
                                }

                                // Event 1: RunAccepted (cyan)
                                ev1 := EventChip{
                                    draw_bg +: {color: #0a1a1a border_radius: 3.0}
                                    ev_dot := Label{
                                        text: "*"
                                        draw_text +: {color: #00f5ff text_style +: {font_size: 10}}
                                    }
                                    ev_label := Label{
                                        text: "RunAccepted"
                                        draw_text +: {color: #00f5ff text_style +: {font_size: 9}}
                                    }
                                }

                                // Event 2: StepStarted (green)
                                ev2 := EventChip{
                                    draw_bg +: {color: #0a1a0d border_radius: 3.0}
                                    ev_dot := Label{
                                        text: "*"
                                        draw_text +: {color: #39ff14 text_style +: {font_size: 10}}
                                    }
                                    ev_label := Label{
                                        text: "Step:0"
                                        draw_text +: {color: #39ff14 text_style +: {font_size: 9}}
                                    }
                                }

                                // Event 3: ActionScheduled (blue)
                                ev3 := EventChip{
                                    draw_bg +: {color: #0d0d1a border_radius: 3.0}
                                    ev_dot := Label{
                                        text: "*"
                                        draw_text +: {color: #2d6bff text_style +: {font_size: 10}}
                                    }
                                    ev_label := Label{
                                        text: "ActionScheduled"
                                        draw_text +: {color: #2d6bff text_style +: {font_size: 9}}
                                    }
                                }

                                // Event 4: Do invoked (orange)
                                ev4 := EventChip{
                                    draw_bg +: {color: #1a0d00 border_radius: 3.0}
                                    ev_dot := Label{
                                        text: "*"
                                        draw_text +: {color: #ff6b00 text_style +: {font_size: 10}}
                                    }
                                    ev_label := Label{
                                        text: "github.issue.create"
                                        draw_text +: {color: #ff6b00 text_style +: {font_size: 9}}
                                    }
                                }

                                // Event 5: Succeeded (green)
                                ev5 := EventChip{
                                    draw_bg +: {color: #0a1a0d border_radius: 3.0}
                                    ev_dot := Label{
                                        text: "*"
                                        draw_text +: {color: #39ff14 text_style +: {font_size: 10}}
                                    }
                                    ev_label := Label{
                                        text: "Succeeded"
                                        draw_text +: {color: #39ff14 text_style +: {font_size: 9}}
                                    }
                                }

                                // Event 6: Choose branch (purple)
                                ev6 := EventChip{
                                    draw_bg +: {color: #12061a border_radius: 3.0}
                                    ev_dot := Label{
                                        text: "*"
                                        draw_text +: {color: #b14dff text_style +: {font_size: 10}}
                                    }
                                    ev_label := Label{
                                        text: "Choose[0]"
                                        draw_text +: {color: #b14dff text_style +: {font_size: 9}}
                                    }
                                }

                                // Event 7: ForEach iteration (blue)
                                ev7 := EventChip{
                                    draw_bg +: {color: #0d0d1a border_radius: 3.0}
                                    ev_dot := Label{
                                        text: "*"
                                        draw_text +: {color: #2d6bff text_style +: {font_size: 10}}
                                    }
                                    ev_label := Label{
                                        text: "ForEach[2/3]"
                                        draw_text +: {color: #2d6bff text_style +: {font_size: 9}}
                                    }
                                }

                                // Event 8: Taint (magenta)
                                ev8 := EventChip{
                                    draw_bg +: {color: #1a001a border_radius: 3.0}
                                    ev_dot := Label{
                                        text: "!"
                                        draw_text +: {color: #ff00ff text_style +: {font_size: 10}}
                                    }
                                    ev_label := Label{
                                        text: "Taint(S12)"
                                        draw_text +: {color: #ff00ff text_style +: {font_size: 9}}
                                    }
                                }

                                // Event 9: Completed (green)
                                ev9 := EventChip{
                                    draw_bg +: {color: #0a1a0d border_radius: 3.0}
                                    ev_dot := Label{
                                        text: "*"
                                        draw_text +: {color: #39ff14 text_style +: {font_size: 10}}
                                    }
                                    ev_label := Label{
                                        text: "Completed"
                                        draw_text +: {color: #39ff14 text_style +: {font_size: 9}}
                                    }
                                }

                                // Event 10: RunFinished (cyan)
                                ev10 := EventChip{
                                    draw_bg +: {color: #0a1a1a border_radius: 3.0}
                                    ev_dot := Label{
                                        text: "*"
                                        draw_text +: {color: #00f5ff text_style +: {font_size: 10}}
                                    }
                                    ev_label := Label{
                                        text: "RunFinished"
                                        draw_text +: {color: #00f5ff text_style +: {font_size: 9}}
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
}

impl MatchEvent for VbApp {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        // Transport button handling
        if self.ui.button(cx, ids!(btn_start)).clicked(actions) {
            script_eval!(cx, {
                std.println("transport: jump to start")
            });
        }
        if self.ui.button(cx, ids!(btn_prev)).clicked(actions) {
            script_eval!(cx, {
                std.println("transport: step backward")
            });
        }
        if self.ui.button(cx, ids!(btn_play)).clicked(actions) {
            script_eval!(cx, {
                std.println("transport: play/pause toggle")
            });
        }
        if self.ui.button(cx, ids!(btn_next)).clicked(actions) {
            script_eval!(cx, {
                std.println("transport: step forward")
            });
        }
        if self.ui.button(cx, ids!(btn_end)).clicked(actions) {
            script_eval!(cx, {
                std.println("transport: jump to end")
            });
        }

        // Jump chip handling
        if self.ui.button(cx, ids!(jump_failure)).clicked(actions) {
            script_eval!(cx, {
                std.println("jump: seeking to first failure event")
            });
        }
        if self.ui.button(cx, ids!(jump_action)).clicked(actions) {
            script_eval!(cx, {
                std.println("jump: seeking to next action boundary")
            });
        }
        if self.ui.button(cx, ids!(jump_done)).clicked(actions) {
            script_eval!(cx, {
                std.println("jump: seeking to run completion")
            });
        }
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
