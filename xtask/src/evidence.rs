#![allow(unreachable_pub)]
#![allow(dead_code)]
// UI release contract moved to velvet-optional (deferred)
include!("evidence/release_contract.rs");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutKernelError {
    CoordinateOverflow,
    MissingSelectedIndicator,
}

type LayoutKernelResult<T> = std::result::Result<T, LayoutKernelError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedIndicator {
    Visible(Rect),
    Hidden(Rect),
    Missing,
}

impl Rect {
    fn new(x: u32, y: u32, width: u32, height: u32) -> LayoutKernelResult<Self> {
        let rect = Self {
            x,
            y,
            width,
            height,
        };
        rect_right(rect)?;
        rect_bottom(rect)?;
        Ok(rect)
    }
}

fn overlap_area_px(first: Rect, second: Rect) -> LayoutKernelResult<u32> {
    let first_right = rect_right(first)?;
    let first_bottom = rect_bottom(first)?;
    let second_right = rect_right(second)?;
    let second_bottom = rect_bottom(second)?;
    let left = first.x.max(second.x);
    let top = first.y.max(second.y);
    let right = first_right.min(second_right);
    let bottom = first_bottom.min(second_bottom);
    if right <= left || bottom <= top {
        return Ok(0);
    }
    let width = checked_sub(left, right)?;
    let height = checked_sub(top, bottom)?;
    checked_mul(width, height)
}

fn rect_right(rect: Rect) -> LayoutKernelResult<u32> {
    checked_add(rect.x, rect.width)
}

fn rect_bottom(rect: Rect) -> LayoutKernelResult<u32> {
    checked_add(rect.y, rect.height)
}

fn rect_has_positive_area(rect: Rect) -> bool {
    rect.width > 0 && rect.height > 0
}

fn rect_contains(container: Rect, child: Rect) -> LayoutKernelResult<bool> {
    let container_right = rect_right(container)?;
    let container_bottom = rect_bottom(container)?;
    let child_right = rect_right(child)?;
    let child_bottom = rect_bottom(child)?;
    Ok(child.x >= container.x
        && child.y >= container.y
        && child_right <= container_right
        && child_bottom <= container_bottom)
}

fn is_clipped(container: Rect, label: Rect) -> LayoutKernelResult<bool> {
    rect_contains(container, label).map(|contained| !contained)
}

fn is_out_of_bounds(viewport: Rect, control: Rect) -> LayoutKernelResult<bool> {
    rect_contains(viewport, control).map(|contained| !contained)
}

fn chip_is_readable(chip: Rect, contrast_milli: u32) -> bool {
    const CHIP_MIN_WIDTH: u32 = 24;
    const CHIP_MIN_HEIGHT: u32 = 12;
    const CHIP_MIN_CONTRAST_MILLI: u32 = 4_500;
    rect_has_positive_area(chip)
        && chip.width >= CHIP_MIN_WIDTH
        && chip.height >= CHIP_MIN_HEIGHT
        && contrast_milli >= CHIP_MIN_CONTRAST_MILLI
}

fn selected_state_is_visible(
    viewport: Rect,
    indicator: SelectedIndicator,
) -> LayoutKernelResult<bool> {
    let rect = match indicator {
        SelectedIndicator::Visible(rect) => rect,
        SelectedIndicator::Hidden(_) => return Ok(false),
        SelectedIndicator::Missing => return Err(LayoutKernelError::MissingSelectedIndicator),
    };
    rect_contains(viewport, rect).map(|contained| contained && rect_has_positive_area(rect))
}

fn checked_add(left: u32, right: u32) -> LayoutKernelResult<u32> {
    left.checked_add(right)
        .ok_or(LayoutKernelError::CoordinateOverflow)
}

fn checked_sub(left: u32, right: u32) -> LayoutKernelResult<u32> {
    right
        .checked_sub(left)
        .ok_or(LayoutKernelError::CoordinateOverflow)
}

fn checked_mul(left: u32, right: u32) -> LayoutKernelResult<u32> {
    left.checked_mul(right)
        .ok_or(LayoutKernelError::CoordinateOverflow)
}

include!("evidence/release_validation.rs");
include!("evidence/tooling_and_gate_types.rs");
include!("evidence/bundle.rs");
include!("evidence/error_profile_domain.rs");
include!("evidence/parsed_documents.rs");
include!("evidence/raw_documents.rs");
include!("evidence/fixture_parsers.rs");
include!("evidence/profile_runner.rs");
include!("evidence/release_model.rs");
include!("evidence/artifact_facts.rs");
include!("evidence/release_validators.rs");
include!("evidence/release_rendering.rs");
include!("evidence/negative_fixtures.rs");
include!("evidence/persistence.rs");
include!("evidence/tests.rs");
