//! Action ticket panel -- shows durable action metadata for replay safety decisions.

use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::value::Taint;

// ---------------------------------------------------------------------------
// Side-effect certainty
// ---------------------------------------------------------------------------

/// How certain the system is about whether an action has side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            format!("Run: {}", self.run.as_u64()),
            format!("Step: {}", self.step.get()),
            format!("Action: {}", self.action.get()),
            format!("Seq: {}", self.seq.as_u64()),
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
        assert_eq!(ticket.run.as_u64(), 42);
        assert_eq!(ticket.step.get(), 3);
        assert_eq!(ticket.action.get(), 7);
        assert_eq!(ticket.seq.as_u64(), 100);
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
}
