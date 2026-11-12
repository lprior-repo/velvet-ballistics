impl Shard {
    pub fn ticket_with_retry_capacity(
        &self,
        ticket: ActionTicket,
        retry_policy: VbCoreRetryPolicy,
    ) -> RuntimeResult<ActionTicket> {
        let Some(state) = self.run_state_get(ticket.run) else {
            return Err(RuntimeError::RunNotFound);
        };
        if retry_policy != VbCoreRetryPolicy::Retryable
            || !crate::shard::helpers::retry_metadata_exists(state, ticket.step)
        {
            return Ok(ticket);
        }
        let policy = crate::shard::helpers::retry_policy_after_action(state, ticket.step)?;
        Ok(ActionTicket {
            capacity: ticket.capacity.max(policy.max_attempts),
            ..ticket
        })
    }
}
