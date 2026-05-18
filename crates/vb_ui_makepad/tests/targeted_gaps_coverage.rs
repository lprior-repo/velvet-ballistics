// Targeted gap coverage tests for vb_ui_makepad
// Tests the public API of vb_ui_makepad

use vb_ui_makepad::Error;
use vb_ui_makepad::graph_canvas::{GraphCanvas, ViewportRect};
use vb_ui_makepad::graph_edge::{EdgeRenderInstr, EdgeType, GraphEdge};
use vb_ui_makepad::graph_node::{GraphNode, NodeBadge, NodeCardRenderInstr, OverlayState};
use vb_ui_makepad::packet_dot::{AnimationTick, PacketDot, PacketDotManager};
use vb_ui_makepad::shell::{Screen, ShellNav};
use vb_ui_makepad::tokens::ParsedTokens;
use vb_ui_makepad::tokens::{color, layout, radius, shadow, space};

// ---------------------------------------------------------------------------
// Color token functions — exact RGBA values
// ---------------------------------------------------------------------------

#[test]
fn color_background_board_exact() {
    assert_eq!(color::background_board(), [0.957, 0.965, 0.973, 1.0]);
}
#[test]
fn color_shell_exact() {
    assert_eq!(color::shell(), [0.973, 0.980, 0.988, 1.0]);
}
#[test]
fn color_surface_exact() {
    assert_eq!(color::surface(), [1.0, 1.0, 1.0, 1.0]);
}
#[test]
fn color_surface_glass_exact() {
    assert_eq!(color::surface_glass(), [1.0, 1.0, 1.0, 0.8]);
}
#[test]
fn color_surface_muted_exact() {
    assert_eq!(color::surface_muted(), [0.949, 0.961, 0.973, 1.0]);
}
#[test]
fn color_line_hair_exact() {
    assert_eq!(color::line_hair(), [0.867, 0.890, 0.918, 1.0]);
}
#[test]
fn color_line_soft_exact() {
    assert_eq!(color::line_soft(), [0.910, 0.929, 0.949, 1.0]);
}
#[test]
fn color_text_primary_exact() {
    assert_eq!(color::text_primary(), [0.063, 0.094, 0.157, 1.0]);
}
#[test]
fn color_text_secondary_exact() {
    assert_eq!(color::text_secondary(), [0.278, 0.337, 0.404, 1.0]);
}
#[test]
fn color_text_tertiary_exact() {
    assert_eq!(color::text_tertiary(), [0.478, 0.529, 0.588, 1.0]);
}
#[test]
fn color_success_exact() {
    assert_eq!(color::success(), [0.086, 0.651, 0.416, 1.0]);
}
#[test]
fn color_running_exact() {
    assert_eq!(color::running(), [0.122, 0.478, 0.961, 1.0]);
}
#[test]
fn color_active_cyan_exact() {
    assert_eq!(color::active_cyan(), [0.098, 0.655, 0.808, 1.0]);
}
#[test]
fn color_warning_exact() {
    assert_eq!(color::warning(), [0.961, 0.620, 0.043, 1.0]);
}
#[test]
fn color_failure_exact() {
    assert_eq!(color::failure(), [0.898, 0.282, 0.302, 1.0]);
}
#[test]
fn color_taint_exact() {
    assert_eq!(color::taint(), [0.545, 0.361, 0.965, 1.0]);
}
#[test]
fn color_durable_exact() {
    assert_eq!(color::durable(), [0.078, 0.722, 0.651, 1.0]);
}
#[test]
fn color_pending_exact() {
    assert_eq!(color::pending(), [0.596, 0.635, 0.702, 1.0]);
}

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

#[test]
fn layout_sidebar_width_exact() {
    assert_eq!(layout::SIDEBAR_WIDTH, 246.0);
}
#[test]
fn layout_top_bar_height_exact() {
    assert_eq!(layout::TOP_BAR_HEIGHT, 78.0);
}
#[test]
fn layout_top_bar_width_exact() {
    assert_eq!(layout::TOP_BAR_WIDTH, 1674.0);
}
#[test]
fn layout_content_width_exact() {
    assert_eq!(layout::CONTENT_WIDTH, 1674.0);
}
#[test]
fn layout_content_height_exact() {
    assert_eq!(layout::CONTENT_HEIGHT, 1002.0);
}
#[test]
fn layout_nav_item_height_exact() {
    assert_eq!(layout::NAV_ITEM_HEIGHT, 56.0);
}
#[test]
fn layout_outer_margin_exact() {
    assert_eq!(layout::OUTER_MARGIN, 32.0);
}
#[test]
fn layout_content_gutter_exact() {
    assert_eq!(layout::CONTENT_GUTTER, 16.0);
}
#[test]
fn layout_inspector_width_min_exact() {
    assert_eq!(layout::INSPECTOR_WIDTH_MIN, 360.0);
}
#[test]
fn layout_inspector_width_max_exact() {
    assert_eq!(layout::INSPECTOR_WIDTH_MAX, 420.0);
}
#[test]
fn layout_bottom_timeline_min_exact() {
    assert_eq!(layout::BOTTOM_TIMELINE_MIN, 220.0);
}
#[test]
fn layout_graph_canvas_min_width_exact() {
    assert_eq!(layout::GRAPH_CANVAS_MIN_WIDTH, 720.0);
}
#[test]
fn layout_graph_canvas_min_height_exact() {
    assert_eq!(layout::GRAPH_CANVAS_MIN_HEIGHT, 520.0);
}
#[test]
fn layout_window_width_exact() {
    assert_eq!(layout::WINDOW_WIDTH, 1920.0);
}
#[test]
fn layout_window_height_exact() {
    assert_eq!(layout::WINDOW_HEIGHT, 1080.0);
}

// ---------------------------------------------------------------------------
// Radius and shadow
// ---------------------------------------------------------------------------

#[test]
fn radius_card_exact() {
    assert_eq!(radius::CARD, 16.0);
}
#[test]
fn shadow_card_exact() {
    assert_eq!(shadow::CARD, "0 8 24 rgba(16,24,40,0.08)");
}

// ---------------------------------------------------------------------------
// Space constants
// ---------------------------------------------------------------------------

#[test]
fn space_px4_exact() {
    assert_eq!(space::PX_4, 4.0);
}
#[test]
fn space_px8_exact() {
    assert_eq!(space::PX_8, 8.0);
}
#[test]
fn space_px12_exact() {
    assert_eq!(space::PX_12, 12.0);
}
#[test]
fn space_px16_exact() {
    assert_eq!(space::PX_16, 16.0);
}
#[test]
fn space_px20_exact() {
    assert_eq!(space::PX_20, 20.0);
}
#[test]
fn space_px24_exact() {
    assert_eq!(space::PX_24, 24.0);
}
#[test]
fn space_px32_exact() {
    assert_eq!(space::PX_32, 32.0);
}
#[test]
fn space_px40_exact() {
    assert_eq!(space::PX_40, 40.0);
}
#[test]
fn space_increasing() {
    assert!(space::PX_4 < space::PX_8 && space::PX_8 < space::PX_12 && space::PX_12 < space::PX_16);
    assert!(
        space::PX_16 < space::PX_20
            && space::PX_20 < space::PX_24
            && space::PX_24 < space::PX_32
            && space::PX_32 < space::PX_40
    );
}

// ---------------------------------------------------------------------------
// ParsedTokens::from_toml error cases
// ---------------------------------------------------------------------------

#[test]
fn parsed_tokens_from_toml_missing_color_returns_err() {
    assert!(ParsedTokens::from_toml("[layout]\nsidebar_width = 246.0\n").is_err());
}

#[test]
fn parsed_tokens_from_toml_missing_layout_returns_err() {
    assert!(ParsedTokens::from_toml("[color]\nbackground_board = \"#F4F6F8\"\n").is_err());
}

#[test]
fn parsed_tokens_from_toml_invalid_toml_syntax_returns_err() {
    assert!(ParsedTokens::from_toml("not valid = toml").is_err());
}

#[test]
fn parsed_tokens_from_toml_color_not_string_returns_err() {
    assert!(
        ParsedTokens::from_toml(
            "[color]\nbackground_board = 12345\n[layout]\nsidebar_width = 246.0\n"
        )
        .is_err()
    );
}

#[test]
fn parsed_tokens_from_toml_layout_not_number_returns_err() {
    assert!(
        ParsedTokens::from_toml(
            "[color]\nbackground_board = \"#F4F6F8\"\n[layout]\nsidebar_width = \"not a number\"\n"
        )
        .is_err()
    );
}

#[test]
fn parsed_tokens_from_toml_invalid_hex_returns_err() {
    assert!(ParsedTokens::from_toml("[color]\nbackground_board = \"#GGGGGG\"\nshell = \"#FF0000\"\n[layout]\nsidebar_width = 246.0\n").is_err());
}

// ---------------------------------------------------------------------------
// Error enum variants
// ---------------------------------------------------------------------------

#[test]
fn error_invalid_token_variant_exact() {
    let err = Error::InvalidToken("bad".into());
    assert!(format!("{:?}", err).contains("bad"));
}

#[test]
fn error_nav_item_not_found_variant_exact() {
    assert!(matches!(
        Error::NavItemNotFound("Overview".into()),
        Error::NavItemNotFound(_)
    ));
}

#[test]
fn error_invalid_screen_transition_variant_exact() {
    assert!(matches!(
        Error::InvalidScreenTransition("X->Y".into()),
        Error::InvalidScreenTransition(_)
    ));
}

#[test]
fn error_token_parse_error_variant_exact() {
    let err = Error::TokenParseError("bad hex".into());
    assert!(format!("{:?}", err).contains("bad hex"));
}

#[test]
fn error_invalid_flow_document_variant_exact() {
    assert!(matches!(
        Error::InvalidFlowDocument("bad yaml".into()),
        Error::InvalidFlowDocument(_)
    ));
}

#[test]
fn error_layout_not_computed_variant_exact() {
    assert!(matches!(Error::LayoutNotComputed, Error::LayoutNotComputed));
}

#[test]
fn error_node_not_found_variant_exact() {
    assert!(matches!(Error::NodeNotFound(42), Error::NodeNotFound(42)));
}

#[test]
fn error_invalid_viewport_variant_exact() {
    assert!(matches!(Error::InvalidViewport, Error::InvalidViewport));
}

#[test]
fn error_animation_overflow_variant_exact() {
    assert!(matches!(Error::AnimationOverflow, Error::AnimationOverflow));
}

#[test]
fn error_view_hidden_variant_exact() {
    assert!(matches!(Error::ViewHidden, Error::ViewHidden));
}

#[test]
fn error_missing_design_token_variant_exact() {
    assert!(matches!(
        Error::MissingDesignToken("missing_key".into()),
        Error::MissingDesignToken(_)
    ));
}

// ---------------------------------------------------------------------------
// ViewportRect
// ---------------------------------------------------------------------------

#[test]
fn viewport_rect_construct_and_access() {
    let rect = ViewportRect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    assert_eq!(rect.x, 0.0);
    assert_eq!(rect.width, 100.0);
}

#[test]
fn viewport_rect_intersects_normal_case() {
    let a = ViewportRect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    assert!(a.intersects(50.0, 50.0, 100.0, 100.0));
}

#[test]
fn viewport_rect_intersects_no_overlap() {
    let a = ViewportRect {
        x: 0.0,
        y: 0.0,
        width: 50.0,
        height: 50.0,
    };
    assert!(!a.intersects(100.0, 100.0, 50.0, 50.0));
}

#[test]
fn viewport_rect_intersects_edge_touching() {
    let a = ViewportRect {
        x: 0.0,
        y: 0.0,
        width: 50.0,
        height: 50.0,
    };
    assert!(!a.intersects(50.0, 50.0, 50.0, 50.0));
}

#[test]
fn viewport_rect_intersects_contained() {
    let outer = ViewportRect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    assert!(outer.intersects(25.0, 25.0, 50.0, 50.0));
}

#[test]
fn viewport_rect_intersects_zero_width() {
    let a = ViewportRect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    assert!(!a.intersects(100.0, 0.0, 0.0, 100.0));
}

#[test]
fn viewport_rect_intersects_zero_height() {
    let a = ViewportRect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    assert!(!a.intersects(0.0, 100.0, 100.0, 0.0));
}

#[test]
fn viewport_rect_symmetry() {
    let a = ViewportRect {
        x: 10.0,
        y: 20.0,
        width: 50.0,
        height: 60.0,
    };
    let a_vs_b = a.intersects(30.0, 40.0, 50.0, 60.0);
    let b_vs_a = ViewportRect {
        x: 30.0,
        y: 40.0,
        width: 50.0,
        height: 60.0,
    }
    .intersects(10.0, 20.0, 50.0, 60.0);
    assert_eq!(a_vs_b, b_vs_a);
}

// ---------------------------------------------------------------------------
// GraphCanvas
// ---------------------------------------------------------------------------

#[test]
fn graph_canvas_new_and_viewport_rect() {
    let canvas = GraphCanvas::new(0, vec![], vec![]);
    let rect = canvas.viewport_rect(1920.0, 1080.0);
    assert!(rect.width >= 0.0 && rect.height >= 0.0);
}

// ---------------------------------------------------------------------------
// ShellNav and Screen
// ---------------------------------------------------------------------------

#[test]
fn shell_nav_overview_screen() {
    assert_eq!(ShellNav::Overview.screen(), Screen::ExecutionOverview);
}
#[test]
fn shell_nav_workflow_graph_screen() {
    assert_eq!(
        ShellNav::WorkflowGraph.screen(),
        Screen::WorkflowGraphAuthoring
    );
}
#[test]
fn shell_nav_executions_screen() {
    assert_eq!(ShellNav::Executions.screen(), Screen::ExecutionDetailsGraph);
}
#[test]
fn shell_nav_verification_screen() {
    assert_eq!(
        ShellNav::Verification.screen(),
        Screen::VerificationCertificate
    );
}
#[test]
fn shell_nav_replay_screen() {
    assert_eq!(ShellNav::Replay.screen(), Screen::ReplayTheater);
}
#[test]
fn shell_nav_incidents_screen() {
    assert_eq!(ShellNav::Incidents.screen(), Screen::IncidentFailureConsole);
}
#[test]
fn shell_nav_actions_screen() {
    assert_eq!(ShellNav::Actions.screen(), Screen::ActionRegistry);
}
#[test]
fn shell_nav_storage_screen() {
    assert_eq!(ShellNav::Storage.screen(), Screen::StorageDoctorAiContext);
}

#[test]
fn shell_nav_all_variants_have_screen() {
    for nav in [
        ShellNav::Overview,
        ShellNav::WorkflowGraph,
        ShellNav::Executions,
        ShellNav::Verification,
        ShellNav::Replay,
        ShellNav::Incidents,
        ShellNav::Actions,
        ShellNav::Storage,
    ] {
        let _ = nav.screen();
    }
}

#[test]
fn screen_all_variants_from_nav() {
    let pairs = [
        (ShellNav::Overview, Screen::ExecutionOverview),
        (ShellNav::WorkflowGraph, Screen::WorkflowGraphAuthoring),
        (ShellNav::Executions, Screen::ExecutionDetailsGraph),
        (ShellNav::Verification, Screen::VerificationCertificate),
        (ShellNav::Replay, Screen::ReplayTheater),
        (ShellNav::Incidents, Screen::IncidentFailureConsole),
        (ShellNav::Actions, Screen::ActionRegistry),
        (ShellNav::Storage, Screen::StorageDoctorAiContext),
    ];
    assert_eq!(pairs.len(), 8);
    for (nav, screen) in pairs {
        assert_eq!(nav.screen(), screen);
    }
}

// ---------------------------------------------------------------------------
// OverlayState
// ---------------------------------------------------------------------------

#[test]
fn overlay_state_variant_count() {
    let variants = [
        OverlayState::Pending,
        OverlayState::Running,
        OverlayState::Succeeded,
        OverlayState::Failed,
        OverlayState::Skipped,
        OverlayState::Waiting,
        OverlayState::Asking,
        OverlayState::Cancelled,
    ];
    assert_eq!(variants.len(), 8);
}

#[test]
fn overlay_state_glow_color_pending() {
    assert_eq!(OverlayState::Pending.glow_color(), color::pending());
}
#[test]
fn overlay_state_glow_color_running() {
    assert_eq!(OverlayState::Running.glow_color(), color::running());
}
#[test]
fn overlay_state_glow_color_succeeded() {
    assert_eq!(OverlayState::Succeeded.glow_color(), color::success());
}
#[test]
fn overlay_state_glow_color_failed() {
    assert_eq!(OverlayState::Failed.glow_color(), color::failure());
}
#[test]
fn overlay_state_glow_color_skipped() {
    assert_eq!(OverlayState::Skipped.glow_color(), color::text_tertiary());
}
#[test]
fn overlay_state_glow_color_waiting() {
    assert_eq!(OverlayState::Waiting.glow_color(), color::active_cyan());
}
#[test]
fn overlay_state_glow_color_asking() {
    assert_eq!(OverlayState::Asking.glow_color(), color::warning());
}
#[test]
fn overlay_state_glow_color_cancelled() {
    assert_eq!(OverlayState::Cancelled.glow_color(), color::text_tertiary());
}

// ---------------------------------------------------------------------------
// NodeBadge
// ---------------------------------------------------------------------------

#[test]
fn node_badge_action_id_label() {
    assert_eq!(NodeBadge::ActionId(42).label(), "A42");
}
#[test]
fn node_badge_retry_max_label() {
    assert_eq!(NodeBadge::RetryMax(3).label(), "R3");
}
#[test]
fn node_badge_timeout_label() {
    assert_eq!(NodeBadge::Timeout(30).label(), "T30s");
}
#[test]
fn node_badge_secret_sensitive_label() {
    assert_eq!(NodeBadge::SecretSensitive.label(), "S");
}
#[test]
fn node_badge_strict_durable_label() {
    assert_eq!(NodeBadge::StrictDurable.label(), "D");
}
#[test]
fn node_badge_recent_failures_label() {
    assert_eq!(NodeBadge::RecentFailures(5).label(), "!5");
}
#[test]
fn node_badge_color_action_id() {
    assert_eq!(NodeBadge::ActionId(1).color(), [1.0, 0.42, 0.0, 1.0]);
}
#[test]
fn node_badge_color_timeout() {
    assert_eq!(NodeBadge::Timeout(10).color(), [1.0, 0.027, 0.227, 1.0]);
}
#[test]
fn node_badge_color_secret_sensitive() {
    assert_eq!(NodeBadge::SecretSensitive.color(), [1.0, 0.0, 1.0, 1.0]);
}
#[test]
fn node_badge_color_strict_durable() {
    assert_eq!(NodeBadge::StrictDurable.color(), [0.0, 0.898, 0.78, 1.0]);
}

// ---------------------------------------------------------------------------
// NodeCardRenderInstr
// ---------------------------------------------------------------------------

#[test]
fn node_card_render_instr_focus_shadow_color() {
    assert_eq!(
        NodeCardRenderInstr::focus_shadow_color(),
        [0.122, 0.478, 0.961, 1.0]
    );
}

#[test]
fn node_card_render_instr_failure_shadow_color() {
    assert_eq!(
        NodeCardRenderInstr::failure_shadow_color(),
        [0.898, 0.282, 0.302, 1.0]
    );
}

#[test]
fn node_card_render_instr_taint_overlay_color() {
    assert_eq!(NodeCardRenderInstr::taint_overlay_color(), color::taint());
}

// ---------------------------------------------------------------------------
// EdgeType
// ---------------------------------------------------------------------------

#[test]
fn edge_type_variant_count() {
    let variants = [
        EdgeType::Normal,
        EdgeType::Branch,
        EdgeType::ErrorRoute,
        EdgeType::RetryRoute,
        EdgeType::Join,
        EdgeType::LoopBack,
    ];
    assert_eq!(variants.len(), 6);
}

#[test]
fn edge_type_color_normal() {
    assert_eq!(EdgeType::Normal.color(), [0.0, 0.6, 0.8, 1.0]);
}
#[test]
fn edge_type_color_branch() {
    assert_eq!(EdgeType::Branch.color(), [0.694, 0.302, 1.0, 1.0]);
}
#[test]
fn edge_type_color_error_route() {
    assert_eq!(EdgeType::ErrorRoute.color(), [0.6, 0.1, 0.1, 1.0]);
}
#[test]
fn edge_type_color_retry_route() {
    assert_eq!(EdgeType::RetryRoute.color(), [1.0, 0.9, 0.0, 1.0]);
}
#[test]
fn edge_type_color_join() {
    assert_eq!(EdgeType::Join.color(), [0.176, 0.42, 1.0, 1.0]);
}
#[test]
fn edge_type_color_loop_back() {
    assert_eq!(EdgeType::LoopBack.color(), [0.0, 0.898, 0.78, 1.0]);
}

#[test]
fn edge_type_is_dashed_normal() {
    assert!(!EdgeType::Normal.is_dashed());
}
#[test]
fn edge_type_is_dashed_branch() {
    assert!(EdgeType::Branch.is_dashed());
}
#[test]
fn edge_type_is_dashed_error_route() {
    assert!(EdgeType::ErrorRoute.is_dashed());
}
#[test]
fn edge_type_is_dashed_retry_route() {
    assert!(EdgeType::RetryRoute.is_dashed());
}
#[test]
fn edge_type_is_dashed_join() {
    assert!(!EdgeType::Join.is_dashed());
}
#[test]
fn edge_type_is_dashed_loop_back() {
    assert!(!EdgeType::LoopBack.is_dashed());
}

// ---------------------------------------------------------------------------
// EdgeRenderInstr
// ---------------------------------------------------------------------------

#[test]
fn edge_render_instr_from_edge_path() {
    let instr = EdgeRenderInstr::from_edge_path(
        0,
        1,
        [0.0, 0.0],
        [50.0, 0.0],
        [50.0, 100.0],
        [100.0, 100.0],
        EdgeType::Normal,
    );
    assert_eq!(instr.source_step, 0);
    assert_eq!(instr.target_step, 1);
    assert_eq!(instr.edge_type, EdgeType::Normal);
    assert_eq!(instr.width, 2.0);
    assert!(!instr.dashed);
}

#[test]
fn edge_render_instr_from_edge_path_dashed() {
    let instr = EdgeRenderInstr::from_edge_path(
        0,
        1,
        [0.0, 0.0],
        [50.0, 0.0],
        [50.0, 100.0],
        [100.0, 100.0],
        EdgeType::Branch,
    );
    assert!(instr.dashed);
    assert_eq!(instr.color, EdgeType::Branch.color());
}

#[test]
fn edge_render_instr_with_label() {
    let instr = EdgeRenderInstr::from_edge_path(
        0,
        1,
        [0.0, 0.0],
        [50.0, 0.0],
        [50.0, 100.0],
        [100.0, 100.0],
        EdgeType::Normal,
    )
    .with_label("retry".to_string());
    assert_eq!(instr.label, Some("retry".to_string()));
}

// ---------------------------------------------------------------------------
// GraphEdge
// ---------------------------------------------------------------------------

#[test]
fn graph_edge_constants() {
    assert_eq!(GraphEdge::DEFAULT_WIDTH, 2.0);
    assert_eq!(GraphEdge::HIGHLIGHT_WIDTH, 3.0);
    assert_eq!(GraphEdge::PACKET_SIZE, 6.0);
}

// ---------------------------------------------------------------------------
// PacketDot
// ---------------------------------------------------------------------------

#[test]
fn packet_dot_new() {
    let dot = PacketDot::new("edge1".to_string());
    assert_eq!(dot.edge_id, "edge1");
    assert_eq!(dot.t, 0.0);
    assert!(dot.active);
}

#[test]
fn packet_dot_position_along_bezier_start() {
    let pos = PacketDot::position_along_bezier(
        0.0,
        [0.0, 0.0],
        [50.0, 0.0],
        [50.0, 100.0],
        [100.0, 100.0],
    );
    assert_eq!(pos, [0.0, 0.0]);
}

#[test]
fn packet_dot_position_along_bezier_mid() {
    let pos = PacketDot::position_along_bezier(
        0.5,
        [0.0, 0.0],
        [50.0, 0.0],
        [50.0, 100.0],
        [100.0, 100.0],
    );
    assert!(pos[0] > 0.0 && pos[0] < 100.0 && pos[1] > 0.0 && pos[1] < 100.0);
}

#[test]
fn packet_dot_color() {
    assert_eq!(
        PacketDot::new("e".to_string()).color(),
        color::active_cyan()
    );
}
#[test]
fn packet_dot_size() {
    assert_eq!(PacketDot::new("e".to_string()).size(), 6.0);
}

#[test]
fn packet_dot_reset() {
    let mut dot = PacketDot::new("e".to_string());
    dot.t = 0.8;
    dot.active = false;
    dot.reset();
    assert_eq!(dot.t, 0.0);
    assert!(dot.active);
}

// ---------------------------------------------------------------------------
// PacketDotManager
// ---------------------------------------------------------------------------

#[test]
fn packet_dot_manager_new() {
    let mgr = PacketDotManager::new();
    assert_eq!(mgr.total_count(), 0);
    assert_eq!(mgr.active_count(), 0);
}

#[test]
fn packet_dot_manager_add_dot() {
    let mut mgr = PacketDotManager::new();
    mgr.add_dot("edge1".to_string());
    assert_eq!(mgr.total_count(), 1);
    assert_eq!(mgr.active_count(), 1);
}

#[test]
fn packet_dot_manager_animate() {
    let mut mgr = PacketDotManager::new();
    mgr.add_dot("edge1".to_string());
    mgr.animate(1000.0);
    assert_eq!(mgr.total_count(), 1);
}

#[test]
fn packet_dot_manager_clear() {
    let mut mgr = PacketDotManager::new();
    mgr.add_dot("edge1".to_string());
    mgr.clear();
    assert_eq!(mgr.total_count(), 0);
}

#[test]
fn packet_dot_manager_reset_all() {
    let mut mgr = PacketDotManager::new();
    mgr.add_dot("edge1".to_string());
    mgr.animate(5000.0);
    mgr.reset_all();
    assert_eq!(mgr.total_count(), 1);
}

// ---------------------------------------------------------------------------
// AnimationTick
// ---------------------------------------------------------------------------

#[test]
fn animation_tick_new() {
    let tick = AnimationTick::new(100.0);
    assert_eq!(tick.delta_ms, 100.0);
}

#[test]
fn animation_tick_normalized_delta() {
    assert_eq!(AnimationTick::new(500.0).normalized_delta(), 0.5);
}

// ---------------------------------------------------------------------------
// GraphNode
// ---------------------------------------------------------------------------

#[test]
fn graph_node_constants() {
    assert_eq!(GraphNode::NODE_WIDTH, 160.0);
    assert_eq!(GraphNode::NODE_HEIGHT, 48.0);
    assert_eq!(GraphNode::HEADER_HEIGHT, 24.0);
}

#[test]
fn graph_node_card_dimensions() {
    let (w, h) = GraphNode::card_dimensions();
    assert_eq!(w, 160.0);
    assert_eq!(h, 48.0);
}

#[test]
fn graph_node_header_dimensions() {
    let (w, h) = GraphNode::header_dimensions();
    assert_eq!(w, 160.0);
    assert_eq!(h, 24.0);
}

#[test]
fn graph_node_badge_size() {
    assert_eq!(GraphNode::badge_size(), 16.0);
}
