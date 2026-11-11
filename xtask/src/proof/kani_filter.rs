/// Derive the `--harness` filter from a `required: kani:` file reference.
/// The registry names the proof file (`kani_step_budget.rs`) or a
/// `file.rs::harness` pair; Kani matches filters against fully qualified
/// harness names, which never contain `.rs`.
pub(super) fn kani_harness_filter(reference: &str) -> &str {
    let bare = match reference.rsplit("::").next() {
        Some(part) => part,
        None => reference,
    };
    match bare.strip_suffix(".rs") {
        Some(stem) => stem,
        None => bare,
    }
}
