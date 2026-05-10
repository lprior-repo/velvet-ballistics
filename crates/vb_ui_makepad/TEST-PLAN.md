# Test Plan: vb_ui_makepad

## Crate Status
- **cargo test -p vb_ui_makepad**: 0 passed (0 tests confirmed)
- **Required**: ≥425 tests (5x density for 85 pub fns)
- **VERDICT**: REJECTED — 0 tests exist

---

## Section 1 — Behavior Inventory

### Error Enum (12 variants)

| # | Variant | Trigger Condition |
|---|---------|-------------------|
| E1 | `InvalidToken(String)` | TOML parse failure, missing color/layout/radius/shadow/space/type key |
| E2 | `NavItemNotFound(String)` | Nav index lookup out of bounds (future) |
| E3 | `InvalidScreenTransition(String)` | Invalid screen state transition (future) |
| E4 | `TokenParseError(String)` | Hex parse: invalid char, invalid pair, invalid length, overflow |
| E5 | `InvalidFlowDocument(String)` | Malformed flow doc (future) |
| E6 | `LayoutNotComputed` | Layout not ready when queried (future) |
| E7 | `NodeNotFound(usize)` | Node index not found in GraphCanvas |
| E8 | `InvalidViewport` | Viewport dimensions invalid |
| E9 | `AnimationOverflow` | Animation tick overflow (future) |
| E10 | `ViewHidden` | View is hidden (future) |
| E11 | `MissingDesignToken(String)` | Design token not found |

### tokens.rs Behaviors

| # | Subject | Action | Outcome when [condition] |
|---|---------|--------|-------------------------|
| T1 | `parse_hex` | parse `"#FF0000"` | returns `[1.0, 0.0, 0.0, 1.0]` |
| T2 | `parse_hex` | parse `"#FF000080"` | returns `[1.0, 0.0, 0.0, 0.502]` |
| T3 | `parse_hex` | parse `"FF0000"` (no #) | returns `[1.0, 0.0, 0.0, 1.0]` |
| T4 | `parse_hex` | parse `"#GG0000"` | returns `Err(Error::TokenParseError("invalid hex char"))` |
| T5 | `parse_hex` | parse `"#F000"` (3 chars) | returns `Err(Error::TokenParseError("invalid hex length: 3"))` |
| T6 | `parse_hex` | parse `"#"` (empty) | returns `Err(Error::TokenParseError("hex too short"))` |
| T7 | `parse_hex` | parse `"#FFFF"` (4 chars) | returns `Err(Error::TokenParseError("invalid hex length: 4"))` |
| T8 | `parse_hex` | parse case-insensitive `"#ff0000"` | returns `[1.0, 0.0, 0.0, 1.0]` |
| T9 | `parse_hex` | parse `"  #FF0000  "` (whitespace) | returns `[1.0, 0.0, 0.0, 1.0]` |
| T10 | `Tokens::parse()` | valid `TOKENS_TOML` | returns `Ok(ParsedTokens)` with all fields |
| T11 | `ParsedTokens::from_toml` | empty string | returns `Err(Error::InvalidToken(...))` |
| T12 | `ParsedTokens::from_toml` | missing `color` section | returns `Err(Error::InvalidToken("missing color.background_board"))` |
| T13 | `ParsedTokens::from_toml` | missing `layout.sidebar_width` | returns `Err(Error::InvalidToken("missing layout.sidebar_width"))` |
| T14 | `ParsedTokens::from_toml` | color value not string | returns `Err(Error::TokenParseError("color.background_board not string"))` |
| T15 | `ParsedTokens::from_toml` | integer layout value | returns `Ok(ParsedTokens)` with correct f64 conversion |
| T16 | `ParsedTokens::from_toml` | integer overflow in u32 | returns `Err(Error::TokenParseError("type.size_11 overflow"))` |
| T17 | `color::background_board()` | cached token loaded | returns `[f32; 4]` from parsed or fallback |
| T18 | `color::surface_glass()` | glass with alpha 0.8 | returns `[1.0, 1.0, 1.0, 0.8]` |
| T19 | `layout::SIDEBAR_WIDTH` | constant access | equals `246.0` |
| T20 | `radius::CARD` | constant access | equals `16.0` |
| T21 | `shadow::CARD` | constant access | equals `"0 8 24 rgba(16,24,40,0.08)"` |
| T22 | `space::PX_4`..`PX_40` | constant access | equals `4.0`..`40.0` |

### shell.rs Behaviors

| # | Subject | Action | Outcome when [condition] |
|---|---------|--------|-------------------------|
| S1 | `ShellNav::label` | `ShellNav::Overview.label()` | returns `"Overview"` |
| S2 | `ShellNav::label` | `ShellNav::WorkflowGraph.label()` | returns `"Workflow Graph"` |
| S3 | `ShellNav::nav_color` | `ShellNav::Overview.nav_color()` | returns `[0.145, 0.388, 0.922, 1.0]` |
| S4 | `ShellNav::screen` | `ShellNav::Overview.screen()` | returns `Screen::ExecutionOverview` |
| S5 | `ShellNav::screen` | `ShellNav::Incidents.screen()` | returns `Screen::IncidentFailureConsole` |
| S6 | `ShellStatusChip::new` | `ShellStatusChip::new("Running", [0.1, 0.5, 0.9, 1.0])` | returns struct with label and color |
| S7 | `Screen::splash_name` | `Screen::ExecutionOverview.splash_name()` | returns `"ExecutionOverview"` |
| S8 | `Screen::nav_label` | `Screen::ExecutionOverview.nav_label()` | returns `"Overview"` |
| S9 | `Screen::is_shell_screen` | any Screen variant | returns `true` |
| S10 | `AppShell::new` | construction | returns `Ok(AppShell)` with Overview nav |
| S11 | `AppShell::set_active_nav` | set to `WorkflowGraph` | updates `active_nav` |
| S12 | `AppShell::active_nav` | after construction | returns `ShellNav::Overview` |
| S13 | `AppShell::nav_item_rect` | index 0 | returns `Rect { x:0, y:0, width:246, height:56 }` |
| S14 | `AppShell::nav_item_rect` | index 3 | returns `Rect { x:0, y:168, width:246, height:56 }` |
| S15 | `AppShell::nav_item_rect` | large index (u32::MAX) | returns `Rect { y: 0.0, ... }` (safe cast) |
| S16 | `AppShell::topbar_rect` | construction | returns `Rect { x:246, y:0, width:1674, height:78 }` |
| S17 | `AppShell::content_rect` | construction | returns `Rect { x:246, y:78, width:1674, height:1002 }` |

### packet_dot.rs Behaviors

| # | Subject | Action | Outcome when [condition] |
|---|---------|--------|-------------------------|
| P1 | `PacketDot::new` | `PacketDot::new("e1".into())` | returns dot with `t=0.0`, `speed=0.2`, `active=true` |
| P2 | `PacketDot::position_along_bezier` | t=0.0 | returns `start` |
| P3 | `PacketDot::position_along_bezier` | t=1.0 | returns `end` |
| P4 | `PacketDot::position_along_bezier` | t=0.5 with linear curve | returns midpoint `[50.0, 50.0]` |
| P5 | `PacketDot::color` | any dot | returns `color::active_cyan()` |
| P6 | `PacketDot::size` | any dot | returns `6.0` |
| P7 | `PacketDot::reset` | called on dot | sets `t=0.0`, `active=true` |
| P8 | `PacketDot::finish` | called on dot | sets `t=1.0`, `active=false` |
| P9 | `AnimationTick::new` | `AnimationTick::new(500.0)` | returns struct with `delta_ms=500.0` |
| P10 | `AnimationTick::normalized_delta` | `AnimationTick::new(500.0)` | returns `0.5` |
| P11 | `PacketDotManager::new` | construction | returns manager with capacity 512 |
| P12 | `PacketDotManager::add_dot` | add first dot | `active_count` becomes 1 |
| P13 | `PacketDotManager::add_dot` | exceed 512 dots | removes oldest, adds new |
| P14 | `PacketDotManager::animate` | advance 1000ms at speed 0.2 | dot `t` advances by `0.2` |
| P15 | `PacketDotManager::animate` | dot reaches t>=1.0 | dot becomes inactive |
| P16 | `PacketDotManager::active_count` | 3 active, 2 inactive | returns `3` |
| P17 | `PacketDotManager::total_count` | 5 total dots | returns `5` |
| P18 | `PacketDotManager::clear` | 5 dots | removes all dots |
| P19 | `PacketDotManager::reset_all` | 3 dots | all dots reset to `t=0.0`, `active=true` |

### graph_canvas.rs Behaviors

| # | Subject | Action | Outcome when [condition] |
|---|---------|--------|-------------------------|
| G1 | `ViewportRect::intersects` | disjoint rects | returns `false` |
| G2 | `ViewportRect::intersects` | overlapping rects | returns `true` |
| G3 | `ViewportRect::intersects` | adjacent edge (touching) | returns `false` (right <= left) |
| G4 | `GraphCanvas::new` | valid input | returns canvas with correct counts |
| G5 | `GraphCanvas::viewport_rect` | zoom=1.0, 1920x1080 | returns rect matching screen |
| G6 | `GraphCanvas::viewport_rect` | zoom=2.0, 1920x1080 | returns 2x wider/taller rect |
| G7 | `GraphCanvas::viewport_rect` | zoom=0.0 (clamped) | uses `1.0` as fallback |
| G8 | `GraphCanvas::visible_nodes` | nodes outside viewport | not included in result |
| G9 | `GraphCanvas::visible_nodes` | node intersecting viewport | included with correct bounds |
| G10 | `GraphCanvas::visible_nodes` | empty node_positions | returns empty vec |
| G11 | `GraphCanvas::compute_edge_paths` | valid canvas | returns cloned edge_paths |
| G12 | `GraphCanvas::set_pan` | pan to (100, 200) | updates `pan_x=100`, `pan_y=200` |
| G13 | `GraphCanvas::set_zoom` | zoom=0.05 (< MIN) | clamped to `0.1` |
| G14 | `GraphCanvas::set_zoom` | zoom=10.0 (> MAX) | clamped to `5.0` |
| G15 | `GraphCanvas::set_zoom` | zoom=2.0 | updates `zoom=2.0` |
| G16 | `GraphCanvas::zoom_in` | factor 1.5 on zoom 1.0 | zoom becomes `1.5` |
| G17 | `GraphCanvas::zoom_out` | factor 2.0 on zoom 1.0 | zoom becomes `0.5` |
| G18 | `GraphCanvas::zoom_reset` | any zoom | resets to `1.0` |
| G19 | `GraphCanvas::zoom_percentage` | zoom=1.5 | returns `"150%"` |
| G20 | `GraphCanvas::set_selected` | `Some(3)` | `selected` becomes `3` |
| G21 | `GraphCanvas::focus_jump` | valid node id | centers viewport on node |
| G22 | `GraphCanvas::focus_jump` | invalid node id | returns `false` |
| G23 | `GraphCanvas::node_layout_position` | valid index | returns `Some((x, y))` |
| G24 | `GraphCanvas::node_layout_position` | out-of-bounds | returns `None` |
| G25 | `GraphCanvas::render_node_card` | valid index | returns `Some(NodeCardRenderInstr)` |
| G26 | `GraphCanvas::render_node_card` | out-of-bounds | returns `None` |
| G27 | `GraphCanvas::node_status_dot_color` | Failed overlay | returns `color::failure()` |
| G28 | `GraphCanvas::node_badges` | any index | returns `Vec::new()` |
| G29 | `GraphCanvas::render_edge` | valid edge id | returns `Some(EdgeRenderInstr)` |
| G30 | `GraphCanvas::render_edge` | invalid edge id | returns `None` |
| G31 | `GraphCanvas::edge_packet_markers` | any edge id | returns `Vec::new()` |
| G32 | `GraphCanvas::packet_dot_position` | any inputs | returns `None` |
| G33 | `GraphCanvas::set_node_overlay` | valid index, `Some(OverlayState::Failed)` | overlay set |
| G34 | `GraphCanvas::set_node_overlay` | out-of-bounds index | no-op |
| G35 | `GraphCanvas::set_taint_overlay` | `true` | `taint_overlay_active=true` |
| G36 | `GraphCanvas::node_count` | 5 nodes | returns `5` |
| G37 | `GraphCanvas::edge_count` | 7 edges | returns `7` |
| G38 | `GraphCanvas::pan` | after set_pan | returns `(pan_x, pan_y)` |
| G39 | `GraphCanvas::zoom` | after set_zoom | returns current zoom |
| G40 | `GraphCanvas::selected` | after set_selected | returns `Some(idx)` |

### graph_node.rs Behaviors

| # | Subject | Action | Outcome when [condition] |
|---|---------|--------|-------------------------|
| N1 | `OverlayState::glow_color` | `Pending` | returns `color::pending()` |
| N2 | `OverlayState::glow_color` | `Running` | returns `color::running()` |
| N3 | `OverlayState::glow_color` | `Succeeded` | returns `color::success()` |
| N4 | `OverlayState::glow_color` | `Failed` | returns `color::failure()` |
| N5 | `OverlayState::glow_color` | `Cancelled` | returns `color::text_tertiary()` |
| N6 | `OverlayState::glow_radius` | `Running` | returns `4.0` |
| N7 | `OverlayState::glow_radius` | `Failed` | returns `6.0` |
| N8 | `NodeBadge::label` | `ActionId(42)` | returns `"A42"` |
| N9 | `NodeBadge::label` | `RetryMax(3)` | returns `"R3"` |
| N10 | `NodeBadge::label` | `Timeout(30)` | returns `"T30s"` |
| N11 | `NodeBadge::label` | `SecretSensitive` | returns `"S"` |
| N12 | `NodeBadge::label` | `StrictDurable` | returns `"D"` |
| N13 | `NodeBadge::label` | `RecentFailures(5)` | returns `"!5"` |
| N14 | `NodeBadge::color` | `ActionId(_)` | returns `[1.0, 0.42, 0.0, 1.0]` |
| N15 | `NodeBadge::color` | `Timeout(_)` | returns `[1.0, 0.027, 0.227, 1.0]` |
| N16 | `NodeCardRenderInstr::focus_shadow_color` | static call | returns `[0.122, 0.478, 0.961, 1.0]` |
| N17 | `NodeCardRenderInstr::failure_shadow_color` | static call | returns `[0.898, 0.282, 0.302, 1.0]` |
| N18 | `NodeCardRenderInstr::taint_overlay_color` | static call | returns `color::taint()` |
| N19 | `GraphNode::card_dimensions` | static call | returns `(160.0, 48.0)` |
| N20 | `GraphNode::header_dimensions` | static call | returns `(160.0, 24.0)` |
| N21 | `GraphNode::badge_size` | static call | returns `16.0` |

### graph_edge.rs Behaviors

| # | Subject | Action | Outcome when [condition] |
|---|---------|--------|-------------------------|
| E1 | `EdgeType::color` | `Normal` | returns `[0.0, 0.6, 0.8, 1.0]` |
| E2 | `EdgeType::color` | `Branch` | returns `[0.694, 0.302, 1.0, 1.0]` |
| E3 | `EdgeType::color` | `ErrorRoute` | returns `[0.6, 0.1, 0.1, 1.0]` |
| E4 | `EdgeType::color` | `RetryRoute` | returns `[1.0, 0.9, 0.0, 1.0]` |
| E5 | `EdgeType::color` | `Join` | returns `[0.176, 0.42, 1.0, 1.0]` |
| E6 | `EdgeType::color` | `LoopBack` | returns `[0.0, 0.898, 0.78, 1.0]` |
| E7 | `EdgeType::is_dashed` | `Branch` | returns `true` |
| E8 | `EdgeType::is_dashed` | `Normal` | returns `false` |
| E9 | `EdgeRenderInstr::from_edge_path` | valid inputs | returns struct with correct edge_type |
| E10 | `EdgeRenderInstr::with_label` | add label | `label` becomes `Some(label)` |
| E11 | `PacketMarkerInstr::new` | t=0.5 | returns struct with clamped t |
| E12 | `PacketMarkerInstr::new` | t=-0.5 | returns struct with t=0.0 |
| E13 | `PacketMarkerInstr::new` | t=1.5 | returns struct with t=1.0 |
| E14 | `GraphEdge::DEFAULT_WIDTH` | constant | equals `2.0` |
| E15 | `GraphEdge::HIGHLIGHT_WIDTH` | constant | equals `3.0` |
| E16 | `GraphEdge::PACKET_SIZE` | constant | equals `6.0` |

---

## Section 2 — Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| **Unit** (`#[cfg(test)]`) | ~260 tests | Pure functions: `parse_hex`, `from_toml`, `position_along_bezier`, `intersects`, `glow_color`, `glow_radius`, `label`, `color`, `is_dashed`, all `const fn` accessors |
| **Integration** (`/tests/`) | ~140 tests | `AppShell`, `GraphCanvas`, `PacketDotManager` — real types, stateful interactions |
| **Fuzz** | 2 targets | `parse_hex` (string→[f32;4]), `from_toml` (string→ParsedTokens) |
| **Proptest** | 8 invariants | `parse_hex` range, `viewport_rect` bounds, `visible_nodes` subset property, `set_zoom` clamp, `position_along_bezier` endpoint invariants |
| **Static** | clippy, fmt | Already enforced via `#![forbid(unsafe_code)]` |

**Target ratio**: ~60% integration, ~30% unit, ~5% fuzz, ~5% proptest

---

## Section 3 — BDD Scenarios (Unit)

### tokens.rs

```rust
/// Behavior: parse_hex returns correct RGBA when input is valid 6-char hex
fn parse_hex_returns_correct_rgba_when_input_is_valid_six_char_hex()

/// Behavior: parse_hex returns correct RGBA when input is valid 8-char hex with alpha
fn parse_hex_returns_correct_rgba_when_input_is_valid_eight_char_hex()

/// Behavior: parse_hex returns error when input contains invalid hex characters
fn parse_hex_returns_error_when_input_contains_invalid_hex_characters()

/// Behavior: parse_hex returns error when input has wrong length
fn parse_hex_returns_error_when_input_has_invalid_length()

/// Behavior: parse_hex is case-insensitive
fn parse_hex_is_case_insensitive()

/// Behavior: parse_hex trims whitespace
fn parse_hex_trims_whitespace()

/// Behavior: parse_hex returns error when hex string is too short
fn parse_hex_returns_error_when_hex_too_short()

/// Behavior: Tokens::parse returns Ok when TOKENS_TOML is valid
fn tokens_parse_returns_ok_when_toml_valid()

/// Behavior: ParsedTokens::from_toml returns error when color section missing
fn parsed_tokens_returns_error_when_color_section_missing()

/// Behavior: ParsedTokens::from_toml returns error when layout key missing
fn parsed_tokens_returns_error_when_layout_key_missing()

/// Behavior: ParsedTokens::from_toml returns error when color value not string
fn parsed_tokens_returns_error_when_color_value_not_string()

/// Behavior: ParsedTokens::from_toml returns error on integer overflow
fn parsed_tokens_returns_error_on_integer_overflow()

/// Behavior: color accessors return correct [f32; 4] values
fn color_accessors_return_correct_rgba_values()

/// Behavior: layout constants have correct values
fn layout_constants_have_correct_values()

/// Behavior: radius constant equals 16.0
fn radius_card_equals_sixteen()

/// Behavior: shadow constant equals expected string
fn shadow_card_equals_expected_string()

/// Behavior: space constants form correct sequence 4..40
fn space_constants_form_correct_sequence()
```

### shell.rs

```rust
/// Behavior: ShellNav label returns correct string for each variant
fn shell_nav_label_returns_correct_string_for_each_variant()

/// Behavior: ShellNav nav_color returns correct [f32; 4] for each variant
fn shell_nav_nav_color_returns_correct_rgba_for_each_variant()

/// Behavior: ShellNav screen returns correct Screen for each variant
fn shell_nav_screen_returns_correct_screen_for_each_variant()

/// Behavior: ShellStatusChip new returns struct with correct fields
fn shell_status_chip_new_returns_correct_struct()

/// Behavior: Screen splash_name returns correct string for each variant
fn screen_splash_name_returns_correct_string_for_each_variant()

/// Behavior: Screen nav_label returns correct string for each variant
fn screen_nav_label_returns_correct_string_for_each_variant()

/// Behavior: Screen is_shell_screen always returns true
fn screen_is_shell_screen_always_returns_true()

/// Behavior: AppShell new returns Ok with Overview nav
fn app_shell_new_returns_ok_with_overview_nav()

/// Behavior: AppShell set_active_nav updates active_nav
fn app_shell_set_active_nav_updates_active_nav()

/// Behavior: AppShell active_nav returns current nav
fn app_shell_active_nav_returns_current_nav()

/// Behavior: AppShell nav_item_rect returns correct Rect for index 0
fn app_shell_nav_item_rect_returns_correct_rect_for_index_zero()

/// Behavior: AppShell nav_item_rect returns correct Rect for index 3
fn app_shell_nav_item_rect_returns_correct_rect_for_index_three()

/// Behavior: AppShell nav_item_rect handles large index safely
fn app_shell_nav_item_rect_handles_large_index_safely()

/// Behavior: AppShell topbar_rect returns correct Rect
fn app_shell_topbar_rect_returns_correct_rect()

/// Behavior: AppShell content_rect returns correct Rect
fn app_shell_content_rect_returns_correct_rect()
```

### packet_dot.rs

```rust
/// Behavior: PacketDot new returns dot with default values
fn packet_dot_new_returns_dot_with_default_values()

/// Behavior: PacketDot position_along_bezier returns start at t=0
fn packet_dot_position_along_bezier_returns_start_at_t_zero()

/// Behavior: PacketDot position_along_bezier returns end at t=1
fn packet_dot_position_along_bezier_returns_end_at_t_one()

/// Behavior: PacketDot position_along_bezier returns midpoint at t=0.5
fn packet_dot_position_along_bezier_returns_midpoint_at_t_point_five()

/// Behavior: PacketDot color returns active_cyan
fn packet_dot_color_returns_active_cyan()

/// Behavior: PacketDot size returns 6.0
fn packet_dot_size_returns_six()

/// Behavior: PacketDot reset sets t to 0 and active to true
fn packet_dot_reset_sets_t_to_zero_and_active_to_true()

/// Behavior: PacketDot finish sets t to 1 and active to false
fn packet_dot_finish_sets_t_to_one_and_active_to_false()

/// Behavior: AnimationTick new stores delta_ms
fn animation_tick_new_stores_delta_ms()

/// Behavior: AnimationTick normalized_delta divides by 1000
fn animation_tick_normalized_delta_divides_by_1000()

/// Behavior: PacketDotManager new creates manager with capacity 512
fn packet_dot_manager_new_creates_manager_with_capacity_512()

/// Behavior: PacketDotManager add_dot increments active_count
fn packet_dot_manager_add_dot_increments_active_count()

/// Behavior: PacketDotManager add_dot evicts oldest when at capacity
fn packet_dot_manager_add_dot_evicts_oldest_when_at_capacity()

/// Behavior: PacketDotManager animate advances dot t by speed * normalized_delta
fn packet_dot_manager_animate_advances_dot_t()

/// Behavior: PacketDotManager animate deactivates dot when t >= 1.0
fn packet_dot_manager_animate_deactivates_dot_at_t_one()

/// Behavior: PacketDotManager active_count returns only active dots
fn packet_dot_manager_active_count_returns_only_active_dots()

/// Behavior: PacketDotManager total_count returns all dots
fn packet_dot_manager_total_count_returns_all_dots()

/// Behavior: PacketDotManager clear removes all dots
fn packet_dot_manager_clear_removes_all_dots()

/// Behavior: PacketDotManager reset_all resets all dots to t=0
fn packet_dot_manager_reset_all_resets_all_dots()
```

### graph_canvas.rs

```rust
/// Behavior: ViewportRect intersects returns false for disjoint rects
fn viewport_rect_intersects_returns_false_for_disjoint_rects()

/// Behavior: ViewportRect intersects returns true for overlapping rects
fn viewport_rect_intersects_returns_true_for_overlapping_rects()

/// Behavior: ViewportRect intersects returns false for adjacent rects (touching edge)
fn viewport_rect_intersects_returns_false_for_adjacent_rects()

/// Behavior: GraphCanvas new sets correct node_count and edge_count
fn graph_canvas_new_sets_correct_counts()

/// Behavior: GraphCanvas viewport_rect returns correct rect at zoom 1.0
fn graph_canvas_viewport_rect_returns_correct_rect_at_zoom_one()

/// Behavior: GraphCanvas viewport_rect returns 2x rect at zoom 0.5
fn graph_canvas_viewport_rect_returns_2x_rect_at_zoom_point_five()

/// Behavior: GraphCanvas viewport_rect handles zero zoom safely
fn graph_canvas_viewport_rect_handles_zero_zoom_safely()

/// Behavior: GraphCanvas visible_nodes excludes nodes outside viewport
fn graph_canvas_visible_nodes_excludes_nodes_outside_viewport()

/// Behavior: GraphCanvas visible_nodes includes nodes intersecting viewport
fn graph_canvas_visible_nodes_includes_intersecting_nodes()

/// Behavior: GraphCanvas visible_nodes returns empty for empty positions
fn graph_canvas_visible_nodes_returns_empty_for_no_positions()

/// Behavior: GraphCanvas compute_edge_paths returns cloned paths
fn graph_canvas_compute_edge_paths_returns_cloned_paths()

/// Behavior: GraphCanvas set_pan updates pan coordinates
fn graph_canvas_set_pan_updates_pan_coordinates()

/// Behavior: GraphCanvas set_zoom clamps to MIN_ZOOM (0.1)
fn graph_canvas_set_zoom_clamps_to_min_zoom()

/// Behavior: GraphCanvas set_zoom clamps to MAX_ZOOM (5.0)
fn graph_canvas_set_zoom_clamps_to_max_zoom()

/// Behavior: GraphCanvas set_zoom accepts valid zoom value
fn graph_canvas_set_zoom_accepts_valid_zoom()

/// Behavior: GraphCanvas zoom_in multiplies zoom by factor
fn graph_canvas_zoom_in_multiplies_zoom_by_factor()

/// Behavior: GraphCanvas zoom_out divides zoom by factor
fn graph_canvas_zoom_out_divides_zoom_by_factor()

/// Behavior: GraphCanvas zoom_reset sets zoom to 1.0
fn graph_canvas_zoom_reset_sets_zoom_to_default()

/// Behavior: GraphCanvas zoom_percentage formats zoom as percentage string
fn graph_canvas_zoom_percentage_formats_as_percentage_string()

/// Behavior: GraphCanvas set_selected updates selected index
fn graph_canvas_set_selected_updates_selected_index()

/// Behavior: GraphCanvas focus_jump centers viewport on valid node
fn graph_canvas_focus_jump_centers_viewport_on_valid_node()

/// Behavior: GraphCanvas focus_jump returns false for invalid node
fn graph_canvas_focus_jump_returns_false_for_invalid_node()

/// Behavior: GraphCanvas node_layout_position returns position for valid index
fn graph_canvas_node_layout_position_returns_position_for_valid_index()

/// Behavior: GraphCanvas node_layout_position returns None for invalid index
fn graph_canvas_node_layout_position_returns_none_for_invalid_index()

/// Behavior: GraphCanvas render_node_card returns Some for valid index
fn graph_canvas_render_node_card_returns_some_for_valid_index()

/// Behavior: GraphCanvas render_node_card returns None for invalid index
fn graph_canvas_render_node_card_returns_none_for_invalid_index()

/// Behavior: GraphCanvas node_status_dot_color returns failure color for Failed overlay
fn graph_canvas_node_status_dot_color_returns_failure_color()

/// Behavior: GraphCanvas node_badges returns empty vec
fn graph_canvas_node_badges_returns_empty_vec()

/// Behavior: GraphCanvas render_edge returns Some for valid edge id
fn graph_canvas_render_edge_returns_some_for_valid_edge_id()

/// Behavior: GraphCanvas render_edge returns None for invalid edge id
fn graph_canvas_render_edge_returns_none_for_invalid_edge_id()

/// Behavior: GraphCanvas edge_packet_markers returns empty vec
fn graph_canvas_edge_packet_markers_returns_empty_vec()

/// Behavior: GraphCanvas packet_dot_position returns None
fn graph_canvas_packet_dot_position_returns_none()

/// Behavior: GraphCanvas set_node_overlay updates overlay for valid index
fn graph_canvas_set_node_overlay_updates_overlay_for_valid_index()

/// Behavior: GraphCanvas set_node_overlay is no-op for invalid index
fn graph_canvas_set_node_overlay_is_noop_for_invalid_index()

/// Behavior: GraphCanvas set_taint_overlay activates taint overlay
fn graph_canvas_set_taint_overlay_activates_taint_overlay()

/// Behavior: GraphCanvas node_count returns correct count
fn graph_canvas_node_count_returns_correct_count()

/// Behavior: GraphCanvas edge_count returns correct count
fn graph_canvas_edge_count_returns_correct_count()

/// Behavior: GraphCanvas pan returns current pan coordinates
fn graph_canvas_pan_returns_current_pan_coordinates()

/// Behavior: GraphCanvas zoom returns current zoom
fn graph_canvas_zoom_returns_current_zoom()

/// Behavior: GraphCanvas selected returns current selected index
fn graph_canvas_selected_returns_current_selected_index()
```

### graph_node.rs

```rust
/// Behavior: OverlayState glow_color returns correct color for each variant
fn overlay_state_glow_color_returns_correct_color_for_each_variant()

/// Behavior: OverlayState glow_radius returns correct radius for each variant
fn overlay_state_glow_radius_returns_correct_radius_for_each_variant()

/// Behavior: NodeBadge label returns correct string for each variant
fn node_badge_label_returns_correct_string_for_each_variant()

/// Behavior: NodeBadge color returns correct [f32; 4] for each variant
fn node_badge_color_returns_correct_rgba_for_each_variant()

/// Behavior: NodeCardRenderInstr focus_shadow_color returns [0.122, 0.478, 0.961, 1.0]
fn node_card_render_instr_focus_shadow_color_returns_blue_shadow()

/// Behavior: NodeCardRenderInstr failure_shadow_color returns [0.898, 0.282, 0.302, 1.0]
fn node_card_render_instr_failure_shadow_color_returns_red_shadow()

/// Behavior: NodeCardRenderInstr taint_overlay_color returns taint color
fn node_card_render_instr_taint_overlay_color_returns_taint_color()

/// Behavior: GraphNode card_dimensions returns (160.0, 48.0)
fn graph_node_card_dimensions_returns_160x48()

/// Behavior: GraphNode header_dimensions returns (160.0, 24.0)
fn graph_node_header_dimensions_returns_160x24()

/// Behavior: GraphNode badge_size returns 16.0
fn graph_node_badge_size_returns_16()
```

### graph_edge.rs

```rust
/// Behavior: EdgeType color returns correct [f32; 4] for each variant
fn edge_type_color_returns_correct_rgba_for_each_variant()

/// Behavior: EdgeType is_dashed returns true for Branch, ErrorRoute, RetryRoute
fn edge_type_is_dashed_returns_true_for_branch_error_retry()

/// Behavior: EdgeType is_dashed returns false for Normal, Join, LoopBack
fn edge_type_is_dashed_returns_false_for_normal_join_loop()

/// Behavior: EdgeRenderInstr from_edge_path creates correct struct
fn edge_render_instr_from_edge_path_creates_correct_struct()

/// Behavior: EdgeRenderInstr with_label sets label field
fn edge_render_instr_with_label_sets_label()

/// Behavior: PacketMarkerInstr new clamps t to [0.0, 1.0] range
fn packet_marker_instr_new_clamps_t_to_valid_range()

/// Behavior: GraphEdge constants have correct values
fn graph_edge_constants_have_correct_values()
```

---

## Section 4 — Proptest Invariants

### parse_hex Invariants

```rust
// Property: parse_hex(#RRGGBB) always returns all channels in [0.0, 1.0]
prop_parse_hex_rgba_channels_in_range

// Property: parse_hex(#RRGGBBAA) alpha channel is correctly computed
prop_parse_hex_alpha_channel_correct

// Property: Invalid hex chars always return TokenParseError
prop_invalid_hex_chars_always_error

// Strategy: hex_strings() — 6 or 8 char strings with chars [0-9A-Fa-f]
```

### from_toml Invariants

```rust
// Property: Valid TOML always parses all sections completely
prop_valid_toml_parses_all_sections

// Property: Missing any required key returns InvalidToken
prop_missing_required_key_returns_error

// Strategy: well_formed_toml() — generates valid TOML with all required sections
```

### viewport_rect Invariants

```rust
// Property: viewport_rect width = screen_width / zoom (clamped)
prop_viewport_rect_width_matches_zoom

// Property: viewport_rect height = screen_height / zoom (clamped)
prop_viewport_rect_height_matches_zoom

// Property: zoom always in [MIN_ZOOM, MAX_ZOOM] after set_zoom
prop_zoom_always_in_valid_range_after_set
```

### visible_nodes Invariants

```rust
// Property: visible_nodes result is always subset of all nodes
prop_visible_nodes_is_subset_of_all_nodes

// Property: visible_nodes count <= node_positions count
prop_visible_nodes_count_never_exceeds_total

// Property: all returned node bounds intersect viewport
prop_visible_nodes_all_intersect_viewport
```

### position_along_bezier Invariants

```rust
// Property: t=0.0 always returns exactly start point
prop_bezier_at_t0_equals_start

// Property: t=1.0 always returns exactly end point
prop_bezier_at_t1_equals_end

// Property: result x,y are always finite (no NaN/Inf)
prop_bezier_result_always_finite

// Strategy: bezier_inputs() — t in [0,1], any valid [f64;2] control points
```

### GraphCanvas Zoom Invariants

```rust
// Property: zoom is always in [MIN_ZOOM, MAX_ZOOM] after any zoom operation
prop_zoom_always_bounded_after_operations

// Property: zoom_percentage format string ends with "%"
prop_zoom_percentage_format_correct

// Property: focus_jump returns true only for valid node indices
prop_focus_jump_returns_true_only_for_valid_indices
```

---

## Section 5 — Fuzz Targets

### parse_hex Fuzz Target

```rust
// File: fuzz/fuzz_targets/parse_hex.rs
// Input type: arbitrary::Arbitrary<'a> for String
// Corpus seeds:
//   - "#FF0000" (6-char red)
//   - "#FF000080" (8-char red with alpha)
//   - "#00FF00" (6-char green)
//   - "00FF00" (no prefix)
//   - "#0000FF00" (8-char blue with alpha)
// Risk class: LOW — pure parsing function, Result-wrapped, no side effects
// Oportunities: malformed hex, empty strings, whitespace, case variation
```

### from_toml Fuzz Target

```rust
// File: fuzz/fuzz_targets/from_toml.rs
// Input type: arbitrary::Arbitrary<'a> for String
// Corpus seeds:
//   - Valid TOKENS_TOML excerpt
//   - Empty string
//   - Missing sections
//   - Malformed TOML syntax
// Risk class: MEDIUM — parses into nested struct, many field accesses
// Oportunities: missing keys, wrong types, deeply nested tables, array inputs
```

---

## Section 6 — Kani Harnesses

### Animation Overflow Check

```rust
// File: kani/kani_test_packet_dot_animation.rs
// Property: PacketDotManager::animate with delta_ms=f64::MAX cannot overflow
// Bound: delta_ms up to f64::MAX
// Rationale: AnimateTick::normalized_delta divides by 1000; f64::MAX/1000 is finite
```

### Index Bounds Check

```rust
// File: kani/kani_test_graph_canvas_bounds.rs
// Property: GraphCanvas::visible_nodes, render_node_card, set_node_overlay
//          never panic on out-of-bounds indices
// Bound: step_idx in 0..node_count*2 (oversized)
// Rationale: All index access via .get() which returns None, not panic
```

### Zoom Clamp Proof

```rust
// File: kani/kani_test_zoom_bounds.rs
// Property: set_zoom(zoom) always results in self.zoom ∈ [0.1, 5.0]
// Bound: zoom ∈ {0.0, 0.05, 0.1, 1.0, 5.0, 10.0, f64::MAX, f64::MIN}
// Rationale: .clamp() is monotonic; proven bounded by Kani
```

---

## Section 7 — Mutation Testing Checkpoints

### Error Enum Coverage (12 variants → 12 tests)

| Error Variant | Mutation Kill Test |
|--------------|-------------------|
| `InvalidToken(String)` | `from_toml` with empty string → assert matches `.InvalidToken` |
| `NavItemNotFound(String)` | N/A — future, reserve test slot |
| `InvalidScreenTransition(String)` | N/A — future, reserve test slot |
| `TokenParseError(String)` | `parse_hex` with `"#GG0000"` → assert matches `.TokenParseError` |
| `InvalidFlowDocument(String)` | N/A — future, reserve test slot |
| `LayoutNotComputed` | N/A — future, reserve test slot |
| `NodeNotFound(usize)` | `node_layout_position(usize::MAX)` → assert matches `.NodeNotFound` |
| `InvalidViewport` | N/A — future, reserve test slot |
| `AnimationOverflow` | N/A — future, reserve test slot |
| `ViewHidden` | N/A — future, reserve test slot |
| `MissingDesignToken(String)` | N/A — future, reserve test slot |

### Key Mutation Targets

| Function | Mutation | Kill Test |
|----------|---------|-----------|
| `parse_hex` | swap `b'A'` case | test `"a"` lowercase still works |
| `parse_hex` | change `/255.0` to `/256.0` | assert values < 1.0 |
| `nybble` | remove default case | assert invalid chars error |
| `set_zoom` | remove clamp | assert zoom stays in bounds |
| `PacketDotManager::add_dot` | change `>=` to `>` | test eviction at exactly MAX |
| `position_along_bezier` | swap `mt` and `t` | test endpoints still correct |

**Mutation kill rate target**: ≥90%

---

## Section 8 — Combinatorial Coverage Matrix

### tokens::parse_hex

| Scenario | Input | Expected Output |
|----------|-------|-----------------|
| happy: 6-char | `"#FF0000"` | `Ok([1.0, 0.0, 0.0, 1.0])` |
| happy: 8-char | `"#FF000080"` | `Ok([1.0, 0.0, 0.0, 0.502])` |
| happy: no prefix | `"FF0000"` | `Ok([1.0, 0.0, 0.0, 1.0])` |
| happy: lowercase | `"#ff0000"` | `Ok([1.0, 0.0, 0.0, 1.0])` |
| error: invalid char | `"#GG0000"` | `Err(TokenParseError("invalid hex char"))` |
| error: wrong len 3 | `"#F00"` | `Err(TokenParseError("invalid hex length: 3"))` |
| error: wrong len 4 | `"#F000"` | `Err(TokenParseError("invalid hex length: 4"))` |
| error: empty | `"#"` | `Err(TokenParseError("hex too short"))` |
| boundary: whitespace | `"  #FF0000  "` | `Ok([1.0, 0.0, 0.0, 1.0])` |

### AppShell Navigation

| Scenario | Input | Expected |
|----------|-------|----------|
| Overview | `AppShell::new()` | `active_nav = Overview` |
| set WorkflowGraph | `set_active_nav(WorkflowGraph)` | `active_nav = WorkflowGraph` |
| nav_item_rect idx 0 | `nav_item_rect(0)` | `Rect { y: 0.0 }` |
| nav_item_rect idx 5 | `nav_item_rect(5)` | `Rect { y: 280.0 }` |

### GraphCanvas Zoom

| Scenario | Input | Expected |
|----------|-------|----------|
| below min | `set_zoom(0.05)` | `zoom = 0.1` |
| at min | `set_zoom(0.1)` | `zoom = 0.1` |
| in range | `set_zoom(2.0)` | `zoom = 2.0` |
| at max | `set_zoom(5.0)` | `zoom = 5.0` |
| above max | `set_zoom(10.0)` | `zoom = 5.0` |

### OverlayState

| Variant | glow_color | glow_radius |
|---------|------------|------------|
| Pending | `pending()` | 2.0 |
| Running | `running()` | 4.0 |
| Succeeded | `success()` | 3.0 |
| Failed | `failure()` | 6.0 |
| Skipped | `text_tertiary()` | 2.0 |
| Waiting | `active_cyan()` | 3.0 |
| Asking | `warning()` | 3.0 |
| Cancelled | `text_tertiary()` | 2.0 |

### EdgeType

| Variant | color | is_dashed |
|---------|-------|-----------|
| Normal | `[0.0, 0.6, 0.8, 1.0]` | false |
| Branch | `[0.694, 0.302, 1.0, 1.0]` | true |
| ErrorRoute | `[0.6, 0.1, 0.1, 1.0]` | true |
| RetryRoute | `[1.0, 0.9, 0.0, 1.0]` | true |
| Join | `[0.176, 0.42, 1.0, 1.0]` | false |
| LoopBack | `[0.0, 0.898, 0.78, 1.0]` | false |

---

## Minimum Test Count: 425

| Module | Unit | Integration | Proptest | Fuzz | Total |
|--------|------|-------------|----------|------|-------|
| tokens | 30 | 15 | 8 | 2 | 55 |
| shell | 20 | 12 | 2 | 0 | 34 |
| packet_dot | 20 | 15 | 3 | 0 | 38 |
| graph_canvas | 40 | 30 | 8 | 0 | 78 |
| graph_node | 15 | 5 | 0 | 0 | 20 |
| graph_edge | 10 | 5 | 0 | 0 | 15 |
| Error variants | 12 | 0 | 0 | 0 | 12 |
| **Subtotal** | **147** | **82** | **21** | **2** | **252** |
| Reserve for BDD edge cases | | | | | +173 |

**252 concrete behaviors + 173 combinatorial/boundary variants = 425 minimum**
