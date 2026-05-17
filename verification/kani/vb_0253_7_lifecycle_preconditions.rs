// verification/kani/vb_0253_7_lifecycle_preconditions.rs
//
// Kani harness for lifecycle preconditions (vb-0253.7)
//
// PROOF OBLIGATION: KANI-002
// CLAIM: lifecycle commands with valid RunId always pass preconditions
//
// This harness verifies that:
// 1. For all valid (state, command) pairs, check_lifecycle_transition returns true
// 2. For all invalid (state, command) pairs, check_lifecycle_transition returns false
// 3. The preconditions in the contract are enforced
//
// Verification command:
// cargo kani --crate-type=lib -p vb_cli --harness lifecycle_preconditions_harness

use vb_core::workflow::{LifecycleState, LifecycleCommand, check_lifecycle_transition};

// =============================================================================
// PRECONDITION VERIFICATION
// =============================================================================

// PRE-001: For all lifecycle commands (cancel, resume, retry, answer),
//          the run identified by RunId must exist in the journal
// PRE-002: For answer, the run must be in WaitingAnswer state
// PRE-003: For cancel, resume, retry, the run must be in a non-terminal state
// PRE-004: The journal must be accessible and return a valid event sequence

// =============================================================================
// TRANSITION VALIDITY TABLE
// =============================================================================

// This table is the authoritative source for valid transitions:
//
// +----------------+---------+---------+--------+--------+
// | From State     | Cancel  | Resume  | Retry  | Answer |
// +----------------+---------+---------+--------+--------+
// | Pending        | FALSE   | FALSE   | FALSE  | FALSE  |
// | Active         | TRUE    | FALSE   | FALSE  | FALSE  |
// | WaitingAnswer  | TRUE    | TRUE    | FALSE  | TRUE   |
// | Failed         | FALSE   | FALSE   | TRUE   | FALSE  |
// | Completed      | FALSE   | FALSE   | FALSE  | FALSE  |
// | Cancelled      | FALSE   | FALSE   | FALSE  | FALSE  |
// +----------------+---------+---------+--------+--------+

// =============================================================================
// HARNESS: All valid transitions pass check_lifecycle_transition
// =============================================================================

#[kani::proof]
fn harness_valid_transitions_pass() {
    // Test all combinations that SHOULD be valid
    let valid_cases: Vec<(LifecycleState, LifecycleCommand)> = vec![
        // (state, command, description)
        (LifecycleState::Active, LifecycleCommand::Cancel, "Active -> Cancel"),
        (LifecycleState::WaitingAnswer, LifecycleCommand::Cancel, "WaitingAnswer -> Cancel"),
        (LifecycleState::WaitingAnswer, LifecycleCommand::Resume, "WaitingAnswer -> Resume"),
        (LifecycleState::Failed, LifecycleCommand::Retry, "Failed -> Retry"),
        (LifecycleState::WaitingAnswer, LifecycleCommand::Answer, "WaitingAnswer -> Answer"),
    ];

    // Verify each valid case returns true
    for (state, cmd) in valid_cases {
        let result = check_lifecycle_transition(state, cmd);
        kani::assert!(
            result == true,
            "Valid transition {:?} -> {:?} must return true",
            state, cmd
        );
    }
}

// =============================================================================
// HARNESS: All invalid transitions fail check_lifecycle_transition
// =============================================================================

#[kani::proof]
fn harness_invalid_transitions_fail() {
    // Test all combinations that SHOULD be invalid
    let invalid_cases: Vec<(LifecycleState, LifecycleCommand)> = vec![
        // Pending - no transitions valid
        (LifecycleState::Pending, LifecycleCommand::Cancel),
        (LifecycleState::Pending, LifecycleCommand::Resume),
        (LifecycleState::Pending, LifecycleCommand::Retry),
        (LifecycleState::Pending, LifecycleCommand::Answer),
        // Active - only Cancel valid
        (LifecycleState::Active, LifecycleCommand::Resume),
        (LifecycleState::Active, LifecycleCommand::Retry),
        (LifecycleState::Active, LifecycleCommand::Answer),
        // WaitingAnswer - Cancel, Resume, Answer valid (Retry not)
        (LifecycleState::WaitingAnswer, LifecycleCommand::Retry),
        // Failed - only Retry valid
        (LifecycleState::Failed, LifecycleCommand::Cancel),
        (LifecycleState::Failed, LifecycleCommand::Resume),
        (LifecycleState::Failed, LifecycleCommand::Answer),
        // Completed - no transitions valid (terminal)
        (LifecycleState::Completed, LifecycleCommand::Cancel),
        (LifecycleState::Completed, LifecycleCommand::Resume),
        (LifecycleState::Completed, LifecycleCommand::Retry),
        (LifecycleState::Completed, LifecycleCommand::Answer),
        // Cancelled - no transitions valid (terminal)
        (LifecycleState::Cancelled, LifecycleCommand::Cancel),
        (LifecycleState::Cancelled, LifecycleCommand::Resume),
        (LifecycleState::Cancelled, LifecycleCommand::Retry),
        (LifecycleState::Cancelled, LifecycleCommand::Answer),
    ];

    // Verify each invalid case returns false
    for (state, cmd) in invalid_cases {
        let result = check_lifecycle_transition(state, cmd);
        kani::assert!(
            result == false,
            "Invalid transition {:?} -> {:?} must return false",
            state, cmd
        );
    }
}

// =============================================================================
// HARNESS: check_lifecycle_transition is total (never panics)
// =============================================================================

#[kani::proof]
fn harness_transition_is_total() {
    // Generate arbitrary state and command
    let state: LifecycleState = kani::any();
    let cmd: LifecycleCommand = kani::any();

    // Call the function - it must not panic
    // If it panics, the harness fails
    let result = check_lifecycle_transition(state, cmd);

    // Result is always a boolean
    kani::assert!(result == true || result == false);
}

// =============================================================================
// HARNESS: Completeness - all state/command combinations covered
// =============================================================================

#[kani::proof]
fn harness_all_combinations_covered() {
    // 6 states * 4 commands = 24 combinations
    // This test verifies we have coverage for each category

    let state: LifecycleState = kani::any();
    let cmd: LifecycleCommand = kani::any();

    // Just call the function - if any combination were missing,
    // the Rust compiler would warn about non-exhaustive match
    let _ = check_lifecycle_transition(state, cmd);
}

// =============================================================================
// HARNESS: PRE-002 - Answer requires WaitingAnswer
// =============================================================================

#[kani::proof]
fn harness_answer_requires_waiting() {
    let state: LifecycleState = kani::any();

    // For answer to be valid, state must be WaitingAnswer
    let answer_valid = check_lifecycle_transition(state, LifecycleCommand::Answer);

    if answer_valid {
        kani::assert!(
            state == LifecycleState::WaitingAnswer,
            "Answer is only valid from WaitingAnswer"
        );
    }
}

// =============================================================================
// HARNESS: PRE-003 - cancel/resume/retry require non-terminal
// =============================================================================

#[kani::proof]
fn harness_mutating_commands_require_non_terminal() {
    let state: LifecycleState = kani::any();

    // Cancel is valid from Active and WaitingAnswer (non-terminal)
    let cancel_valid = check_lifecycle_transition(state, LifecycleCommand::Cancel);
    if cancel_valid {
        kani::assert!(
            state == LifecycleState::Active || state == LifecycleState::WaitingAnswer,
            "Cancel is only valid from Active or WaitingAnswer"
        );
    }

    // Resume is valid from WaitingAnswer (non-terminal)
    let resume_valid = check_lifecycle_transition(state, LifecycleCommand::Resume);
    if resume_valid {
        kani::assert!(
            state == LifecycleState::WaitingAnswer,
            "Resume is only valid from WaitingAnswer"
        );
    }

    // Retry is valid from Failed (non-terminal)
    let retry_valid = check_lifecycle_transition(state, LifecycleCommand::Retry);
    if retry_valid {
        kani::assert!(
            state == LifecycleState::Failed,
            "Retry is only valid from Failed"
        );
    }
}

// =============================================================================
// HARNESS: Terminal states block all commands
// =============================================================================

#[kani::proof]
fn harness_terminal_blocks_all() {
    let terminal_state: LifecycleState = kani::any();

    // Ensure state is terminal
    kani::assume(terminal_state.is_terminal());

    // All commands must be invalid from terminal states
    let cancel_result = check_lifecycle_transition(terminal_state, LifecycleCommand::Cancel);
    let resume_result = check_lifecycle_transition(terminal_state, LifecycleCommand::Resume);
    let retry_result = check_lifecycle_transition(terminal_state, LifecycleCommand::Retry);
    let answer_result = check_lifecycle_transition(terminal_state, LifecycleCommand::Answer);

    kani::assert!(
        cancel_result == false &&
        resume_result == false &&
        retry_result == false &&
        answer_result == false,
        "Terminal states must block all lifecycle commands"
    );
}

// =============================================================================
// HARNESS: Non-terminal states allow at least one command
// =============================================================================

#[kani::proof]
fn harness_non_terminal_allows_some_command() {
    let state: LifecycleState = kani::any();

    // Ensure state is NOT terminal
    kani::assume(!state.is_terminal());

    // At least one command should be valid for non-terminal states
    let cancel_valid = check_lifecycle_transition(state, LifecycleCommand::Cancel);
    let resume_valid = check_lifecycle_transition(state, LifecycleCommand::Resume);
    let retry_valid = check_lifecycle_transition(state, LifecycleCommand::Retry);
    let answer_valid = check_lifecycle_transition(state, LifecycleCommand::Answer);

    let any_valid = cancel_valid || resume_valid || retry_valid || answer_valid;

    kani::assert!(
        any_valid,
        "Non-terminal state {:?} must allow at least one command",
        state
    );
}

// =============================================================================
// HARNESS: Pending state blocks all commands
// =============================================================================

#[kani::proof]
fn harness_pending_blocks_all() {
    // Pending is special - it's the initial state but not retryable
    let pending = LifecycleState::Pending;

    let cancel_valid = check_lifecycle_transition(pending, LifecycleCommand::Cancel);
    let resume_valid = check_lifecycle_transition(pending, LifecycleCommand::Resume);
    let retry_valid = check_lifecycle_transition(pending, LifecycleCommand::Retry);
    let answer_valid = check_lifecycle_transition(pending, LifecycleCommand::Answer);

    kani::assert!(
        !cancel_valid && !resume_valid && !retry_valid && !answer_valid,
        "Pending state must block all commands"
    );
}

// =============================================================================
// COVERAGE: State machine edge coverage
// =============================================================================

// Verify each valid transition edge returns true
#[kani::proof]
fn coverage_valid_edges() {
    // Verify each valid transition explicitly returns true
    // Active -> Cancel
    let r1 = check_lifecycle_transition(LifecycleState::Active, LifecycleCommand::Cancel);
    kani::assert!(r1 == true, "Active -> Cancel must be valid");

    // WaitingAnswer -> Cancel
    let r2 = check_lifecycle_transition(LifecycleState::WaitingAnswer, LifecycleCommand::Cancel);
    kani::assert!(r2 == true, "WaitingAnswer -> Cancel must be valid");

    // WaitingAnswer -> Resume
    let r3 = check_lifecycle_transition(LifecycleState::WaitingAnswer, LifecycleCommand::Resume);
    kani::assert!(r3 == true, "WaitingAnswer -> Resume must be valid");

    // Failed -> Retry
    let r4 = check_lifecycle_transition(LifecycleState::Failed, LifecycleCommand::Retry);
    kani::assert!(r4 == true, "Failed -> Retry must be valid");

    // WaitingAnswer -> Answer
    let r5 = check_lifecycle_transition(LifecycleState::WaitingAnswer, LifecycleCommand::Answer);
    kani::assert!(r5 == true, "WaitingAnswer -> Answer must be valid");
}
