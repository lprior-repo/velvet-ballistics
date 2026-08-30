#![forbid(unsafe_code)]
//! Helper utilities for building normalized semantic observations.

use super::types::SlotObservation;
use blake3::Hasher;

/// Compute a BLAKE3 observation digest over the ordered observation list.
///
/// The digest is computed by hashing the concatenation of all observation
/// type discriminants and their field values in canonical order. This provides
/// a stable fingerprint for comparing two observation lists without needing
/// to compare every element individually.
pub fn observation_digest(
    observations: &[super::types::JournalObservation],
) -> Result<[u8; 32], super::types::ObservationSignatureError> {
    let mut hasher = Hasher::new();
    for observation in observations {
        digest_observation(observation, &mut hasher);
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(hasher.finalize().as_bytes());
    Ok(digest)
}

/// Compute a BLAKE3 digest of a byte slice.
pub fn serialized_digest(data: &[u8]) -> Result<[u8; 32], super::types::ObservationSignatureError> {
    let mut hasher = Hasher::new();
    hasher.update(data);
    let mut digest = [0u8; 32];
    digest.copy_from_slice(hasher.finalize().as_bytes());
    Ok(digest)
}

/// Compute a BLAKE3 digest of a string (UTF-8 bytes).
pub fn str_digest(s: &str) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(s.as_bytes());
    let mut digest = [0u8; 32];
    digest.copy_from_slice(hasher.finalize().as_bytes());
    digest
}

/// Create a `SlotObservation` from slot event fields.
pub fn slot_observation(
    slot: vb_core::SlotIdx,
    attempt: u16,
    value: Option<&[u8]>,
    extra: Option<&[u8]>,
) -> SlotObservation {
    let value_digest = value.map(|bytes| {
        let mut hasher = Hasher::new();
        hasher.update(bytes);
        let mut digest = [0u8; 32];
        digest.copy_from_slice(hasher.finalize().as_bytes());
        digest
    });
    let extra_digest = extra.map(|bytes| {
        let mut hasher = Hasher::new();
        hasher.update(bytes);
        let mut digest = [0u8; 32];
        digest.copy_from_slice(hasher.finalize().as_bytes());
        digest
    });
    SlotObservation {
        slot,
        attempt,
        value_digest,
        extra_digest,
    }
}

/// Return a canonical u32 length from a usize count.
///
/// Returns `ObservationSignatureError::AllocationFailed` if the count
/// exceeds `u32::MAX`. This is acceptable for capability sets and similar
/// bounded collections.
pub fn stable_len(len: usize) -> Result<u32, super::types::ObservationSignatureError> {
    u32::try_from(len).map_err(|_| super::types::ObservationSignatureError::AllocationFailed)
}

/// Digest a single observation into a hasher, contributing to the overall
/// observation digest.
fn digest_observation(observation: &super::types::JournalObservation, hasher: &mut Hasher) {
    match observation {
        super::types::JournalObservation::Lifecycle(l) => {
            hasher.update(b"Lifecycle");
            match l {
                super::types::LifecycleObservation::Accepted => {
                    hasher.update(b"Accepted");
                }
                super::types::LifecycleObservation::Admitted {
                    policy,
                    capabilities_digest,
                    capability_count,
                } => {
                    hasher.update(b"Admitted");
                    hasher.update(format!("{policy:?}").as_bytes());
                    hasher.update(&*capabilities_digest);
                    hasher.update(&capability_count.to_be_bytes());
                }
                super::types::LifecycleObservation::Resumed => {
                    hasher.update(b"Resumed");
                }
                super::types::LifecycleObservation::Retried => {
                    hasher.update(b"Retried");
                }
            }
        }
        super::types::JournalObservation::Step(s) => {
            hasher.update(b"Step");
            match s {
                super::types::StepObservation::Started { step, attempt } => {
                    hasher.update(b"Started");
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                }
                super::types::StepObservation::Succeeded { step, output } => {
                    hasher.update(b"Succeeded");
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&output.get().to_be_bytes());
                }
                super::types::StepObservation::Failed { step, attempt } => {
                    hasher.update(b"Failed");
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                }
            }
        }
        super::types::JournalObservation::Terminal(t) => {
            hasher.update(b"Terminal");
            match t {
                super::types::TerminalObservation::Cancelled {
                    attempt,
                    reason_digest,
                } => {
                    hasher.update(b"Cancelled");
                    hasher.update(&attempt.to_be_bytes());
                    if let Some(d) = reason_digest {
                        hasher.update(b"some");
                        hasher.update(&*d);
                    } else {
                        hasher.update(b"none");
                    }
                }
                super::types::TerminalObservation::Killed { attempt } => {
                    hasher.update(b"Killed");
                    hasher.update(&attempt.to_be_bytes());
                }
                super::types::TerminalObservation::Finished { result, attempt } => {
                    hasher.update(b"Finished");
                    hasher.update(&result.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                }
                super::types::TerminalObservation::Failed { attempt } => {
                    hasher.update(b"Failed");
                    hasher.update(&attempt.to_be_bytes());
                }
            }
        }
        super::types::JournalObservation::Slot(slot_obs) => {
            hasher.update(b"Slot");
            hasher.update(&slot_obs.slot.get().to_be_bytes());
            hasher.update(&slot_obs.attempt.to_be_bytes());
            if let Some(d) = slot_obs.value_digest {
                hasher.update(b"some");
                hasher.update(&d);
            } else {
                hasher.update(b"none");
            }
            if let Some(d) = slot_obs.extra_digest {
                hasher.update(b"extra_some");
                hasher.update(&d);
            } else {
                hasher.update(b"extra_none");
            }
        }
        super::types::JournalObservation::Action(a) => {
            hasher.update(b"Action");
            match a {
                super::types::ActionObservation::Scheduled {
                    action,
                    step,
                    attempt,
                    action_abi_digest,
                } => {
                    hasher.update(b"Scheduled");
                    hasher.update(&action.get().to_be_bytes());
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                    if let Some(d) = action_abi_digest {
                        hasher.update(b"some");
                        hasher.update(&d.as_bytes());
                    } else {
                        hasher.update(b"none");
                    }
                }
                super::types::ActionObservation::Completed {
                    action,
                    step,
                    attempt,
                } => {
                    hasher.update(b"Completed");
                    hasher.update(&action.get().to_be_bytes());
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                }
                super::types::ActionObservation::Failed {
                    action,
                    step,
                    attempt,
                } => {
                    hasher.update(b"Failed");
                    hasher.update(&action.get().to_be_bytes());
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                }
                super::types::ActionObservation::Abandoned {
                    action,
                    step,
                    attempt,
                    capacity,
                } => {
                    hasher.update(b"Abandoned");
                    hasher.update(&action.get().to_be_bytes());
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                    hasher.update(&capacity.to_be_bytes());
                }
            }
        }
        super::types::JournalObservation::Wait(w) => {
            hasher.update(b"Wait");
            match w {
                super::types::WaitObservation::Scheduled { step, attempt } => {
                    hasher.update(b"Scheduled");
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                }
                super::types::WaitObservation::Resolved { step, attempt } => {
                    hasher.update(b"Resolved");
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                }
            }
        }
        super::types::JournalObservation::Ask(a) => {
            hasher.update(b"Ask");
            match a {
                super::types::AskObservation::Scheduled { step, attempt } => {
                    hasher.update(b"Scheduled");
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                }
                super::types::AskObservation::Answered { step, attempt } => {
                    hasher.update(b"Answered");
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                }
                super::types::AskObservation::AnswerRecorded { slot, answer } => {
                    hasher.update(b"AnswerRecorded");
                    hasher.update(&slot.get().to_be_bytes());
                    hasher.update(format!("{answer:?}").as_bytes());
                }
                super::types::AskObservation::TimedOut { step, attempt } => {
                    hasher.update(b"TimedOut");
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                }
            }
        }
        super::types::JournalObservation::Timer(t) => {
            hasher.update(b"Timer");
            match t {
                super::types::TimerObservation::RetryScheduled { step, attempt } => {
                    hasher.update(b"RetryScheduled");
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                }
                super::types::TimerObservation::AskTimedOut { step, attempt } => {
                    hasher.update(b"AskTimedOut");
                    hasher.update(&step.get().to_be_bytes());
                    hasher.update(&attempt.to_be_bytes());
                }
            }
        }
        super::types::JournalObservation::Digest(d) => {
            hasher.update(b"Digest");
            match d.subject {
                super::types::DigestSubject::Workflow => {
                    hasher.update(b"Workflow");
                }
                super::types::DigestSubject::Artifact => {
                    hasher.update(b"Artifact");
                }
            }
            hasher.update(&d.digest);
        }
    }
}
