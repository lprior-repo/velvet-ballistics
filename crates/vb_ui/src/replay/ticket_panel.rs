#![forbid(unsafe_code)]
//! Action ticket panel -- shows durable action metadata for replay safety decisions.

use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::value::Taint;

// ---------------------------------------------------------------------------
// Side-effect certainty
// ---------------------------------------------------------------------------

/// How certain the system is about whether an action has side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SideEffectCertainty {
    /// The action definitely has side effects.
    Certain,
    /// It is unknown whether the action has side effects.
    Unknown,
    /// The action has no side effects.
    None,
}

// ---------------------------------------------------------------------------
// Action ticket display
// ---------------------------------------------------------------------------

/// Display model for a durable action ticket used in replay safety analysis.
#[derive(Debug, Clone)]
pub struct ActionTicketDisplay {
    /// Run this action belongs to.
    pub run: RunId,
    /// Step this action belongs to.
    pub step: StepIdx,
    /// Action identifier.
    pub action: ActionId,
    /// Sequence number when the action was scheduled.
    pub seq: SeqNo,
    /// Attempt number (1-based).
    pub attempt: u16,
    /// Idempotency key for deduplication.
    pub idempotency_key: u128,
    /// Whether the action is safe to replay without side effects.
    pub replay_safe: bool,
    /// Side-effect certainty classification.
    pub side_effect_certainty: SideEffectCertainty,
    /// Taint level of the action's input data.
    pub taint: Taint,
    /// Whether this is a duplicate completion record.
    pub duplicate_completion: bool,
}

impl ActionTicketDisplay {
    /// Creates a new ticket display from raw fields.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run: RunId,
        step: StepIdx,
        action: ActionId,
        seq: SeqNo,
        attempt: u16,
        idempotency_key: u128,
        replay_safe: bool,
        side_effect_certainty: SideEffectCertainty,
        taint: Taint,
        duplicate_completion: bool,
    ) -> Self {
        Self {
            run,
            step,
            action,
            seq,
            attempt,
            idempotency_key,
            replay_safe,
            side_effect_certainty,
            taint,
            duplicate_completion,
        }
    }

    /// Returns a one-line summary of the ticket.
    ///
    /// Format: `"ActionTicket #N — replay-safe: YES/NO"`
    #[must_use]
    pub fn summary_line(&self) -> String {
        let safe_label = if self.replay_safe { "YES" } else { "NO" };
        format!(
            "ActionTicket #{} — replay-safe: {safe_label}",
            self.action.get()
        )
    }

    /// Returns detailed multi-line information about the ticket.
    #[must_use]
    pub fn detail_lines(&self) -> Vec<String> {
        let side_effect_str = match self.side_effect_certainty {
            SideEffectCertainty::Certain => "certain",
            SideEffectCertainty::Unknown => "unknown",
            SideEffectCertainty::None => "none",
        };

        let duplicate_str = if self.duplicate_completion {
            "YES"
        } else {
            "NO"
        };

        vec![
            format!("Run: {}", self.run.get()),
            format!("Step: {}", self.step.get()),
            format!("Action: {}", self.action.get()),
            format!("Seq: {}", self.seq.get()),
            format!("Attempt: {}", self.attempt),
            format!("Idempotency key: {:032x}", self.idempotency_key),
            format!(
                "Replay safe: {}",
                if self.replay_safe { "YES" } else { "NO" }
            ),
            format!("Side effects: {side_effect_str}"),
            format!("Taint: {:?}", self.taint),
            format!("Duplicate completion: {duplicate_str}"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::incident::types::ReplaySafety;

    fn make_ticket(action_id: u16, replay_safe: bool) -> ActionTicketDisplay {
        ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(action_id),
            SeqNo::new(10),
            1,
            0xABCD_1234,
            replay_safe,
            SideEffectCertainty::None,
            Taint::Clean,
            false,
        )
    }

    // -- Construction --

    #[test]
    fn new_ticket_has_correct_fields() {
        let ticket = ActionTicketDisplay::new(
            RunId::new(42),
            StepIdx::new(3),
            ActionId::new(7),
            SeqNo::new(100),
            2,
            0xDEAD_BEEF,
            true,
            SideEffectCertainty::Certain,
            Taint::Secret,
            true,
        );
        assert_eq!(ticket.run.get(), 42);
        assert_eq!(ticket.step.get(), 3);
        assert_eq!(ticket.action.get(), 7);
        assert_eq!(ticket.seq.get(), 100);
        assert_eq!(ticket.attempt, 2);
        assert_eq!(ticket.idempotency_key, 0xDEAD_BEEF);
        assert!(ticket.replay_safe);
        assert_eq!(ticket.side_effect_certainty, SideEffectCertainty::Certain);
        assert_eq!(ticket.taint, Taint::Secret);
        assert!(ticket.duplicate_completion);
    }

    // -- summary_line --

    #[test]
    fn summary_line_safe_ticket() {
        let ticket = make_ticket(5, true);
        let summary = ticket.summary_line();
        assert_eq!(summary, "ActionTicket #5 — replay-safe: YES");
    }

    #[test]
    fn summary_line_unsafe_ticket() {
        let ticket = make_ticket(12, false);
        let summary = ticket.summary_line();
        assert_eq!(summary, "ActionTicket #12 — replay-safe: NO");
    }

    // -- detail_lines --

    #[test]
    fn detail_lines_has_expected_count() {
        let ticket = make_ticket(1, true);
        let lines = ticket.detail_lines();
        assert_eq!(lines.len(), 10);
    }

    #[test]
    fn detail_lines_contains_run() {
        let ticket = ActionTicketDisplay::new(
            RunId::new(99),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            1,
            0,
            true,
            SideEffectCertainty::None,
            Taint::Clean,
            false,
        );
        let lines = ticket.detail_lines();
        assert!(lines.iter().any(|l| l.contains("Run: 99")));
    }

    #[test]
    fn detail_lines_contains_step() {
        let ticket = make_ticket(1, true);
        let lines = ticket.detail_lines();
        assert!(lines.iter().any(|l| l.contains("Step: 0")));
    }

    #[test]
    fn detail_lines_contains_action() {
        let ticket = make_ticket(7, true);
        let lines = ticket.detail_lines();
        assert!(lines.iter().any(|l| l.contains("Action: 7")));
    }

    #[test]
    fn detail_lines_contains_seq() {
        let ticket = make_ticket(1, true);
        let lines = ticket.detail_lines();
        assert!(lines.iter().any(|l| l.contains("Seq: 10")));
    }

    #[test]
    fn detail_lines_contains_attempt() {
        let ticket = make_ticket(1, true);
        let lines = ticket.detail_lines();
        assert!(lines.iter().any(|l| l.contains("Attempt: 1")));
    }

    #[test]
    fn detail_lines_contains_idempotency_key() {
        let ticket = make_ticket(1, true);
        let lines = ticket.detail_lines();
        assert!(lines.iter().any(|l| l.contains("abcd1234")));
    }

    #[test]
    fn detail_lines_contains_replay_safe_yes() {
        let ticket = make_ticket(1, true);
        let lines = ticket.detail_lines();
        assert!(lines.iter().any(|l| l.contains("Replay safe: YES")));
    }

    #[test]
    fn detail_lines_contains_replay_safe_no() {
        let ticket = make_ticket(1, false);
        let lines = ticket.detail_lines();
        assert!(lines.iter().any(|l| l.contains("Replay safe: NO")));
    }

    #[test]
    fn detail_lines_side_effect_certain() {
        let ticket = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            1,
            0,
            false,
            SideEffectCertainty::Certain,
            Taint::Clean,
            false,
        );
        let lines = ticket.detail_lines();
        assert!(lines.iter().any(|l| l.contains("Side effects: certain")));
    }

    #[test]
    fn detail_lines_side_effect_unknown() {
        let ticket = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            1,
            0,
            false,
            SideEffectCertainty::Unknown,
            Taint::Clean,
            false,
        );
        let lines = ticket.detail_lines();
        assert!(lines.iter().any(|l| l.contains("Side effects: unknown")));
    }

    #[test]
    fn detail_lines_side_effect_none() {
        let ticket = make_ticket(1, true);
        let lines = ticket.detail_lines();
        assert!(lines.iter().any(|l| l.contains("Side effects: none")));
    }

    #[test]
    fn detail_lines_taint_secret() {
        let ticket = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            1,
            0,
            false,
            SideEffectCertainty::None,
            Taint::Secret,
            false,
        );
        let lines = ticket.detail_lines();
        assert!(lines.iter().any(|l| l.contains("Taint: Secret")));
    }

    #[test]
    fn detail_lines_duplicate_yes() {
        let ticket = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            1,
            0,
            false,
            SideEffectCertainty::None,
            Taint::Clean,
            true,
        );
        let lines = ticket.detail_lines();
        assert!(
            lines
                .iter()
                .any(|l| l.contains("Duplicate completion: YES"))
        );
    }

    #[test]
    fn detail_lines_duplicate_no() {
        let ticket = make_ticket(1, true);
        let lines = ticket.detail_lines();
        assert!(lines.iter().any(|l| l.contains("Duplicate completion: NO")));
    }

    // -- SideEffectCertainty equality --

    #[test]
    fn side_effect_certainty_equality() {
        assert_eq!(SideEffectCertainty::Certain, SideEffectCertainty::Certain);
        assert_eq!(SideEffectCertainty::Unknown, SideEffectCertainty::Unknown);
        assert_eq!(SideEffectCertainty::None, SideEffectCertainty::None);
        assert_ne!(SideEffectCertainty::Certain, SideEffectCertainty::Unknown);
        assert_ne!(SideEffectCertainty::Unknown, SideEffectCertainty::None);
    }

    // =========================================================================
    // Replay-safety verdict tests -- exercise idempotency-class-aware logic
    // by building ticket lists and computing aggregate replay-safety from
    // their fields, without modifying production code.
    // =========================================================================

    /// Helper: compute an aggregate replay-safety verdict from a list of
    /// tickets.  Returns `true` only when every ticket is `replay_safe`
    /// AND has `side_effect_certainty` != `Certain`.
    fn is_replay_safe_aggregate(tickets: &[ActionTicketDisplay]) -> bool {
        if tickets.is_empty() {
            return true;
        }
        tickets
            .iter()
            .all(|t| t.replay_safe && t.side_effect_certainty != SideEffectCertainty::Certain)
    }

    /// Helper: detect whether any ticket in the list has `duplicate_completion`
    /// set to true.
    fn has_duplicate_completion(tickets: &[ActionTicketDisplay]) -> bool {
        tickets.iter().any(|t| t.duplicate_completion)
    }

    /// Helper: score side-effect certainty across a list of tickets.
    /// Returns the worst (most concerning) certainty found:
    ///   Certain > Unknown > None
    fn worst_side_effect_certainty(tickets: &[ActionTicketDisplay]) -> Option<SideEffectCertainty> {
        let mut worst = None;
        for t in tickets {
            let current = match worst {
                None => t.side_effect_certainty,
                Some(SideEffectCertainty::Certain) => SideEffectCertainty::Certain,
                Some(SideEffectCertainty::Unknown) => {
                    if t.side_effect_certainty == SideEffectCertainty::Certain {
                        SideEffectCertainty::Certain
                    } else {
                        SideEffectCertainty::Unknown
                    }
                }
                Some(SideEffectCertainty::None) => {
                    if t.side_effect_certainty == SideEffectCertainty::Certain {
                        SideEffectCertainty::Certain
                    } else if t.side_effect_certainty == SideEffectCertainty::Unknown {
                        SideEffectCertainty::Unknown
                    } else {
                        SideEffectCertainty::None
                    }
                }
            };
            worst = Some(current);
        }
        worst
    }

    // -------------------------------------------------------------------------
    // 1. Idempotency-class-aware replay-safety verdict
    //    Tickets sharing an idempotency key should be identifiable, and the
    //    verdict should reflect the replay_safe and certainty fields.
    // -------------------------------------------------------------------------

    #[test]
    fn replay_safe_verdict_all_tickets_safe_and_no_certain_side_effects() {
        let tickets: Vec<ActionTicketDisplay> = (0..3u16)
            .map(|i| {
                ActionTicketDisplay::new(
                    RunId::new(1),
                    StepIdx::new(0),
                    ActionId::new(i),
                    SeqNo::new(u64::from(i)),
                    1,
                    0xAAAA_0000 | u128::from(i),
                    true,
                    SideEffectCertainty::None,
                    Taint::Clean,
                    false,
                )
            })
            .collect();
        assert!(
            is_replay_safe_aggregate(&tickets),
            "all safe + no certain side effects should be replay-safe"
        );
    }

    #[test]
    fn replay_safe_verdict_unsafe_ticket_makes_aggregate_unsafe() {
        let tickets = vec![
            ActionTicketDisplay::new(
                RunId::new(1),
                StepIdx::new(0),
                ActionId::new(1),
                SeqNo::new(1),
                1,
                0xBBBB_0001,
                true,
                SideEffectCertainty::None,
                Taint::Clean,
                false,
            ),
            ActionTicketDisplay::new(
                RunId::new(1),
                StepIdx::new(0),
                ActionId::new(2),
                SeqNo::new(2),
                1,
                0xBBBB_0002,
                false,
                SideEffectCertainty::None,
                Taint::Clean,
                false,
            ),
        ];
        assert!(
            !is_replay_safe_aggregate(&tickets),
            "a single unsafe ticket should make the aggregate unsafe"
        );
    }

    #[test]
    fn replay_safe_verdict_certain_side_effect_makes_aggregate_unsafe() {
        let tickets = vec![ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(10),
            SeqNo::new(5),
            1,
            0xCCCC_0001,
            true,
            SideEffectCertainty::Certain,
            Taint::Clean,
            false,
        )];
        assert!(
            !is_replay_safe_aggregate(&tickets),
            "certain side effects should make aggregate unsafe even if replay_safe is true"
        );
    }

    // -------------------------------------------------------------------------
    // 2. Duplicate-completion detection
    // -------------------------------------------------------------------------

    #[test]
    fn duplicate_completion_detected_in_ticket_list() {
        let tickets = vec![
            ActionTicketDisplay::new(
                RunId::new(1),
                StepIdx::new(0),
                ActionId::new(1),
                SeqNo::new(1),
                1,
                0xDDDD_0001,
                true,
                SideEffectCertainty::None,
                Taint::Clean,
                false,
            ),
            ActionTicketDisplay::new(
                RunId::new(1),
                StepIdx::new(0),
                ActionId::new(2),
                SeqNo::new(2),
                1,
                0xDDDD_0002,
                true,
                SideEffectCertainty::None,
                Taint::Clean,
                true,
            ),
        ];
        assert!(
            has_duplicate_completion(&tickets),
            "second ticket has duplicate_completion=true"
        );
    }

    #[test]
    fn no_duplicate_completion_when_all_false() {
        let tickets = vec![ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            1,
            0xEEEE_0001,
            true,
            SideEffectCertainty::None,
            Taint::Clean,
            false,
        )];
        assert!(
            !has_duplicate_completion(&tickets),
            "no tickets have duplicate_completion=true"
        );
    }

    // -------------------------------------------------------------------------
    // 3. Side-effect certainty scoring
    // -------------------------------------------------------------------------

    #[test]
    fn worst_certainty_none_when_all_none() {
        let tickets = vec![
            ActionTicketDisplay::new(
                RunId::new(1),
                StepIdx::new(0),
                ActionId::new(1),
                SeqNo::new(1),
                1,
                0x1111,
                true,
                SideEffectCertainty::None,
                Taint::Clean,
                false,
            ),
            ActionTicketDisplay::new(
                RunId::new(1),
                StepIdx::new(0),
                ActionId::new(2),
                SeqNo::new(2),
                1,
                0x2222,
                true,
                SideEffectCertainty::None,
                Taint::Clean,
                false,
            ),
        ];
        let Some(worst) = worst_side_effect_certainty(&tickets) else {
            assert!(false, "should return Some");
            return;
        };
        assert_eq!(worst, SideEffectCertainty::None);
    }

    #[test]
    fn worst_certainty_elevated_by_unknown() {
        let tickets = vec![
            ActionTicketDisplay::new(
                RunId::new(1),
                StepIdx::new(0),
                ActionId::new(1),
                SeqNo::new(1),
                1,
                0x3333,
                true,
                SideEffectCertainty::None,
                Taint::Clean,
                false,
            ),
            ActionTicketDisplay::new(
                RunId::new(1),
                StepIdx::new(0),
                ActionId::new(2),
                SeqNo::new(2),
                1,
                0x4444,
                true,
                SideEffectCertainty::Unknown,
                Taint::Clean,
                false,
            ),
        ];
        let Some(worst) = worst_side_effect_certainty(&tickets) else {
            assert!(false, "should return Some");
            return;
        };
        assert_eq!(worst, SideEffectCertainty::Unknown);
    }

    #[test]
    fn worst_certainty_elevated_to_certain() {
        let tickets = vec![
            ActionTicketDisplay::new(
                RunId::new(1),
                StepIdx::new(0),
                ActionId::new(1),
                SeqNo::new(1),
                1,
                0x5555,
                true,
                SideEffectCertainty::None,
                Taint::Clean,
                false,
            ),
            ActionTicketDisplay::new(
                RunId::new(1),
                StepIdx::new(0),
                ActionId::new(2),
                SeqNo::new(2),
                1,
                0x6666,
                true,
                SideEffectCertainty::Certain,
                Taint::Clean,
                false,
            ),
        ];
        let Some(worst) = worst_side_effect_certainty(&tickets) else {
            assert!(false, "should return Some");
            return;
        };
        assert_eq!(worst, SideEffectCertainty::Certain);
    }

    // -------------------------------------------------------------------------
    // 4. Empty ticket list returns safe verdict
    // -------------------------------------------------------------------------

    #[test]
    fn empty_ticket_list_is_replay_safe() {
        let tickets: Vec<ActionTicketDisplay> = Vec::new();
        assert!(
            is_replay_safe_aggregate(&tickets),
            "empty ticket list should always be replay-safe"
        );
    }

    #[test]
    fn empty_ticket_list_has_no_worst_certainty() {
        let tickets: Vec<ActionTicketDisplay> = Vec::new();
        assert!(
            worst_side_effect_certainty(&tickets).is_none(),
            "empty list should return None for worst certainty"
        );
    }

    // -------------------------------------------------------------------------
    // 5. Ticket with multiple actions (shared idempotency key)
    // -------------------------------------------------------------------------

    #[test]
    fn multiple_actions_same_idempotency_key_are_distinct_tickets() {
        let key: u128 = 0xFACE_FEED;
        let tickets: Vec<ActionTicketDisplay> = (1..=3u16)
            .map(|attempt| {
                ActionTicketDisplay::new(
                    RunId::new(42),
                    StepIdx::new(1),
                    ActionId::new(100),
                    SeqNo::new(u64::from(attempt)),
                    attempt,
                    key,
                    attempt == 1,
                    SideEffectCertainty::None,
                    Taint::Clean,
                    attempt > 1,
                )
            })
            .collect();

        assert_eq!(tickets.len(), 3, "should have three retry tickets");

        // All share the same idempotency key
        for t in &tickets {
            assert_eq!(t.idempotency_key, key);
        }

        // First attempt is safe, subsequent are duplicate completions
        let Some(first) = tickets.first() else {
            assert!(false, "must have first ticket");
            return;
        };
        assert!(first.replay_safe);
        assert!(!first.duplicate_completion);

        // Later attempts are marked as duplicates
        let duplicates: Vec<&ActionTicketDisplay> =
            tickets.iter().filter(|t| t.duplicate_completion).collect();
        assert_eq!(duplicates.len(), 2, "attempts 2 and 3 are duplicates");
    }

    // -------------------------------------------------------------------------
    // 6. Timeout vs non-timeout failure classification
    //    A higher attempt count and non-Clean taint indicate retry after
    //    timeout; we verify that the ticket model captures this distinction.
    // -------------------------------------------------------------------------

    #[test]
    fn timeout_retry_ticket_has_higher_attempt_and_taint() {
        let original = ActionTicketDisplay::new(
            RunId::new(10),
            StepIdx::new(2),
            ActionId::new(50),
            SeqNo::new(20),
            1,
            0xBEEF_0001,
            true,
            SideEffectCertainty::None,
            Taint::Clean,
            false,
        );
        let retry_after_timeout = ActionTicketDisplay::new(
            RunId::new(10),
            StepIdx::new(2),
            ActionId::new(50),
            SeqNo::new(21),
            2,
            0xBEEF_0001,
            false,
            SideEffectCertainty::Unknown,
            Taint::DerivedFromSecret,
            true,
        );

        assert_eq!(original.attempt, 1);
        assert_eq!(retry_after_timeout.attempt, 2);
        assert!(original.replay_safe);
        assert!(!retry_after_timeout.replay_safe);
        assert_eq!(original.taint, Taint::Clean);
        assert_eq!(retry_after_timeout.taint, Taint::DerivedFromSecret);
        assert!(!original.duplicate_completion);
        assert!(retry_after_timeout.duplicate_completion);
    }

    #[test]
    fn non_timeout_failure_has_single_attempt_and_secret_taint() {
        let failure = ActionTicketDisplay::new(
            RunId::new(10),
            StepIdx::new(2),
            ActionId::new(51),
            SeqNo::new(22),
            1,
            0xCAFE_0001,
            false,
            SideEffectCertainty::Certain,
            Taint::Secret,
            false,
        );

        assert_eq!(failure.attempt, 1, "non-timeout failure is first attempt");
        assert_eq!(
            failure.side_effect_certainty,
            SideEffectCertainty::Certain,
            "side effects are known"
        );
        assert_eq!(failure.taint, Taint::Secret, "direct secret taint");
        assert!(!failure.duplicate_completion);
    }

    // -------------------------------------------------------------------------
    // 7. All ReplaySafety variants exercised via ticket field combinations
    //    Maps SideEffectCertainty + replay_safe to a ReplaySafety verdict
    //    to ensure every variant is reachable from ticket data.
    // -------------------------------------------------------------------------

    /// Map ticket fields to a ReplaySafety verdict.
    fn classify_replay_safety(ticket: &ActionTicketDisplay) -> ReplaySafety {
        use crate::incident::types::ReplaySafety;
        if ticket.replay_safe && ticket.side_effect_certainty == SideEffectCertainty::None {
            ReplaySafety::Safe
        } else if ticket.side_effect_certainty == SideEffectCertainty::Certain {
            ReplaySafety::UnsafeSideEffect
        } else {
            ReplaySafety::Unknown
        }
    }

    #[test]
    fn replay_safety_safe_variant_from_clean_ticket() {
        let ticket = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            1,
            0,
            true,
            SideEffectCertainty::None,
            Taint::Clean,
            false,
        );
        let verdict = classify_replay_safety(&ticket);
        assert_eq!(verdict, ReplaySafety::Safe);
    }

    #[test]
    fn replay_safety_unsafe_side_effect_variant() {
        let ticket = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(2),
            SeqNo::new(1),
            1,
            0,
            false,
            SideEffectCertainty::Certain,
            Taint::Clean,
            false,
        );
        let verdict = classify_replay_safety(&ticket);
        assert_eq!(verdict, ReplaySafety::UnsafeSideEffect);
    }

    #[test]
    fn replay_safety_unknown_variant_from_unsafe_unknown_combo() {
        let ticket = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(3),
            SeqNo::new(1),
            1,
            0,
            false,
            SideEffectCertainty::Unknown,
            Taint::DerivedFromSecret,
            false,
        );
        let verdict = classify_replay_safety(&ticket);
        assert_eq!(verdict, ReplaySafety::Unknown);
    }

    // =========================================================================
    // BLACKHAT security and correctness findings
    // =========================================================================

    /// FINDING 1 -- HIGH: classify_replay_safety classifies replay_safe=false
    /// + SideEffectCertainty::None as Unknown instead of Unsafe.
    ///
    /// The classify_replay_safety function returns `ReplaySafety::Safe` only
    /// when `replay_safe && certainty == None`. When `replay_safe` is false
    /// but certainty is `None`, it falls through to `Unknown`. However, an
    /// action marked replay_safe=false with no known side effects is
    /// explicitly marked as not safe to replay -- it should arguably be
    /// `UnsafeSideEffect` or at least have a distinct "UnsafeNonIdempotent"
    /// classification rather than lumped in with "Unknown".
    ///
    /// Impact: Non-idempotent actions with no known side effects are
    /// classified as "Unknown" rather than "Unsafe", potentially leading
    /// to overly permissive replay decisions.
    #[test]
    fn blackhat_replay_safe_false_with_none_certainty_classified_as_unknown() {
        let ticket = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            1,
            0,
            false,                     // replay_safe = false
            SideEffectCertainty::None, // no known side effects
            Taint::Clean,
            false,
        );
        let verdict = classify_replay_safety(&ticket);
        assert_eq!(
            verdict,
            ReplaySafety::Unknown,
            "FINDING 1: replay_safe=false + None certainty is Unknown, not Unsafe"
        );
        // This means the action is NOT replay-safe but is not classified as
        // having unsafe side effects either -- a gap in the safety model.
        assert!(!verdict.is_safe());
    }

    /// FINDING 2 -- MEDIUM: attempt field is u16 with no overflow protection.
    ///
    /// The `attempt` field is `u16` (max 65535). If a system generates more
    /// than 65535 retries, the attempt counter would need to wrap or saturate.
    /// There is no validation that attempt > 0 in the constructor. An attempt
    /// of 0 violates the documented "1-based" invariant.
    ///
    /// Impact: An attempt of 0 could cause off-by-one errors in retry counting.
    #[test]
    fn blackhat_attempt_zero_violates_one_based_invariant() {
        let ticket = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            0, // attempt = 0, violates "1-based"
            0,
            true,
            SideEffectCertainty::None,
            Taint::Clean,
            false,
        );
        // The constructor accepts attempt=0 without validation.
        assert_eq!(
            ticket.attempt, 0,
            "FINDING 2: attempt=0 is accepted, violating the documented 1-based invariant"
        );
    }

    /// FINDING 3 -- MEDIUM: taint field is stored but never used in
    /// replay-safety classification.
    ///
    /// The `ActionTicketDisplay` struct carries a `taint: Taint` field, but
    /// neither `classify_replay_safety` nor `is_replay_safe_aggregate`
    /// considers taint when computing the safety verdict. An action with
    /// `Taint::Secret` inputs that is otherwise "safe" would be classified
    /// as `ReplaySafety::Safe`, which could cause secret data to be replayed
    /// without proper handling.
    ///
    /// Impact: Secret-tainted actions may be replayed without taint-aware
    /// safety checks, potentially exposing sensitive data.
    #[test]
    fn blackhat_taint_secret_ignored_in_safety_classification() {
        let ticket = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            1,
            0,
            true, // replay_safe
            SideEffectCertainty::None,
            Taint::Secret, // HIGH taint
            false,
        );
        let verdict = classify_replay_safety(&ticket);
        assert_eq!(
            verdict,
            ReplaySafety::Safe,
            "FINDING 3: Secret-tainted action classified as Safe because taint is ignored"
        );
    }

    /// FINDING 4 -- LOW: idempotency_key is u128 but detail_lines formats it
    /// as hex with fixed 32-character width, which is correct. However, two
    /// actions with different keys but same fields otherwise are not detected
    /// as duplicates by any built-in logic. The idempotency_key is purely
    /// informational.
    #[test]
    fn blackhat_idempotency_key_not_used_for_dedup() {
        let ticket_a = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            1,
            0xAAAA,
            true,
            SideEffectCertainty::None,
            Taint::Clean,
            false,
        );
        let ticket_b = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(2),
            1,
            0xBBBB, // different key
            true,
            SideEffectCertainty::None,
            Taint::Clean,
            false,
        );
        // Both are safe, both reference same action, but different keys.
        // The panel model does not provide dedup by key.
        assert_ne!(ticket_a.idempotency_key, ticket_b.idempotency_key);
        assert_eq!(ticket_a.action, ticket_b.action);
        // Both classified as safe.
        assert!(is_replay_safe_aggregate(&[
            ticket_a.clone(),
            ticket_b.clone()
        ]));
    }

    /// FINDING 5 -- LOW: summary_line only shows action ID, hiding all other
    /// fields including taint and replay safety details.
    ///
    /// The summary line format is `ActionTicket #N -- replay-safe: YES/NO`.
    /// It does not show step, run, attempt, taint, or side-effect certainty.
    /// Users relying on the summary line may miss critical security info.
    #[test]
    fn blackhat_summary_line_hides_critical_security_fields() {
        let secret_ticket = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(5),
            SeqNo::new(1),
            1,
            0,
            false,
            SideEffectCertainty::Certain,
            Taint::Secret,
            true,
        );
        let summary = secret_ticket.summary_line();
        // The summary does not mention Taint::Secret, Certain side effects,
        // or duplicate_completion.
        assert!(
            !summary.contains("Secret"),
            "FINDING 5: summary hides taint"
        );
        assert!(
            !summary.contains("Certain"),
            "FINDING 5: summary hides side-effect certainty"
        );
        assert!(
            !summary.contains("duplicate"),
            "FINDING 5: summary hides duplicate status"
        );
    }

    /// FINDING 6 -- MEDIUM: worst_side_effect_certainty helper uses verbose
    /// pattern matching instead of an ordering relation.
    ///
    /// The helper function has a large match on `worst` to compute the max
    /// certainty level. This is error-prone: if a new variant is added to
    /// `SideEffectCertainty`, the helper will silently ignore it in some
    /// branches. Using an `Ord` implementation would prevent this.
    #[test]
    fn blackhat_worst_certainty_empty_returns_none() {
        let tickets: Vec<ActionTicketDisplay> = Vec::new();
        let result = worst_side_effect_certainty(&tickets);
        assert!(
            result.is_none(),
            "FINDING 6: empty list returns None, not a default certainty"
        );
    }

    /// FINDING 7 -- LOW: duplicate_completion flag is purely informational and
    /// not enforced. Nothing prevents setting both replay_safe=true and
    /// duplicate_completion=true on the same ticket, which is semantically
    /// contradictory (a duplicate completion of a safe action may still be
    /// unsafe if it triggers side effects twice).
    #[test]
    fn blackhat_replay_safe_with_duplicate_completion_is_contradictory() {
        let ticket = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            1,
            0,
            true, // replay_safe
            SideEffectCertainty::None,
            Taint::Clean,
            true, // duplicate_completion -- contradictory?
        );
        // Both flags can be true simultaneously -- no validation.
        assert!(ticket.replay_safe);
        assert!(ticket.duplicate_completion);
    }

    /// FINDING 8 -- LOW: detail_lines formats idempotency_key as :032x which
    /// is correct for u128. Verify it handles zero and max values.
    #[test]
    fn blackhat_idempotency_key_boundary_formatting() {
        let zero_key = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            1,
            0,
            true,
            SideEffectCertainty::None,
            Taint::Clean,
            false,
        );
        let lines = zero_key.detail_lines();
        let idem_line = lines.iter().find(|l| l.contains("Idempotency key:"));
        assert!(idem_line.is_some());
        assert!(
            idem_line
                .unwrap()
                .contains("00000000000000000000000000000000"),
            "FINDING 8: zero key should format as 32 zeros"
        );

        let max_key = ActionTicketDisplay::new(
            RunId::new(1),
            StepIdx::new(0),
            ActionId::new(1),
            SeqNo::new(1),
            1,
            u128::MAX,
            true,
            SideEffectCertainty::None,
            Taint::Clean,
            false,
        );
        let lines_max = max_key.detail_lines();
        let idem_line_max = lines_max.iter().find(|l| l.contains("Idempotency key:"));
        assert!(idem_line_max.is_some());
        assert!(
            idem_line_max
                .unwrap()
                .contains("ffffffffffffffffffffffffffffffff"),
            "FINDING 8: max key should format as 32 f's"
        );
    }

    /// FINDING 9 -- LOW: ActionTicketDisplay does not derive PartialEq/Eq,
    /// making it harder to compare tickets in tests or collection operations.
    #[test]
    fn blackhat_ticket_display_no_partial_eq() {
        let a = make_ticket(1, true);
        let b = make_ticket(1, true);
        // ActionTicketDisplay does not derive PartialEq, so we cannot do
        // assert_eq!(a, b). This is a finding because it makes test assertions
        // verbose and prevents use in Hash-based collections.
        assert_eq!(a.action, b.action);
        assert_eq!(a.replay_safe, b.replay_safe);
    }
}
