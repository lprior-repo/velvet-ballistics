use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};

pub(super) fn with_nodes(
    nodes: &[CompiledNode],
    entry: StepIdx,
    contract: &ResourceContract,
    visit: impl FnOnce(&[CompiledNode], StepIdx, &ResourceContract),
) {
    for node in nodes {
        assert!(
            kind_is_focused(&node.kind),
            "budget generator produced an unsupported node kind"
        );
    }
    visit(nodes, entry, contract);
}

pub(super) fn kind_is_focused(kind: &CompiledNodeKind) -> bool {
    matches!(
        kind,
        CompiledNodeKind::Nop
            | CompiledNodeKind::Do { .. }
            | CompiledNodeKind::WaitUntil { .. }
            | CompiledNodeKind::WaitEvent { .. }
            | CompiledNodeKind::Ask { .. }
            | CompiledNodeKind::Finish { .. }
    )
}
