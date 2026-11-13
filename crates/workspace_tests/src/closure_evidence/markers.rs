//! Rejected evidence marker detection.

const REJECTED_MARKERS: &[&str] = &[
    "summary-only",
    "summary only",
    "cached-only",
    "cached only",
    "skipped-only",
    "skipped only",
    "subagent-only",
    "subagent only",
    "delegated-only",
    "delegated only",
];

pub(crate) fn find_rejected_marker(text: &str) -> Option<&str> {
    let lower = text.to_lowercase();
    REJECTED_MARKERS
        .iter()
        .find(|&marker| lower.contains(marker))
        .copied()
}
