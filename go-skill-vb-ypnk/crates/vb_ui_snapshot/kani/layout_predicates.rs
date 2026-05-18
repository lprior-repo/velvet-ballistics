use crate::layout_kernel::{
    chip_is_readable, is_clipped, is_out_of_bounds, overlap_area_px, rect_contains,
    selected_state_is_visible, Rect, SelectedIndicator,
};

fn bounded_rect() -> Rect {
    let x = u32::from(kani::any::<u16>());
    let y = u32::from(kani::any::<u16>());
    let width = u32::from(kani::any::<u16>());
    let height = u32::from(kani::any::<u16>());
    kani::assume(x <= 4_096);
    kani::assume(y <= 4_096);
    kani::assume(width <= 4_096);
    kani::assume(height <= 4_096);
    Rect::kani_assumed_valid(x, y, width, height)
}

fn fixed_rect(x: u32, y: u32, width: u32, height: u32) -> Rect {
    Rect::kani_assumed_valid(x, y, width, height)
}

#[kani::proof]
fn layout_overlap_predicate_is_symmetric_and_checked() {
    let first = bounded_rect();
    let second = bounded_rect();
    let forward = overlap_area_px(first, second);
    let reverse = overlap_area_px(second, first);
    assert!(forward == reverse);
    match forward {
        Ok(area) => assert!(area <= 16_777_216),
        Err(_) => assert!(false),
    }
}

#[kani::proof]
fn layout_clipping_rejects_rectangles_outside_container() {
    let container = fixed_rect(0, 0, 100, 100);
    let child = bounded_rect();
    let contained = rect_contains(container, child);
    let clipped = is_clipped(container, child);
    assert!(contained.is_ok());
    assert!(clipped.is_ok());
    if contained == Ok(false) {
        assert!(clipped == Ok(true));
    }
}

#[kani::proof]
fn layout_bounds_rejects_controls_outside_viewport() {
    let viewport = fixed_rect(0, 0, 1_920, 1_080);
    let control = bounded_rect();
    let contained = rect_contains(viewport, control);
    let out_of_bounds = is_out_of_bounds(viewport, control);
    assert!(contained.is_ok());
    assert!(out_of_bounds.is_ok());
    if contained == Ok(false) {
        assert!(out_of_bounds == Ok(true));
    }
}

#[kani::proof]
fn layout_chip_readability_requires_area_dimensions_and_contrast() {
    let chip = bounded_rect();
    let contrast = u32::from(kani::any::<u16>());
    kani::assume(contrast <= 10_000);
    let readable = chip_is_readable(chip, contrast);
    if readable {
        assert!(chip.width() >= 24);
        assert!(chip.height() >= 12);
        assert!(contrast >= 4_500);
    }
}

#[kani::proof]
fn layout_selected_state_requires_visible_positive_indicator() {
    let viewport = fixed_rect(0, 0, 1_920, 1_080);
    let indicator = bounded_rect();
    let selected = selected_state_is_visible(viewport, SelectedIndicator::Visible(indicator));
    assert!(selected.is_ok());
    if selected == Ok(true) {
        assert!(indicator.width() > 0);
        assert!(indicator.height() > 0);
        assert!(rect_contains(viewport, indicator) == Ok(true));
    }
}
