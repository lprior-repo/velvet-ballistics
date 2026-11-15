pub(super) fn text_report(expected: &serde_json::Value) -> String {
    let side_effects = expected["side_effects"]
        .as_array()
        .map_or_else(Vec::new, |rows| {
            rows.iter().map(ToString::to_string).collect()
        });
    let tail = expected["journal_tail"]
        .as_array()
        .map_or_else(Vec::new, |rows| {
            rows.iter().map(ToString::to_string).collect()
        });
    [
        format!("incident report for run {}", expected["run_id"].as_str().map_or("", std::convert::identity)),
        format!("  failure_code:  {}", expected["failure_code"].as_str().map_or("", std::convert::identity)),
        format!("  failed_at_step: {}", expected["failed_at_step"]),
        String::from("  side_effects:"),
        side_effects.into_iter().map(|row| format!("    {row}")).collect::<Vec<_>>().join("\n"),
        String::from("  repair_hints:"),
        String::from("    - run was cancelled; check if cancellation was intentional\n    - review recorded action outcomes for partial cleanup needs"),
        String::from("  journal_tail:"),
        tail.into_iter().map(|row| format!("    {row}")).collect::<Vec<_>>().join("\n"),
    ].join("\n") + "\n"
}
