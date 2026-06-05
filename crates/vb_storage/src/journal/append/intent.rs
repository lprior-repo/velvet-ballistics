use crate::{error::JournalError, events::JournalEvent, keys::index_action_key};

#[cfg(kani)]
use crate::keys::run_event_key;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6ActionIndexIntent {
    None,
    Put {
        action: vb_core::ActionId,
        run: vb_core::RunId,
        step: vb_core::StepIdx,
    },
    Delete {
        action: vb_core::ActionId,
        run: vb_core::RunId,
        step: vb_core::StepIdx,
    },
}

#[cfg(kani)]
pub(crate) type VerificationActionIndexIntent = Mrwe6ActionIndexIntent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6EventClass {
    Scheduled,
    Resolution,
    Unrelated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6IntentKind {
    None,
    PutPending,
    RemovePending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6AtomKind {
    EventOnly,
    EventAndPutPending,
    EventAndRemovePending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mrwe6SeamError {
    ClassIntentMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mrwe6ValidatedAtom {
    class: Mrwe6EventClass,
    intent_kind: Mrwe6IntentKind,
    atom_kind: Mrwe6AtomKind,
}

impl Mrwe6ValidatedAtom {
    #[must_use]
    pub const fn class(self) -> Mrwe6EventClass {
        self.class
    }

    #[must_use]
    pub const fn intent_kind(self) -> Mrwe6IntentKind {
        self.intent_kind
    }

    #[must_use]
    pub const fn atom_kind(self) -> Mrwe6AtomKind {
        self.atom_kind
    }
}

pub(crate) type ActionIndexIntent = Mrwe6ActionIndexIntent;

impl Mrwe6ActionIndexIntent {
    pub(crate) fn for_event(event: &JournalEvent) -> Self {
        match event {
            JournalEvent::ActionScheduled {
                action, run, step, ..
            } => Self::Put {
                action: *action,
                run: *run,
                step: *step,
            },
            JournalEvent::ActionScheduledTicket { ticket, .. } => Self::Put {
                action: ticket.action,
                run: ticket.run,
                step: ticket.step,
            },
            JournalEvent::ActionCompletedEvent {
                action, run, step, ..
            }
            | JournalEvent::ActionFailedEvent {
                action, run, step, ..
            } => Self::Delete {
                action: *action,
                run: *run,
                step: *step,
            },
            JournalEvent::ActionCompletedEnvelope { ticket, .. } => Self::Delete {
                action: ticket.action,
                run: ticket.run,
                step: ticket.step,
            },
            _ => Self::None,
        }
    }

    #[cfg(kani)]
    fn verification_view(self) -> VerificationActionIndexIntent {
        self
    }
}

#[must_use]
pub fn mrwe6_action_index_intent(event: &JournalEvent) -> Mrwe6ActionIndexIntent {
    Mrwe6ActionIndexIntent::for_event(event)
}

#[must_use]
pub fn mrwe6_event_class(event: &JournalEvent) -> Mrwe6EventClass {
    match mrwe6_action_index_intent(event) {
        Mrwe6ActionIndexIntent::Put { .. } => Mrwe6EventClass::Scheduled,
        Mrwe6ActionIndexIntent::Delete { .. } => Mrwe6EventClass::Resolution,
        Mrwe6ActionIndexIntent::None => Mrwe6EventClass::Unrelated,
    }
}

#[must_use]
pub fn mrwe6_intent_kind(intent: Mrwe6ActionIndexIntent) -> Mrwe6IntentKind {
    match intent {
        Mrwe6ActionIndexIntent::None => Mrwe6IntentKind::None,
        Mrwe6ActionIndexIntent::Put { .. } => Mrwe6IntentKind::PutPending,
        Mrwe6ActionIndexIntent::Delete { .. } => Mrwe6IntentKind::RemovePending,
    }
}

#[must_use]
pub fn mrwe6_required_intent_kind_for_class(class: Mrwe6EventClass) -> Mrwe6IntentKind {
    match class {
        Mrwe6EventClass::Scheduled => Mrwe6IntentKind::PutPending,
        Mrwe6EventClass::Resolution => Mrwe6IntentKind::RemovePending,
        Mrwe6EventClass::Unrelated => Mrwe6IntentKind::None,
    }
}

#[must_use]
pub fn mrwe6_event_intent_matches_class(event: &JournalEvent) -> bool {
    mrwe6_intent_kind_matches_event_class(
        mrwe6_event_class(event),
        mrwe6_intent_kind(mrwe6_action_index_intent(event)),
    )
}

#[must_use]
pub fn mrwe6_intent_kind_matches_event_class(
    class: Mrwe6EventClass,
    intent_kind: Mrwe6IntentKind,
) -> bool {
    mrwe6_required_intent_kind_for_class(class) == intent_kind
}

pub fn mrwe6_validated_atom(
    class: Mrwe6EventClass,
    intent_kind: Mrwe6IntentKind,
) -> Result<Mrwe6ValidatedAtom, Mrwe6SeamError> {
    if !mrwe6_intent_kind_matches_event_class(class, intent_kind) {
        return Err(Mrwe6SeamError::ClassIntentMismatch);
    }
    let atom_kind = match intent_kind {
        Mrwe6IntentKind::None => Mrwe6AtomKind::EventOnly,
        Mrwe6IntentKind::PutPending => Mrwe6AtomKind::EventAndPutPending,
        Mrwe6IntentKind::RemovePending => Mrwe6AtomKind::EventAndRemovePending,
    };
    Ok(Mrwe6ValidatedAtom {
        class,
        intent_kind,
        atom_kind,
    })
}

pub fn mrwe6_validated_atom_for_event(
    event: &JournalEvent,
) -> Result<Mrwe6ValidatedAtom, Mrwe6SeamError> {
    let intent = mrwe6_action_index_intent(event);
    mrwe6_validated_atom(mrwe6_event_class(event), mrwe6_intent_kind(intent))
}

pub fn mrwe6_action_index_key_for_intent(
    intent: Mrwe6ActionIndexIntent,
) -> Result<Option<[u8; crate::constants::INDEX_ACTION_KEY_BYTES]>, JournalError> {
    match intent {
        Mrwe6ActionIndexIntent::None => Ok(None),
        Mrwe6ActionIndexIntent::Put { action, run, step }
        | Mrwe6ActionIndexIntent::Delete { action, run, step } => {
            index_action_key(action, run, step).map(Some)
        }
    }
}

#[cfg(kani)]
pub(crate) fn verification_action_index_intent(
    event: &JournalEvent,
) -> VerificationActionIndexIntent {
    ActionIndexIntent::for_event(event).verification_view()
}

#[cfg(kani)]
pub(crate) fn verification_event_and_index_keys_exist(
    event: &JournalEvent,
) -> Result<bool, JournalError> {
    let event_key = run_event_key(event.run_id(), event.seq())?;
    let intent = ActionIndexIntent::for_event(event);
    let index_key_exists = match intent {
        ActionIndexIntent::None => false,
        ActionIndexIntent::Put { action, run, step }
        | ActionIndexIntent::Delete { action, run, step } => {
            index_action_key(action, run, step).is_ok()
        }
    };
    Ok(!event_key.is_empty() && index_key_exists)
}
