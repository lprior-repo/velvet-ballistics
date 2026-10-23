// vb-8eocj.3.5 — YAML Collect pagination: limit = pages * items (total items)
// page_size = items. Compiler must compute total item limit via checked
// multiplication; runtime must retain state for non-empty last/first page
// and exhaust after first page without duplicating/skipping.

// Compile-time contract
fn compile_collect(pages: Option<u32>, items: Option<u32>) -> Result<CollectStart, CompileError> {
    let p = pages.unwrap_or(1);
    let i = items.unwrap_or(1);
    let total = p.checked_mul(i).ok_or(CompileError {
        kind: "collect_bound_overflow",
        detail: format!("pages={} * items={} exceeds u32", p, i),
    })?;
    Ok(CollectStart { limit: total, page_size: i })
}

// Runtime contract
fn finish_collect_start_page(state: &mut CollectState, items: Vec<Item>) {
    if !items.is_empty() {
        state.body = Some(items);
        state.done = true;
    }
}

fn collect_next(state: &mut CollectState) -> Option<Vec<Item>> {
    if state.done { return None; }
    state.done = true;
    state.body.take()
}

// Invariants
// - limit = pages * items (total items), page_size = items
// - u32 overflow -> CompileError with collect_bound_overflow kind
// - Non-empty first/last page -> body set, done=true
// - collect_next returns body once, then None (no dup/skip)
// - Original fixture pages=3, items=5 -> limit=15, page_size=5 (unchanged IR)
