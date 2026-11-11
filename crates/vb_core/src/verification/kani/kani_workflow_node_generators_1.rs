fn arbitrary_node_variant_17() -> CompiledNodeKind {
    CompiledNodeKind::CollectNext {
        collector_slot: SlotIdx::new(kani::any()),
        body: StepIdx::new(kani::any()),
        done: StepIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_18() -> CompiledNodeKind {
    CompiledNodeKind::CollectFinish {
        collector_slot: SlotIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_19() -> CompiledNodeKind {
    CompiledNodeKind::ReduceStart {
        input: SlotIdx::new(kani::any()),
        accumulator: SlotIdx::new(kani::any()),
        initial: ConstIdx::new(kani::any()),
        body: StepIdx::new(kani::any()),
        done: StepIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_20() -> CompiledNodeKind {
    CompiledNodeKind::ReduceNext {
        iterator_slot: SlotIdx::new(kani::any()),
        accumulator: SlotIdx::new(kani::any()),
        body: StepIdx::new(kani::any()),
        done: StepIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_21() -> CompiledNodeKind {
    CompiledNodeKind::ReduceFinish {
        accumulator: SlotIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_22() -> CompiledNodeKind {
    CompiledNodeKind::RepeatStart {
        max_attempts: kani::any::<u16>().into(),
        body: StepIdx::new(kani::any()),
        done: StepIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_23() -> CompiledNodeKind {
    CompiledNodeKind::RepeatAttempt {
        attempt_slot: SlotIdx::new(kani::any()),
        body: StepIdx::new(kani::any()),
        done: StepIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_24() -> CompiledNodeKind {
    CompiledNodeKind::RepeatCheck {
        attempt_slot: SlotIdx::new(kani::any()),
        done: StepIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_25() -> CompiledNodeKind {
    CompiledNodeKind::RepeatFinish {
        result: SlotIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_26() -> CompiledNodeKind {
    CompiledNodeKind::WaitUntil {
        deadline_slot: SlotIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_27() -> CompiledNodeKind {
    CompiledNodeKind::WaitEvent {
        event: SlotIdx::new(kani::any()),
        timeout_slot: arbitrary_optional_slot().into(),
    }
}

fn arbitrary_node_variant_28() -> CompiledNodeKind {
    CompiledNodeKind::Ask {
        prompt: SlotIdx::new(kani::any()),
        timeout_slot: arbitrary_optional_slot().into(),
    }
}

fn arbitrary_node_variant_29() -> CompiledNodeKind {
    CompiledNodeKind::AskResume {
        answer: SlotIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_30() -> CompiledNodeKind {
    CompiledNodeKind::RetryCheck {
        policy_slot: SlotIdx::new(kani::any()),
        body: StepIdx::new(kani::any()),
        exhausted: StepIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_31() -> CompiledNodeKind {
    CompiledNodeKind::ErrorHandler {
        body: StepIdx::new(kani::any()),
        handler: StepIdx::new(kani::any()),
        error_slot: arbitrary_optional_slot().into(),
    }
}

fn arbitrary_node_variant_32() -> CompiledNodeKind {
    CompiledNodeKind::Jump {
        target: StepIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_33() -> CompiledNodeKind {
    CompiledNodeKind::Finish {
        result: SlotIdx::new(kani::any()),
    }
}
