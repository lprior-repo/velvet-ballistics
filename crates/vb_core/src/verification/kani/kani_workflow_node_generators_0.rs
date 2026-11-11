fn arbitrary_node_variant_0() -> CompiledNodeKind {
    CompiledNodeKind::Nop
}

fn arbitrary_node_variant_1() -> CompiledNodeKind {
    CompiledNodeKind::SetConst {
        value: ConstIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_2() -> CompiledNodeKind {
    CompiledNodeKind::Copy {
        source: SlotIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_3() -> CompiledNodeKind {
    CompiledNodeKind::EvalExpr {
        expr: ExprIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_4() -> CompiledNodeKind {
    CompiledNodeKind::BuildObject {
        fields: Box::new([]),
    }
}

fn arbitrary_node_variant_5() -> CompiledNodeKind {
    CompiledNodeKind::BuildList {
        items: Box::new([]),
    }
}

fn arbitrary_node_variant_6() -> CompiledNodeKind {
    CompiledNodeKind::Do {
        action: ActionId::new(kani::any()),
        input: SlotIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_7() -> CompiledNodeKind {
    CompiledNodeKind::Choose {
        branches: bounded_expr_branches(),
        otherwise: arbitrary_optional_step().into(),
    }
}

fn arbitrary_node_variant_8() -> CompiledNodeKind {
    CompiledNodeKind::ChooseSlot {
        branches: bounded_slot_branches(),
        otherwise: arbitrary_optional_step().into(),
    }
}

fn arbitrary_node_variant_9() -> CompiledNodeKind {
    CompiledNodeKind::ForEachStart {
        input: SlotIdx::new(kani::any()),
        item_slot: SlotIdx::new(kani::any()),
        limit: kani::any::<u32>().into(),
        body: StepIdx::new(kani::any()),
        done: StepIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_10() -> CompiledNodeKind {
    CompiledNodeKind::ForEachNext {
        iterator_slot: SlotIdx::new(kani::any()),
        body: StepIdx::new(kani::any()),
        done: StepIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_11() -> CompiledNodeKind {
    CompiledNodeKind::ForEachJoin {
        output: SlotIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_12() -> CompiledNodeKind {
    CompiledNodeKind::TogetherStart {
        branches: bounded_step_indices(),
        join: StepIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_13() -> CompiledNodeKind {
    CompiledNodeKind::TogetherBranch {
        branch: kani::any::<u16>().into(),
        entry: StepIdx::new(kani::any()),
        join: StepIdx::new(kani::any()),
        accumulator: SlotIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_14() -> CompiledNodeKind {
    CompiledNodeKind::TogetherJoin {
        branch_count: kani::any::<u16>().into(),
        accumulator: SlotIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_15() -> CompiledNodeKind {
    CompiledNodeKind::CollectStart {
        source: SlotIdx::new(kani::any()),
        limit: kani::any::<u32>().into(),
        page_size: kani::any(),
        body: StepIdx::new(kani::any()),
        done: StepIdx::new(kani::any()),
    }
}

fn arbitrary_node_variant_16() -> CompiledNodeKind {
    CompiledNodeKind::CollectPage {
        collector_slot: SlotIdx::new(kani::any()),
        body: StepIdx::new(kani::any()),
        done: StepIdx::new(kani::any()),
    }
}
