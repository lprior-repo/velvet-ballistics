// Targeted gap coverage tests for vb_ui_makepad
// Tests the public API of vb_ui_makepad

use vb_ui_makepad::Error;
use vb_ui_makepad::graph_canvas::{EdgePath, GraphCanvas, ViewportRect};
use vb_ui_makepad::graph_edge::{EdgeRenderInstr, EdgeType, GraphEdge, PacketMarkerInstr};
use vb_ui_makepad::graph_node::{GraphNode, NodeBadge, NodeCardRenderInstr, OverlayState};
use vb_ui_makepad::packet_dot::{AnimationTick, PacketDot, PacketDotManager};
use vb_ui_makepad::shell::{AppShell, Screen, ShellNav, ShellStatusChip};
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

// ---------------------------------------------------------------------------
// OverlayState::glow_radius — all 8 variants
// ---------------------------------------------------------------------------

#[test]
fn overlay_state_glow_radius_pending() {
    assert_eq!(OverlayState::Pending.glow_radius(), 2.0);
}
#[test]
fn overlay_state_glow_radius_running() {
    assert_eq!(OverlayState::Running.glow_radius(), 4.0);
}
#[test]
fn overlay_state_glow_radius_succeeded() {
    assert_eq!(OverlayState::Succeeded.glow_radius(), 3.0);
}
#[test]
fn overlay_state_glow_radius_failed() {
    assert_eq!(OverlayState::Failed.glow_radius(), 6.0);
}
#[test]
fn overlay_state_glow_radius_skipped() {
    assert_eq!(OverlayState::Skipped.glow_radius(), 2.0);
}
#[test]
fn overlay_state_glow_radius_waiting() {
    assert_eq!(OverlayState::Waiting.glow_radius(), 3.0);
}
#[test]
fn overlay_state_glow_radius_asking() {
    assert_eq!(OverlayState::Asking.glow_radius(), 3.0);
}
#[test]
fn overlay_state_glow_radius_cancelled() {
    assert_eq!(OverlayState::Cancelled.glow_radius(), 2.0);
}

// ---------------------------------------------------------------------------
// ShellNav::label — all 8 variants
// ---------------------------------------------------------------------------

#[test]
fn shell_nav_label_overview() {
    assert_eq!(ShellNav::Overview.label(), "Overview");
}
#[test]
fn shell_nav_label_workflow_graph() {
    assert_eq!(ShellNav::WorkflowGraph.label(), "Workflow Graph");
}
#[test]
fn shell_nav_label_executions() {
    assert_eq!(ShellNav::Executions.label(), "Executions");
}
#[test]
fn shell_nav_label_verification() {
    assert_eq!(ShellNav::Verification.label(), "Verification");
}
#[test]
fn shell_nav_label_replay() {
    assert_eq!(ShellNav::Replay.label(), "Replay");
}
#[test]
fn shell_nav_label_incidents() {
    assert_eq!(ShellNav::Incidents.label(), "Incidents");
}
#[test]
fn shell_nav_label_actions() {
    assert_eq!(ShellNav::Actions.label(), "Actions");
}
#[test]
fn shell_nav_label_storage() {
    assert_eq!(ShellNav::Storage.label(), "Storage / AI");
}

// ---------------------------------------------------------------------------
// ShellNav::nav_color — all 8 variants with exact RGBA
// ---------------------------------------------------------------------------

#[test]
fn shell_nav_nav_color_overview() {
    assert_eq!(ShellNav::Overview.nav_color(), [0.145, 0.388, 0.922, 1.0]);
}
#[test]
fn shell_nav_nav_color_workflow_graph() {
    assert_eq!(
        ShellNav::WorkflowGraph.nav_color(),
        [0.431, 0.321, 0.898, 1.0]
    );
}
#[test]
fn shell_nav_nav_color_executions() {
    assert_eq!(ShellNav::Executions.nav_color(), [0.145, 0.388, 0.922, 1.0]);
}
#[test]
fn shell_nav_nav_color_verification() {
    assert_eq!(
        ShellNav::Verification.nav_color(),
        [0.086, 0.651, 0.416, 1.0]
    );
}
#[test]
fn shell_nav_nav_color_replay() {
    assert_eq!(ShellNav::Replay.nav_color(), [0.169, 0.424, 1.0, 1.0]);
}
#[test]
fn shell_nav_nav_color_incidents() {
    assert_eq!(ShellNav::Incidents.nav_color(), [0.898, 0.282, 0.302, 1.0]);
}
#[test]
fn shell_nav_nav_color_actions() {
    assert_eq!(ShellNav::Actions.nav_color(), [0.773, 0.357, 0.083, 1.0]);
}
#[test]
fn shell_nav_nav_color_storage() {
    assert_eq!(ShellNav::Storage.nav_color(), [0.078, 0.722, 0.651, 1.0]);
}

// ---------------------------------------------------------------------------
// Screen::splash_name — all 8 variants
// ---------------------------------------------------------------------------

#[test]
fn screen_splash_name_execution_overview() {
    assert_eq!(Screen::ExecutionOverview.splash_name(), "ExecutionOverview");
}
#[test]
fn screen_splash_name_workflow_graph_authoring() {
    assert_eq!(
        Screen::WorkflowGraphAuthoring.splash_name(),
        "WorkflowGraphAuthoring"
    );
}
#[test]
fn screen_splash_name_execution_details_graph() {
    assert_eq!(
        Screen::ExecutionDetailsGraph.splash_name(),
        "ExecutionDetailsGraph"
    );
}
#[test]
fn screen_splash_name_verification_certificate() {
    assert_eq!(
        Screen::VerificationCertificate.splash_name(),
        "VerificationCertificate"
    );
}
#[test]
fn screen_splash_name_replay_theater() {
    assert_eq!(Screen::ReplayTheater.splash_name(), "ReplayTheater");
}
#[test]
fn screen_splash_name_incident_failure_console() {
    assert_eq!(
        Screen::IncidentFailureConsole.splash_name(),
        "IncidentFailureConsole"
    );
}
#[test]
fn screen_splash_name_action_registry() {
    assert_eq!(Screen::ActionRegistry.splash_name(), "ActionRegistry");
}
#[test]
fn screen_splash_name_storage_doctor_ai_context() {
    assert_eq!(
        Screen::StorageDoctorAiContext.splash_name(),
        "StorageDoctorAiContext"
    );
}

// ---------------------------------------------------------------------------
// Screen::nav_label — all 8 variants
// ---------------------------------------------------------------------------

#[test]
fn screen_nav_label_execution_overview() {
    assert_eq!(Screen::ExecutionOverview.nav_label(), "Overview");
}
#[test]
fn screen_nav_label_workflow_graph_authoring() {
    assert_eq!(Screen::WorkflowGraphAuthoring.nav_label(), "Workflow Graph");
}
#[test]
fn screen_nav_label_execution_details_graph() {
    assert_eq!(Screen::ExecutionDetailsGraph.nav_label(), "Executions");
}
#[test]
fn screen_nav_label_verification_certificate() {
    assert_eq!(Screen::VerificationCertificate.nav_label(), "Verification");
}
#[test]
fn screen_nav_label_replay_theater() {
    assert_eq!(Screen::ReplayTheater.nav_label(), "Replay");
}
#[test]
fn screen_nav_label_incident_failure_console() {
    assert_eq!(Screen::IncidentFailureConsole.nav_label(), "Incidents");
}
#[test]
fn screen_nav_label_action_registry() {
    assert_eq!(Screen::ActionRegistry.nav_label(), "Actions");
}
#[test]
fn screen_nav_label_storage_doctor_ai_context() {
    assert_eq!(Screen::StorageDoctorAiContext.nav_label(), "Storage / AI");
}

// ---------------------------------------------------------------------------
// Screen::is_shell_screen — all 8 variants
// ---------------------------------------------------------------------------

#[test]
fn screen_is_shell_screen_all_variants() {
    assert!(Screen::ExecutionOverview.is_shell_screen());
    assert!(Screen::WorkflowGraphAuthoring.is_shell_screen());
    assert!(Screen::ExecutionDetailsGraph.is_shell_screen());
    assert!(Screen::VerificationCertificate.is_shell_screen());
    assert!(Screen::ReplayTheater.is_shell_screen());
    assert!(Screen::IncidentFailureConsole.is_shell_screen());
    assert!(Screen::ActionRegistry.is_shell_screen());
    assert!(Screen::StorageDoctorAiContext.is_shell_screen());
}

// ---------------------------------------------------------------------------
// ShellStatusChip
// ---------------------------------------------------------------------------

#[test]
fn shell_status_chip_new_exact_fields() {
    let chip = ShellStatusChip::new("Running", [0.1, 0.5, 0.9, 1.0]);
    assert_eq!(chip.label, "Running");
    assert_eq!(chip.color, [0.1, 0.5, 0.9, 1.0]);
}

// ---------------------------------------------------------------------------
// AppShell
// ---------------------------------------------------------------------------

#[test]
fn app_shell_new_returns_ok_with_defaults() {
    let shell = AppShell::new();
    let shell = shell.expect("AppShell::new should not fail");
    assert_eq!(shell.active_nav, ShellNav::Overview);
    assert!(shell.status_chips.is_empty());
}

#[test]
fn app_shell_set_and_get_active_nav() {
    let mut shell = AppShell::new().expect("should not fail");
    shell.set_active_nav(ShellNav::Executions);
    assert_eq!(shell.active_nav(), ShellNav::Executions);
    shell.set_active_nav(ShellNav::Verification);
    assert_eq!(shell.active_nav(), ShellNav::Verification);
}

#[test]
fn app_shell_active_nav_getter() {
    let mut shell = AppShell::new().expect("should not fail");
    assert_eq!(shell.active_nav(), ShellNav::Overview);
    shell.set_active_nav(ShellNav::Storage);
    assert_eq!(shell.active_nav(), ShellNav::Storage);
}

#[test]
fn app_shell_nav_item_rect_index_zero() {
    let shell = AppShell::new().expect("should not fail");
    let rect = shell.nav_item_rect(0);
    assert_eq!(rect.x, 0.0);
    assert_eq!(rect.y, 0.0);
    assert_eq!(rect.width, layout::SIDEBAR_WIDTH);
    assert_eq!(rect.height, layout::NAV_ITEM_HEIGHT);
}

#[test]
fn app_shell_nav_item_rect_index_one() {
    let shell = AppShell::new().expect("should not fail");
    let rect = shell.nav_item_rect(1);
    assert_eq!(rect.x, 0.0);
    assert_eq!(rect.y, layout::NAV_ITEM_HEIGHT);
    assert_eq!(rect.width, layout::SIDEBAR_WIDTH);
    assert_eq!(rect.height, layout::NAV_ITEM_HEIGHT);
}

#[test]
fn app_shell_nav_item_rect_index_three() {
    let shell = AppShell::new().expect("should not fail");
    let rect = shell.nav_item_rect(3);
    assert_eq!(rect.y, 3.0 * layout::NAV_ITEM_HEIGHT);
}

#[test]
fn app_shell_nav_item_rect_overflow_index() {
    let shell = AppShell::new().expect("should not fail");
    let rect = shell.nav_item_rect(usize::MAX);
    assert_eq!(rect.y, 0.0); // saturating cast to f64
}

#[test]
fn app_shell_topbar_rect_exact() {
    let shell = AppShell::new().expect("should not fail");
    let rect = shell.topbar_rect();
    assert_eq!(rect.x, layout::SIDEBAR_WIDTH);
    assert_eq!(rect.y, 0.0);
    assert_eq!(rect.width, layout::TOP_BAR_WIDTH);
    assert_eq!(rect.height, layout::TOP_BAR_HEIGHT);
}

#[test]
fn app_shell_content_rect_exact() {
    let shell = AppShell::new().expect("should not fail");
    let rect = shell.content_rect();
    assert_eq!(rect.x, layout::SIDEBAR_WIDTH);
    assert_eq!(rect.y, layout::TOP_BAR_HEIGHT);
    assert_eq!(rect.width, layout::CONTENT_WIDTH);
    assert_eq!(rect.height, layout::CONTENT_HEIGHT);
}

// ---------------------------------------------------------------------------
// GraphCanvas — viewport, pan, zoom
// ---------------------------------------------------------------------------

#[test]
fn graph_canvas_viewport_rect_with_zoom_in() {
    let canvas = GraphCanvas::new(0, vec![], vec![]);
    let rect = canvas.viewport_rect(1920.0, 1080.0);
    assert_eq!(rect.width, 1920.0);
    assert_eq!(rect.height, 1080.0);
}

#[test]
fn graph_canvas_viewport_rect_with_zoom_out() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    canvas.set_zoom(0.5);
    let rect = canvas.viewport_rect(1920.0, 1080.0);
    assert_eq!(rect.width, 1920.0 * 2.0);
    assert_eq!(rect.height, 1080.0 * 2.0);
}

#[test]
fn graph_canvas_viewport_rect_zoom_clamped_to_min() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    canvas.set_zoom(0.0);
    assert_eq!(canvas.zoom(), 0.1); // clamped to MIN_ZOOM
    let rect = canvas.viewport_rect(1920.0, 1080.0);
    assert_eq!(rect.width, 1920.0 / 0.1);
    assert_eq!(rect.height, 1080.0 / 0.1);
}

#[test]
fn graph_canvas_set_pan_exact() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    canvas.set_pan(100.0, -50.0);
    assert_eq!(canvas.pan(), (100.0, -50.0));
    canvas.set_pan(0.0, 0.0);
    assert_eq!(canvas.pan(), (0.0, 0.0));
}

#[test]
fn graph_canvas_set_zoom_clamp_min() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    canvas.set_zoom(0.01);
    assert_eq!(canvas.zoom(), 0.1);
}

#[test]
fn graph_canvas_set_zoom_clamp_max() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    canvas.set_zoom(10.0);
    assert_eq!(canvas.zoom(), 5.0);
}

#[test]
fn graph_canvas_set_zoom_within_range() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    canvas.set_zoom(2.0);
    assert_eq!(canvas.zoom(), 2.0);
}

#[test]
fn graph_canvas_zoom_in() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    canvas.set_zoom(1.0);
    canvas.zoom_in(2.0);
    assert_eq!(canvas.zoom(), 2.0);
}

#[test]
fn graph_canvas_zoom_out() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    canvas.set_zoom(1.0);
    canvas.zoom_out(2.0);
    assert_eq!(canvas.zoom(), 0.5);
}

#[test]
fn graph_canvas_zoom_reset() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    canvas.set_zoom(3.0);
    canvas.zoom_reset();
    assert_eq!(canvas.zoom(), 1.0);
}

#[test]
fn graph_canvas_zoom_percentage_default() {
    let canvas = GraphCanvas::new(0, vec![], vec![]);
    assert_eq!(canvas.zoom_percentage(), "100%");
}

#[test]
fn graph_canvas_zoom_percentage_custom() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    canvas.set_zoom(2.5);
    assert_eq!(canvas.zoom_percentage(), "250%");
}

#[test]
fn graph_canvas_zoom_percentage_rounds() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    canvas.set_zoom(1.333);
    assert_eq!(canvas.zoom_percentage(), "133%");
}

#[test]
fn graph_canvas_set_selected() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    canvas.set_selected(Some(5));
    assert_eq!(canvas.selected(), Some(5));
    canvas.set_selected(None);
    assert_eq!(canvas.selected(), None);
}

// ---------------------------------------------------------------------------
// GraphCanvas — node layout, visibility, rendering
// ---------------------------------------------------------------------------

#[test]
fn graph_canvas_visible_nodes_all_visible() {
    let positions = vec![(100.0, 100.0), (200.0, 200.0), (300.0, 300.0)];
    let canvas = GraphCanvas::new(3, positions.clone(), vec![]);
    let viewport = ViewportRect {
        x: 0.0,
        y: 0.0,
        width: 500.0,
        height: 500.0,
    };
    let visible = canvas.visible_nodes(&viewport, (160.0, 48.0));
    assert_eq!(visible.len(), 3);
}

#[test]
fn graph_canvas_visible_nodes_none_visible() {
    let positions = vec![(1000.0, 1000.0), (2000.0, 2000.0)];
    let canvas = GraphCanvas::new(2, positions, vec![]);
    let viewport = ViewportRect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    let visible = canvas.visible_nodes(&viewport, (160.0, 48.0));
    assert!(visible.is_empty());
}

#[test]
fn graph_canvas_visible_nodes_partial() {
    let positions = vec![(50.0, 50.0), (500.0, 500.0), (25.0, 25.0)];
    let canvas = GraphCanvas::new(3, positions, vec![]);
    let viewport = ViewportRect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    let visible = canvas.visible_nodes(&viewport, (160.0, 48.0));
    assert_eq!(visible.len(), 2);
}

#[test]
fn graph_canvas_compute_edge_paths_returns_cloned_paths() {
    let paths = vec![EdgePath {
        source_step: 0,
        target_step: 1,
        start: [0.0, 0.0],
        cp1: [50.0, 0.0],
        cp2: [50.0, 100.0],
        end: [100.0, 100.0],
    }];
    let canvas = GraphCanvas::new(2, vec![], paths.clone());
    let computed = canvas.compute_edge_paths();
    assert_eq!(computed.len(), 1);
    assert_eq!(computed[0].source_step, 0);
    drop(paths);
    assert_eq!(computed[0].source_step, 0);
}

#[test]
fn graph_canvas_focus_jump_returns_true_for_valid_node() {
    let positions = vec![(100.0, 200.0), (300.0, 400.0)];
    let mut canvas = GraphCanvas::new(2, positions, vec![]);
    let result = canvas.focus_jump(1, 1920.0, 1080.0);
    assert!(result);
    let (pan_x, pan_y) = canvas.pan();
    assert!(pan_x.is_finite());
    assert!(pan_y.is_finite());
}

#[test]
fn graph_canvas_focus_jump_returns_false_for_invalid_node() {
    let positions = vec![(100.0, 200.0)];
    let mut canvas = GraphCanvas::new(1, positions, vec![]);
    let result = canvas.focus_jump(99, 1920.0, 1080.0);
    assert!(!result);
}

#[test]
fn graph_canvas_node_layout_position_valid() {
    let positions = vec![(10.0, 20.0), (30.0, 40.0)];
    let canvas = GraphCanvas::new(2, positions, vec![]);
    assert_eq!(canvas.node_layout_position(0), Some((10.0, 20.0)));
    assert_eq!(canvas.node_layout_position(1), Some((30.0, 40.0)));
}

#[test]
fn graph_canvas_node_layout_position_out_of_bounds() {
    let positions = vec![(10.0, 20.0)];
    let canvas = GraphCanvas::new(1, positions, vec![]);
    assert_eq!(canvas.node_layout_position(5), None);
}

#[test]
fn graph_canvas_render_node_card_valid() {
    let positions = vec![(100.0, 200.0)];
    let canvas = GraphCanvas::new(1, positions, vec![]);
    let card = canvas.render_node_card(0);
    assert!(card.is_some());
    let card = card.unwrap();
    assert_eq!(card.step_idx, 0);
    assert_eq!(card.x, 100.0);
    assert_eq!(card.y, 200.0);
    assert_eq!(card.width, 160.0);
    assert_eq!(card.height, 48.0);
    assert!(!card.is_selected);
    assert!(!card.show_taint_overlay);
}

#[test]
fn graph_canvas_render_node_card_out_of_bounds() {
    let positions = vec![];
    let canvas = GraphCanvas::new(0, positions, vec![]);
    assert!(canvas.render_node_card(0).is_none());
}

#[test]
fn graph_canvas_render_node_card_selected() {
    let positions = vec![(100.0, 200.0)];
    let mut canvas = GraphCanvas::new(1, positions, vec![]);
    canvas.set_selected(Some(0));
    let card = canvas.render_node_card(0);
    assert!(card.is_some());
    assert!(card.unwrap().is_selected);
}

#[test]
fn graph_canvas_render_node_card_with_overlay_failed() {
    let positions = vec![(100.0, 200.0)];
    let mut canvas = GraphCanvas::new(1, positions, vec![]);
    canvas.set_node_overlay(0, Some(OverlayState::Failed));
    let card = canvas.render_node_card(0);
    assert!(card.is_some());
    let c = card.unwrap();
    assert_eq!(c.overlay_state, Some(OverlayState::Failed));
    assert_eq!(c.border_color, NodeCardRenderInstr::failure_shadow_color());
}

#[test]
fn graph_canvas_render_node_card_with_taint_overlay() {
    let positions = vec![(100.0, 200.0)];
    let mut canvas = GraphCanvas::new(1, positions, vec![]);
    canvas.set_taint_overlay(true);
    let card = canvas.render_node_card(0);
    assert!(card.is_some());
    assert!(card.unwrap().show_taint_overlay);
}

#[test]
fn graph_canvas_node_status_dot_color_with_overlay() {
    let positions = vec![(100.0, 200.0)];
    let mut canvas = GraphCanvas::new(1, positions, vec![]);
    canvas.set_node_overlay(0, Some(OverlayState::Running));
    let color = canvas.node_status_dot_color(0);
    assert!(color.is_some());
    assert_eq!(color.unwrap(), color::running());
}

#[test]
fn graph_canvas_node_status_dot_color_no_overlay() {
    let positions = vec![(100.0, 200.0)];
    let canvas = GraphCanvas::new(1, positions, vec![]);
    assert!(canvas.node_status_dot_color(0).is_none());
}

#[test]
fn graph_canvas_node_status_dot_color_out_of_bounds() {
    let positions = vec![];
    let canvas = GraphCanvas::new(0, positions, vec![]);
    assert!(canvas.node_status_dot_color(0).is_none());
}

#[test]
fn graph_canvas_node_badges_returns_empty() {
    let positions = vec![(100.0, 200.0)];
    let canvas = GraphCanvas::new(1, positions, vec![]);
    assert!(canvas.node_badges(0).is_empty());
}

// ---------------------------------------------------------------------------
// GraphCanvas — edge rendering
// ---------------------------------------------------------------------------

#[test]
fn graph_canvas_render_edge_valid() {
    let paths = vec![EdgePath {
        source_step: 0,
        target_step: 1,
        start: [0.0, 0.0],
        cp1: [50.0, 0.0],
        cp2: [50.0, 100.0],
        end: [100.0, 100.0],
    }];
    let canvas = GraphCanvas::new(2, vec![], paths);
    let instr = canvas.render_edge("0");
    assert!(instr.is_some());
    let instr = instr.unwrap();
    assert_eq!(instr.source_step, 0);
    assert_eq!(instr.target_step, 1);
    assert_eq!(instr.edge_type, EdgeType::Normal);
}

#[test]
fn graph_canvas_render_edge_invalid_id() {
    let paths = vec![EdgePath {
        source_step: 0,
        target_step: 1,
        start: [0.0, 0.0],
        cp1: [50.0, 0.0],
        cp2: [50.0, 100.0],
        end: [100.0, 100.0],
    }];
    let canvas = GraphCanvas::new(2, vec![], paths);
    assert!(canvas.render_edge("not_a_number").is_none());
}

#[test]
fn graph_canvas_render_edge_out_of_bounds() {
    let paths = vec![EdgePath {
        source_step: 0,
        target_step: 1,
        start: [0.0, 0.0],
        cp1: [50.0, 0.0],
        cp2: [50.0, 100.0],
        end: [100.0, 100.0],
    }];
    let canvas = GraphCanvas::new(2, vec![], paths);
    assert!(canvas.render_edge("99").is_none());
}

#[test]
fn graph_canvas_edge_packet_markers_returns_empty() {
    let canvas = GraphCanvas::new(0, vec![], vec![]);
    assert!(canvas.edge_packet_markers("any").is_empty());
}

#[test]
fn graph_canvas_packet_dot_position_returns_none() {
    let canvas = GraphCanvas::new(0, vec![], vec![]);
    assert!(canvas.packet_dot_position("any", 0.5).is_none());
}

#[test]
fn graph_canvas_animate_packet_dots_no_panic() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    canvas.animate_packet_dots(100.0);
}

// ---------------------------------------------------------------------------
// GraphCanvas — overlay state
// ---------------------------------------------------------------------------

#[test]
fn graph_canvas_set_node_overlay_valid() {
    let positions = vec![(100.0, 200.0)];
    let mut canvas = GraphCanvas::new(1, positions, vec![]);
    canvas.set_node_overlay(0, Some(OverlayState::Succeeded));
    let color = canvas.node_status_dot_color(0);
    assert_eq!(color, Some(color::success()));
}

#[test]
fn graph_canvas_set_node_overlay_clear() {
    let positions = vec![(100.0, 200.0)];
    let mut canvas = GraphCanvas::new(1, positions, vec![]);
    canvas.set_node_overlay(0, Some(OverlayState::Running));
    canvas.set_node_overlay(0, None);
    assert!(canvas.node_status_dot_color(0).is_none());
}

#[test]
fn graph_canvas_set_node_overlay_out_of_bounds_no_panic() {
    let positions = vec![];
    let mut canvas = GraphCanvas::new(0, positions, vec![]);
    canvas.set_node_overlay(0, Some(OverlayState::Failed));
}

// ---------------------------------------------------------------------------
// GraphCanvas — accessors
// ---------------------------------------------------------------------------

#[test]
fn graph_canvas_node_count() {
    let canvas = GraphCanvas::new(7, vec![], vec![]);
    assert_eq!(canvas.node_count(), 7);
}

#[test]
fn graph_canvas_edge_count() {
    let paths = vec![
        EdgePath {
            source_step: 0,
            target_step: 1,
            start: [0.0, 0.0],
            cp1: [50.0, 0.0],
            cp2: [50.0, 100.0],
            end: [100.0, 100.0],
        },
        EdgePath {
            source_step: 1,
            target_step: 2,
            start: [100.0, 100.0],
            cp1: [150.0, 100.0],
            cp2: [150.0, 200.0],
            end: [200.0, 200.0],
        },
    ];
    let canvas = GraphCanvas::new(3, vec![], paths);
    assert_eq!(canvas.edge_count(), 2);
}

#[test]
fn graph_canvas_pan_accessor() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    assert_eq!(canvas.pan(), (0.0, 0.0));
    canvas.set_pan(-100.0, 50.0);
    assert_eq!(canvas.pan(), (-100.0, 50.0));
}

#[test]
fn graph_canvas_zoom_accessor() {
    let mut canvas = GraphCanvas::new(0, vec![], vec![]);
    assert_eq!(canvas.zoom(), 1.0);
    canvas.set_zoom(2.5);
    assert_eq!(canvas.zoom(), 2.5);
}

// ---------------------------------------------------------------------------
// PacketDot — untested methods
// ---------------------------------------------------------------------------

#[test]
fn packet_dot_position_along_bezier_end() {
    let pos = PacketDot::position_along_bezier(
        1.0,
        [0.0, 0.0],
        [50.0, 0.0],
        [50.0, 100.0],
        [100.0, 100.0],
    );
    assert_eq!(pos, [100.0, 100.0]);
}

#[test]
fn packet_dot_position_along_bezier_quarter() {
    let pos = PacketDot::position_along_bezier(
        0.25,
        [0.0, 0.0],
        [50.0, 0.0],
        [50.0, 100.0],
        [100.0, 100.0],
    );
    assert!(pos[0] > 0.0 && pos[0] < 100.0);
    assert!(pos[1] > 0.0 && pos[1] < 100.0);
}

#[test]
fn packet_dot_position_along_bezier_three_quarter() {
    let pos = PacketDot::position_along_bezier(
        0.75,
        [0.0, 0.0],
        [50.0, 0.0],
        [50.0, 100.0],
        [100.0, 100.0],
    );
    assert!(pos[0] > 0.0 && pos[0] < 100.0);
    assert!(pos[1] > 0.0 && pos[1] < 100.0);
}

#[test]
fn packet_dot_finish() {
    let mut dot = PacketDot::new("e".to_string());
    dot.finish();
    assert_eq!(dot.t, 1.0);
    assert!(!dot.active);
}

#[test]
fn packet_dot_speed_default() {
    let dot = PacketDot::new("e".to_string());
    assert_eq!(dot.speed, 0.2);
}

// ---------------------------------------------------------------------------
// PacketMarkerInstr
// ---------------------------------------------------------------------------

#[test]
fn packet_marker_instr_new_t_clamped_low() {
    let marker = PacketMarkerInstr::new(-0.5);
    assert_eq!(marker.t, 0.0);
    assert_eq!(marker.color, color::active_cyan());
    assert_eq!(marker.size, 6.0);
}

#[test]
fn packet_marker_instr_new_t_clamped_high() {
    let marker = PacketMarkerInstr::new(2.0);
    assert_eq!(marker.t, 1.0);
}

#[test]
fn packet_marker_instr_new_t_mid() {
    let marker = PacketMarkerInstr::new(0.5);
    assert_eq!(marker.t, 0.5);
}

// ---------------------------------------------------------------------------
// PacketDotManager — eviction and partial progress
// ---------------------------------------------------------------------------

#[test]
fn packet_dot_manager_eviction_on_overflow() {
    let mut mgr = PacketDotManager::new();
    for i in 0..600 {
        mgr.add_dot(format!("edge{}", i));
    }
    assert_eq!(mgr.total_count(), 512);
    assert_eq!(mgr.active_count(), 512);
}

#[test]
fn packet_dot_manager_animate_partial_progress() {
    let mut mgr = PacketDotManager::new();
    mgr.add_dot("edge1".to_string());
    mgr.animate(5000.0);
    assert_eq!(mgr.active_count(), 0);
    assert_eq!(mgr.total_count(), 1);
}

#[test]
fn packet_dot_manager_animate_no_progress_short_delta() {
    let mut mgr = PacketDotManager::new();
    mgr.add_dot("edge1".to_string());
    mgr.animate(100.0);
    assert_eq!(mgr.active_count(), 1);
}

#[test]
fn packet_dot_manager_reset_all_restores_active() {
    let mut mgr = PacketDotManager::new();
    mgr.add_dot("edge1".to_string());
    mgr.animate(5000.0);
    assert_eq!(mgr.active_count(), 0);
    mgr.reset_all();
    assert_eq!(mgr.active_count(), 1);
}

#[test]
fn packet_dot_manager_multiple_dots_both_active_after_short_animate() {
    let mut mgr = PacketDotManager::new();
    mgr.add_dot("e1".to_string());
    mgr.add_dot("e2".to_string());
    mgr.animate(2500.0);
    assert_eq!(mgr.active_count(), 2);
    assert_eq!(mgr.total_count(), 2);
}

#[test]
fn packet_dot_manager_both_finish_after_long_animate() {
    let mut mgr = PacketDotManager::new();
    mgr.add_dot("e1".to_string());
    mgr.add_dot("e2".to_string());
    mgr.animate(5000.0);
    assert_eq!(mgr.active_count(), 0);
    assert_eq!(mgr.total_count(), 2);
}

#[test]
fn packet_dot_manager_default() {
    let mgr = PacketDotManager::default();
    assert_eq!(mgr.total_count(), 0);
}
