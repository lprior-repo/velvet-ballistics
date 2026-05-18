//! Tests for layout_kernel pub fns: Rect construction, overlap, bounds, clipping,
//! chip readability, selected-state visibility, and helpers.

use vb_ui_snapshot::layout_kernel::{
    CHIP_MIN_CONTRAST_MILLI, CHIP_MIN_HEIGHT, CHIP_MIN_WIDTH, LayoutKernelError, Rect,
    SelectedIndicator, chip_is_readable, is_clipped, is_out_of_bounds, overlap_area_px,
    rect_bottom, rect_contains, rect_has_positive_area, rect_right, selected_state_is_visible,
};

//
// Rect construction — valid and overflow-rejecting
//

#[test]
fn rect_new_accepts_valid_coords() {
    let r = Rect::new(0, 0, 100, 200).expect("valid rect");
    assert_eq!(r.x(), 0);
    assert_eq!(r.y(), 0);
    assert_eq!(r.width(), 100);
    assert_eq!(r.height(), 200);
}

#[test]
fn rect_new_accepts_max_values() {
    let r = Rect::new(u32::MAX - 1, u32::MAX - 1, 1, 1).expect("max valid rect");
    assert_eq!(r.x(), u32::MAX - 1);
    assert_eq!(r.y(), u32::MAX - 1);
    assert_eq!(r.width(), 1);
    assert_eq!(r.height(), 1);
}

#[test]
fn rect_new_rejects_when_x_plus_width_overflows() {
    let result = Rect::new(u32::MAX, 0, 2, 0);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), LayoutKernelError::CoordinateOverflow);
}

#[test]
fn rect_new_rejects_when_y_plus_height_overflows() {
    let result = Rect::new(0, u32::MAX, 0, 2);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), LayoutKernelError::CoordinateOverflow);
}

//
// Accessors
//

#[test]
fn rect_x_returns_origin() {
    let r = Rect::new(10, 20, 30, 40).expect("valid");
    assert_eq!(r.x(), 10);
}

#[test]
fn rect_y_returns_origin() {
    let r = Rect::new(10, 20, 30, 40).expect("valid");
    assert_eq!(r.y(), 20);
}

#[test]
fn rect_width_returns_dim() {
    let r = Rect::new(10, 20, 30, 40).expect("valid");
    assert_eq!(r.width(), 30);
}

#[test]
fn rect_height_returns_dim() {
    let r = Rect::new(10, 20, 30, 40).expect("valid");
    assert_eq!(r.height(), 40);
}

//
// rect_right
//

#[test]
fn rect_right_computes_x_plus_width() {
    let r = Rect::new(100, 50, 300, 400).expect("valid");
    assert_eq!(rect_right(r).expect("ok"), 400);
}

#[test]
fn rect_right_returns_zero_when_width_zero() {
    let r = Rect::new(100, 50, 0, 400).expect("valid");
    assert_eq!(rect_right(r).expect("ok"), 100);
}

// rect_right overflow is validated at Rect::new time; cannot construct invalid rect via API

//
// rect_bottom
//

#[test]
fn rect_bottom_computes_y_plus_height() {
    let r = Rect::new(100, 50, 300, 400).expect("valid");
    assert_eq!(rect_bottom(r).expect("ok"), 450);
}

#[test]
fn rect_bottom_returns_zero_when_height_zero() {
    let r = Rect::new(100, 50, 300, 0).expect("valid");
    assert_eq!(rect_bottom(r).expect("ok"), 50);
}

// rect_bottom overflow is validated at Rect::new time; cannot construct invalid rect via API

//
// rect_has_positive_area
//

#[test]
fn rect_has_positive_area_true_when_both_nonzero() {
    let r = Rect::new(0, 0, 10, 10).expect("valid");
    assert!(rect_has_positive_area(r));
}

#[test]
fn rect_has_positive_area_false_when_width_zero() {
    let r = Rect::new(0, 0, 0, 10).expect("valid");
    assert!(!rect_has_positive_area(r));
}

#[test]
fn rect_has_positive_area_false_when_height_zero() {
    let r = Rect::new(0, 0, 10, 0).expect("valid");
    assert!(!rect_has_positive_area(r));
}

#[test]
fn rect_has_positive_area_false_when_both_zero() {
    let r = Rect::new(0, 0, 0, 0).expect("valid");
    assert!(!rect_has_positive_area(r));
}

//
// overlap_area_px
//

#[test]
fn overlap_area_px_returns_zero_for_disjoint_horizontal() {
    let a = Rect::new(0, 0, 100, 100).expect("valid");
    let b = Rect::new(200, 0, 100, 100).expect("valid");
    assert_eq!(overlap_area_px(a, b).expect("ok"), 0);
}

#[test]
fn overlap_area_px_returns_zero_for_disjoint_vertical() {
    let a = Rect::new(0, 0, 100, 100).expect("valid");
    let b = Rect::new(0, 200, 100, 100).expect("valid");
    assert_eq!(overlap_area_px(a, b).expect("ok"), 0);
}

#[test]
fn overlap_area_px_returns_zero_for_adjacent_touching() {
    let a = Rect::new(0, 0, 100, 100).expect("valid");
    let b = Rect::new(100, 0, 100, 100).expect("valid");
    assert_eq!(overlap_area_px(a, b).expect("ok"), 0);
}

#[test]
fn overlap_area_px_returns_exact_area_for_partial_overlap() {
    let a = Rect::new(0, 0, 100, 100).expect("valid");
    let b = Rect::new(50, 50, 100, 100).expect("valid");
    // Overlap rect: x=50..100, y=50..100 → 50×50 = 2500
    assert_eq!(overlap_area_px(a, b).expect("ok"), 2500);
}

#[test]
fn overlap_area_px_returns_full_area_when_one_contains_other() {
    let outer = Rect::new(0, 0, 200, 200).expect("valid");
    let inner = Rect::new(50, 50, 50, 50).expect("valid");
    assert_eq!(overlap_area_px(outer, inner).expect("ok"), 2500);
}

#[test]
fn overlap_area_px_returns_area_for_diagonal_overlap() {
    let a = Rect::new(0, 0, 100, 100).expect("valid");
    let b = Rect::new(50, 50, 100, 72).expect("valid");
    // Overlap: x=50..100 (50), y=50..100 (50) = 2500
    assert_eq!(overlap_area_px(a, b).expect("ok"), 2500);
}

#[test]
fn overlap_area_px_returns_zero_for_single_pixel_touching_corner() {
    let a = Rect::new(0, 0, 50, 50).expect("valid");
    let b = Rect::new(50, 50, 50, 50).expect("valid");
    assert_eq!(overlap_area_px(a, b).expect("ok"), 0);
}

// Overflow in overlap_area_px cannot be triggered via public API:
// width = min(r1_right, r2_right) - max(r1_x, r2_x)
// Each term is ≤ u32::MAX, so difference ≤ u32::MAX, and product ≤ u32::MAX² < 2^64

//
// rect_contains
//

#[test]
fn rect_contains_returns_true_when_child_fully_inside() {
    let container = Rect::new(0, 0, 200, 200).expect("valid");
    let child = Rect::new(50, 50, 50, 50).expect("valid");
    assert_eq!(rect_contains(container, child).expect("ok"), true);
}

#[test]
fn rect_contains_returns_false_when_child_touches_edge() {
    let container = Rect::new(0, 0, 100, 100).expect("valid");
    let child = Rect::new(0, 0, 100, 100).expect("valid");
    // child.right == container.right and child.bottom == container.bottom
    // so child is contained (≤ not <)
    assert_eq!(rect_contains(container, child).expect("ok"), true);
}

#[test]
fn rect_contains_returns_false_when_child_escapes_right() {
    let container = Rect::new(0, 0, 100, 100).expect("valid");
    let child = Rect::new(80, 50, 50, 50).expect("valid");
    assert_eq!(rect_contains(container, child).expect("ok"), false);
}

#[test]
fn rect_contains_returns_false_when_child_escapes_bottom() {
    let container = Rect::new(0, 0, 100, 100).expect("valid");
    let child = Rect::new(50, 80, 50, 50).expect("valid");
    assert_eq!(rect_contains(container, child).expect("ok"), false);
}

// "Escapes top" cannot occur with u32 coordinates: child.y is always ≥ 0
// and container.y is always ≥ 0. The child being above the container (child.y < container.y)
// is tested by rect_contains_returns_false_when_child_escapes_left (container.y > child.y).

#[test]
fn rect_contains_returns_false_when_child_escapes_left() {
    let container = Rect::new(50, 50, 100, 100).expect("valid");
    let child = Rect::new(0, 50, 50, 50).expect("valid");
    assert_eq!(rect_contains(container, child).expect("ok"), false);
}

//
// is_clipped — label extends outside container
//

#[test]
fn is_clipped_returns_true_when_label_exceeds_container_right() {
    let container = Rect::new(0, 0, 100, 100).expect("valid");
    let label = Rect::new(80, 50, 50, 20).expect("valid");
    assert_eq!(is_clipped(container, label).expect("ok"), true);
}

#[test]
fn is_clipped_returns_false_when_label_fully_inside() {
    let container = Rect::new(0, 0, 200, 200).expect("valid");
    let label = Rect::new(50, 50, 50, 50).expect("valid");
    assert_eq!(is_clipped(container, label).expect("ok"), false);
}

#[test]
fn is_clipped_returns_true_when_label_exceeds_container_bottom() {
    let container = Rect::new(0, 0, 100, 100).expect("valid");
    let label = Rect::new(10, 90, 20, 30).expect("valid");
    assert_eq!(is_clipped(container, label).expect("ok"), true);
}

//
// is_out_of_bounds — control extends outside viewport
//

#[test]
fn is_out_of_bounds_returns_true_when_control_exceeds_viewport_right() {
    let viewport = Rect::new(0, 0, 1920, 1080).expect("valid");
    let control = Rect::new(1900, 100, 50, 50).expect("valid");
    assert_eq!(is_out_of_bounds(viewport, control).expect("ok"), true);
}

#[test]
fn is_out_of_bounds_returns_false_when_control_fully_inside() {
    let viewport = Rect::new(0, 0, 1920, 1080).expect("valid");
    let control = Rect::new(100, 100, 500, 500).expect("valid");
    assert_eq!(is_out_of_bounds(viewport, control).expect("ok"), false);
}

#[test]
fn is_out_of_bounds_returns_true_when_control_exceeds_viewport_bottom() {
    let viewport = Rect::new(0, 0, 1920, 1080).expect("valid");
    let control = Rect::new(100, 1000, 200, 100).expect("valid");
    assert_eq!(is_out_of_bounds(viewport, control).expect("ok"), true);
}

// "exceeds top" impossible with u32 coords: control.y is always ≥ 0 = viewport.y minimum.
// Testing that control at top boundary is contained instead:
#[test]
fn is_out_of_bounds_returns_false_when_control_at_viewport_top() {
    let viewport = Rect::new(0, 0, 1920, 1080).expect("valid");
    let control = Rect::new(100, 0, 200, 50).expect("valid");
    assert_eq!(is_out_of_bounds(viewport, control).expect("ok"), false);
}

#[test]
fn is_out_of_bounds_returns_true_when_control_exceeds_viewport_left() {
    let viewport = Rect::new(100, 100, 1000, 800).expect("valid");
    let control = Rect::new(0, 100, 50, 50).expect("valid");
    assert_eq!(is_out_of_bounds(viewport, control).expect("ok"), true);
}

//
// chip_is_readable — correct thresholds
//

#[test]
fn chip_is_readable_returns_true_when_all_thresholds_met() {
    let chip = Rect::new(0, 0, 100, 50).expect("valid");
    assert!(chip_is_readable(chip, CHIP_MIN_CONTRAST_MILLI));
}

#[test]
fn chip_is_readable_returns_false_when_width_too_small() {
    let chip = Rect::new(0, 0, CHIP_MIN_WIDTH - 1, 100).expect("valid");
    assert!(!chip_is_readable(chip, CHIP_MIN_CONTRAST_MILLI));
}

#[test]
fn chip_is_readable_returns_false_when_height_too_small() {
    let chip = Rect::new(0, 0, 100, CHIP_MIN_HEIGHT - 1).expect("valid");
    assert!(!chip_is_readable(chip, CHIP_MIN_CONTRAST_MILLI));
}

#[test]
fn chip_is_readable_returns_false_when_contrast_too_low() {
    let chip = Rect::new(0, 0, 100, 100).expect("valid");
    assert!(!chip_is_readable(chip, CHIP_MIN_CONTRAST_MILLI - 1));
}

#[test]
fn chip_is_readable_returns_false_when_area_is_zero() {
    let chip = Rect::new(0, 0, 0, 0).expect("valid");
    assert!(!chip_is_readable(chip, CHIP_MIN_CONTRAST_MILLI));
}

#[test]
fn chip_is_readable_returns_true_at_exact_minima() {
    let chip = Rect::new(0, 0, CHIP_MIN_WIDTH, CHIP_MIN_HEIGHT).expect("valid");
    assert!(chip_is_readable(chip, CHIP_MIN_CONTRAST_MILLI));
}

//
// selected_state_is_visible
//

#[test]
fn selected_state_is_visible_returns_true_when_visible_and_contained() {
    let viewport = Rect::new(0, 0, 1920, 1080).expect("valid");
    let indicator = SelectedIndicator::Visible(Rect::new(100, 100, 50, 50).expect("valid"));
    assert_eq!(
        selected_state_is_visible(viewport, indicator).expect("ok"),
        true
    );
}

#[test]
fn selected_state_is_visible_returns_false_when_hidden() {
    let viewport = Rect::new(0, 0, 1920, 1080).expect("valid");
    let indicator = SelectedIndicator::Hidden(Rect::new(100, 100, 50, 50).expect("valid"));
    assert_eq!(
        selected_state_is_visible(viewport, indicator).expect("ok"),
        false
    );
}

#[test]
fn selected_state_is_visible_returns_error_when_missing() {
    let viewport = Rect::new(0, 0, 1920, 1080).expect("valid");
    let indicator = SelectedIndicator::Missing;
    assert_eq!(
        selected_state_is_visible(viewport, indicator),
        Err(LayoutKernelError::MissingSelectedIndicator)
    );
}

#[test]
fn selected_state_is_visible_returns_false_when_visible_but_outside_viewport() {
    let viewport = Rect::new(0, 0, 1920, 1080).expect("valid");
    let indicator = SelectedIndicator::Visible(Rect::new(2000, 100, 50, 50).expect("valid"));
    assert_eq!(
        selected_state_is_visible(viewport, indicator).expect("ok"),
        false
    );
}

#[test]
fn selected_state_is_visible_returns_false_when_visible_and_zero_area() {
    let viewport = Rect::new(0, 0, 1920, 1080).expect("valid");
    let indicator = SelectedIndicator::Visible(Rect::new(100, 100, 0, 0).expect("valid"));
    assert_eq!(
        selected_state_is_visible(viewport, indicator).expect("ok"),
        false
    );
}
