#![forbid(unsafe_code)]
//! Kani verification harness for collect pagination.

use vb_core::value::SlotValue;

use super::{copy_prefix, page_size_from, validate_page_bound};

/// PO-vb-hbav-024: Kani verification of page_size_from arithmetic bounds.
///
/// Verifies that:
/// - page_size=0 returns typed error (never panics)
/// - page_size in [1, 1024] returns either Ok(usize) or Err
/// - bounded page validation preserves typed failure behavior
#[kani::proof]
#[kani::unwind(4)]
fn collect_page_pagination_bounds() {
    let zero_result = page_size_from(0);
    kani::assert(zero_result.is_err(), "page_size=0 must return error");

    let page_size: u32 = kani::any();
    kani::assume(page_size > 0);
    kani::assume(page_size <= 1024);

    match page_size_from(page_size) {
        Ok(ps) => {
            kani::assert(ps > 0, "positive page_size must produce positive usize");
            kani::assert(ps <= 1024, "page_size result must respect harness bound");
        }
        Err(_) => {}
    }

    verify_page_bound_accepts_within_limit();
    verify_copy_prefix_empty();
    verify_copy_prefix_bounded_items();
}

fn verify_page_bound_accepts_within_limit() {
    let small_ps: usize = kani::any();
    kani::assume(small_ps > 0);
    kani::assume(small_ps <= 1024);
    let Ok(small_ps_u32) = u32::try_from(small_ps) else {
        kani::assume(false);
        return;
    };
    let limit: u32 = kani::any();
    kani::assume(limit >= small_ps_u32);

    let bound_result = validate_page_bound(small_ps, limit);
    kani::assert(
        bound_result.is_ok(),
        "page_size within limit must pass validation",
    );
}

fn verify_copy_prefix_empty() {
    let empty_items: &[SlotValue] = &[];
    let copy_result = copy_prefix(empty_items, 1);
    kani::assert(copy_result.is_ok(), "empty copy_prefix must succeed");
    if let Ok(page) = copy_result {
        kani::assert(page.is_empty(), "empty items must produce empty page");
    }
}

fn verify_copy_prefix_bounded_items() {
    let item_count: u8 = kani::any();
    kani::assume(item_count <= 2);
    let empty: [SlotValue; 0] = [];
    let one = [SlotValue::I64(kani::any())];
    let two = [SlotValue::I64(kani::any()), SlotValue::I64(kani::any())];
    let items = match item_count {
        0 => empty.as_slice(),
        1 => one.as_slice(),
        _ => two.as_slice(),
    };

    let page_size_raw: u8 = kani::any();
    kani::assume(page_size_raw > 0);
    kani::assume(page_size_raw <= 2);
    let page_size = usize::from(page_size_raw);
    let expected_bound = page_size.min(usize::from(item_count));

    match copy_prefix(items, page_size) {
        Ok(page) => kani::assert(
            page.len() <= expected_bound,
            "copied page length must respect page and item bounds",
        ),
        Err(_) => {}
    }
}
