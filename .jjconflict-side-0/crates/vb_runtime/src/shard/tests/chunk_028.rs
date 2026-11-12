
#[test]
fn is_resumable_returns_true_when_state_is_resumable() {
    // Given
    let state = super::RuntimeState::Resumable;

    // When
    let result = state.is_resumable();

    // Then
    assert_eq!(result, true);
}

#[test]
fn is_resumable_returns_false_when_state_cannot_be_resumed() {
    // Given
    let non_resumable_states = [
        super::RuntimeState::Initial,
        super::RuntimeState::Running,
        super::RuntimeState::Resuming,
        super::RuntimeState::Failed,
    ];

    // When / Then
    for state in non_resumable_states {
        assert_eq!(state.is_resumable(), false, "state {state:?} must not be resumable");
    }
}
