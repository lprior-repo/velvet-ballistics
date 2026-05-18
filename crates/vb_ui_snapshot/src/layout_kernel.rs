#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LayoutKernelError {
    CoordinateOverflow,
    MissingSelectedIndicator,
}

pub type LayoutKernelResult<T> = Result<T, LayoutKernelError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SelectedIndicator {
    Visible(Rect),
    Hidden(Rect),
    Missing,
}

impl Rect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> LayoutKernelResult<Self> {
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

    pub fn x(self) -> u32 {
        self.x
    }

    pub fn y(self) -> u32 {
        self.y
    }

    pub fn width(self) -> u32 {
        self.width
    }

    pub fn height(self) -> u32 {
        self.height
    }

    #[cfg(kani)]
    pub fn kani_assumed_valid(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

pub const CHIP_MIN_WIDTH: u32 = 24;
pub const CHIP_MIN_HEIGHT: u32 = 12;
pub const CHIP_MIN_CONTRAST_MILLI: u32 = 4_500;

pub fn overlap_area_px(first: Rect, second: Rect) -> LayoutKernelResult<u32> {
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

pub fn rect_right(rect: Rect) -> LayoutKernelResult<u32> {
    checked_add(rect.x, rect.width)
}

pub fn rect_bottom(rect: Rect) -> LayoutKernelResult<u32> {
    checked_add(rect.y, rect.height)
}

pub fn rect_has_positive_area(rect: Rect) -> bool {
    rect.width > 0 && rect.height > 0
}

pub fn rect_contains(container: Rect, child: Rect) -> LayoutKernelResult<bool> {
    let container_right = rect_right(container)?;
    let container_bottom = rect_bottom(container)?;
    let child_right = rect_right(child)?;
    let child_bottom = rect_bottom(child)?;
    Ok(child.x >= container.x
        && child.y >= container.y
        && child_right <= container_right
        && child_bottom <= container_bottom)
}

pub fn is_clipped(container: Rect, label: Rect) -> LayoutKernelResult<bool> {
    rect_contains(container, label).map(|contained| !contained)
}

pub fn is_out_of_bounds(viewport: Rect, control: Rect) -> LayoutKernelResult<bool> {
    rect_contains(viewport, control).map(|contained| !contained)
}

pub fn chip_is_readable(chip: Rect, contrast_milli: u32) -> bool {
    rect_has_positive_area(chip)
        && chip.width >= CHIP_MIN_WIDTH
        && chip.height >= CHIP_MIN_HEIGHT
        && contrast_milli >= CHIP_MIN_CONTRAST_MILLI
}

pub fn selected_state_is_visible(
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
