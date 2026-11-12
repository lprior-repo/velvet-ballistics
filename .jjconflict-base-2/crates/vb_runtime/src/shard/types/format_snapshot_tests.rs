// ============================================================================
// Regression test for vb-8ilqu: InspectSnapshotFormatter::format_snapshot
// must source the `run` from the snapshot in the Found branch, never from
// a separately-supplied external parameter.
// ============================================================================
#[cfg(test)]
mod format_snapshot_uses_snap_run {
    use super::*;

    /// The Found branch must cite the snapshot's own `run`, not an external
    /// value. Construct a snapshot whose `snap.run` is `B` and verify the
    /// formatted output contains `B` rather than any other value the
    /// previous external-parameter signature would have shadowed.
    #[test]
    fn found_branch_uses_snap_run_not_external() {
        let snap = InspectSnapshot {
            run: RunId::new(7777),
            correlation: 99,
            pc: StepIdx::new(3),
            executed: 5,
        };
        let response = InspectResponse::Found(snap);

        let formatted = InspectSnapshotFormatter::format_snapshot(&response);

        assert!(
            formatted.contains("7777"),
            "formatted output must cite snap.run (7777), got: {formatted}"
        );
        assert!(
            formatted.contains("InspectSnapshot"),
            "formatted output should be the Found-branch shape, got: {formatted}"
        );
    }

    /// Two snapshots with different `snap.run` values must format to
    /// different strings — proves the formatter does not collapse to a
    /// single fixed value.
    #[test]
    fn found_branch_distinguishes_distinct_snap_runs() {
        let response_a = InspectResponse::Found(InspectSnapshot {
            run: RunId::new(1),
            correlation: 0,
            pc: StepIdx::ZERO,
            executed: 0,
        });
        let response_b = InspectResponse::Found(InspectSnapshot {
            run: RunId::new(2),
            correlation: 0,
            pc: StepIdx::ZERO,
            executed: 0,
        });

        let formatted_a = InspectSnapshotFormatter::format_snapshot(&response_a);
        let formatted_b = InspectSnapshotFormatter::format_snapshot(&response_b);

        assert_ne!(
            formatted_a, formatted_b,
            "two snapshots with distinct snap.run must format differently"
        );
        assert!(formatted_a.contains("RunId(1)"));
        assert!(formatted_b.contains("RunId(2)"));
    }
}
