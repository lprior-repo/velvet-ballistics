impl kani::Arbitrary for Idempotency {
    fn any() -> Self {
        match kani::any::<u8>() % 3 {
            0 => Idempotency::DeterministicPure,
            1 => Idempotency::IdempotentExternal,
            _ => Idempotency::AtLeastOnceExternal,
        }
    }
}

impl kani::Arbitrary for SideEffect {
    fn any() -> Self {
        match kani::any::<u8>() % 5 {
            0 => SideEffect::None,
            1 => SideEffect::Writes,
            2 => SideEffect::Sends,
            3 => SideEffect::Creates,
            _ => SideEffect::Destroys,
        }
    }
}

impl kani::Arbitrary for RetrySafety {
    fn any() -> Self {
        match kani::any::<u8>() % 3 {
            0 => RetrySafety::Safe,
            1 => RetrySafety::KeyRequired,
            _ => RetrySafety::Unsafe,
        }
    }
}

impl kani::Arbitrary for Capability {
    fn any() -> Self {
        Capability::new(
            // Bounded name to avoid unbounded symbolic strings
            format!("cap_{}", kani::any::<u8>().wrapping_rem(16)).into_boxed_str(),
            kani::any(),
        )
    }
}

impl kani::Arbitrary for ActionContract {
    fn any() -> Self {
        // Bounded capability count (0..4) to keep symbolic state space manageable
        let cap_count = kani::any::<u8>() % 4;
        let mut caps = Vec::with_capacity(usize::from(cap_count));
        let mut i = 0u8;
        while i < cap_count {
            caps.push(kani::any::<Capability>());
            i = match i.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
        ActionContract {
            id: kani::any(),
            name: ActionName::from_static_infallible("test-action"),
            input_slot_count: kani::any(),
            output_slot_count: kani::any(),
            max_input_bytes: kani::any(),
            max_output_bytes: kani::any(),
            timeout_ms: kani::any(),
            idempotency: kani::any(),
            side_effect: kani::any(),
            retry_safety: kani::any(),
            required_capabilities: caps.into_boxed_slice(),
        }
    }
}
