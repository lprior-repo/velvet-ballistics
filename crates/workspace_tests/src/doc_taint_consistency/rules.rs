//! Static rule tables derived from Python source — identical wording, order, and meaning.

pub(crate) struct NodeRule {
    pub(crate) name: &'static str,
    pub(crate) phrases: &'static [&'static str],
}

pub(crate) const NODE_RULES: &[NodeRule] = &[
    NodeRule {
        name: "EvalExpr",
        phrases: &[
            "Always Clean",
            "always Clean",
            "No taint join",
            "no taint join of expression operands",
        ],
    },
    NodeRule {
        name: "BuildObject",
        phrases: &["Always Clean", "always Clean", "no join of field taints"],
    },
    NodeRule {
        name: "BuildList",
        phrases: &["Always Clean", "always Clean", "no join of item taints"],
    },
];

pub(crate) struct VocabularyConflict {
    pub(crate) phrase: &'static str,
    pub(crate) reason: &'static str,
}

pub(crate) const VOCABULARY_CONFLICTS: &[VocabularyConflict] = &[
    VocabularyConflict {
        phrase: "Clean < Secret < DerivedFromSecret",
        reason: "wrong lattice order",
    },
    VocabularyConflict {
        phrase: "Secret downgrades to Clean",
        reason: "secret downgrade claim",
    },
    VocabularyConflict {
        phrase: "Private",
        reason: "unknown taint term",
    },
];

pub(crate) const CONTROL_FLOW_CONFLICTS: &[&str] = &[
    "tracks secret branch-condition taint",
    "tracks branch-condition taint",
];

pub(crate) const FINISH_REQUIRED: &str = "Finished(SlotValue, Taint)";
