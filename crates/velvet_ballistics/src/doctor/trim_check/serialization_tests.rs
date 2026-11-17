//! Pure projection tests; no journal is opened and no trim eligibility is proved.
use super::build_trim_runs_json;
use vb_core::RunId;
use vb_storage::{EventSeq, TrimBlocker, TrimDiagnostic, TrimEligibility};

#[test]
fn empty_trim_run_projection_is_exactly_empty() {
    let diagnostic = TrimDiagnostic {
        runs: Vec::new(),
        total_runs: 0,
        eligible_runs: 0,
        blocked_runs: 0,
        total_events_trimmable: 0,
    };
    assert_eq!(build_trim_runs_json(&diagnostic), Vec::<serde_json::Value>::new());
}

#[test]
fn trim_run_projection_preserves_exact_rows_fields_and_order() {
    let diagnostic = TrimDiagnostic {
        runs: vec![
            TrimEligibility::Blocked {
                run: RunId::new(73),
                blocker: TrimBlocker::RetentionPolicy { retain_last_n_terminal: 10 },
            },
            TrimEligibility::Eligible {
                run: RunId::new(71),
                safe_point: EventSeq::new(13),
                events_trimmable: 7,
            },
            TrimEligibility::Blocked {
                run: RunId::new(72),
                blocker: TrimBlocker::NoDurableSnapshot,
            },
            TrimEligibility::Eligible {
                run: RunId::new(74),
                safe_point: EventSeq::ZERO,
                events_trimmable: 0,
            },
        ],
        total_runs: 4,
        eligible_runs: 2,
        blocked_runs: 2,
        total_events_trimmable: 7,
    };
    let expected = vec![
        serde_json::json!({"run": 73, "status": "blocked", "blocker": "retention_policy"}),
        serde_json::json!({"run": 71, "status": "eligible", "safe_point": 13, "events_trimmable": 7}),
        serde_json::json!({"run": 72, "status": "blocked", "blocker": "no_durable_snapshot"}),
        serde_json::json!({"run": 74, "status": "eligible", "safe_point": 0, "events_trimmable": 0}),
    ];
    assert_eq!(build_trim_runs_json(&diagnostic), expected);
    // Repeated projection must not consume, reorder or mutate the input rows.
    assert_eq!(build_trim_runs_json(&diagnostic), expected);
}
