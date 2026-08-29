#[cfg(test)]
mod flux_sequence_tests {
    use super::*;

    #[test]
    fn contiguous_sequence_valid() {
        assert!(model_is_contiguous_seq_array(&[0, 1, 2, 3, 4]));
    }

    #[test]
    fn non_contiguous_gap_detected() {
        assert!(model_has_sequence_gap(&[0, 1, 3, 4]));
    }

    #[test]
    fn duplicate_detected() {
        assert!(model_has_sequence_duplicate(&[0, 1, 1, 2]));
    }

    #[test]
    fn empty_sequence_is_contiguous() {
        assert!(model_is_contiguous_seq_array(&[]));
    }

    #[test]
    fn single_element_is_contiguous() {
        assert!(model_is_contiguous_seq_array(&[42]));
    }

    #[test]
    fn contiguity_extends_valid() {
        assert!(model_contiguity_extends(&[0, 1, 2], 3));
    }

    #[test]
    fn contiguity_extends_fails() {
        assert!(!model_contiguity_extends(&[0, 1, 2], 5));
    }

    #[test]
    fn step_order_preserved() {
        assert!(model_step_order_preserved(Some(3), 5));
        assert!(model_step_order_preserved(Some(3), 3));
        assert!(model_step_order_preserved(None, 0));
    }

    #[test]
    fn step_order_diverges() {
        assert!(model_step_order_diverges(Some(5), 3));
        assert!(!model_step_order_diverges(Some(3), 5));
        assert!(!model_step_order_diverges(None, 0));
    }

    #[test]
    fn tail_after_snapshot_valid() {
        assert!(model_tail_events_after_snapshot(10, &[11, 12, 13]));
    }

    #[test]
    fn tail_after_snapshot_rejected() {
        assert!(!model_tail_events_after_snapshot(10, &[10, 11, 12]));
        assert!(!model_tail_events_after_snapshot(10, &[5, 6, 7]));
    }

    #[test]
    fn tail_rejection_detected() {
        assert!(model_tail_events_rejection(10, &[10, 11]));
        assert!(model_tail_events_rejection(10, &[5, 6]));
        assert!(!model_tail_events_rejection(10, &[11, 12]));
    }

    #[test]
    fn stale_attempt_detection() {
        assert!(model_is_stale_attempt(Some(2), 5));
        assert!(!model_is_stale_attempt(Some(5), 5));
        assert!(!model_is_stale_attempt(Some(10), 5));
        assert!(model_is_stale_attempt(None, 5));
    }

    #[test]
    fn current_attempt_detection() {
        assert!(!model_is_current_attempt(Some(2), 5));
        assert!(model_is_current_attempt(Some(5), 5));
        assert!(model_is_current_attempt(Some(10), 5));
        assert!(model_is_current_attempt(None, 5));
    }

    #[test]
    fn max_attempt_at_least_one() {
        assert!(model_max_attempt_ge_one(&[]));
        assert!(model_max_attempt_ge_one(&[1]));
        assert!(model_max_attempt_ge_one(&[3, 5, 2]));
    }
}
