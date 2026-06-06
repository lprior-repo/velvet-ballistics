use crate::positive::{
    admitted_query_summary, admitted_slug_summary, checked_pair_sum, validated_query_count,
    validated_query_path_depth, validated_slug_count, validated_slug_path_depth, validated_total,
};

pub fn invalid_state_probes_fail_under_flux() {
    let _too_many_slugs = validated_slug_count(65536);
    let _too_many_queries = validated_query_count(65536);
    let _slug_too_deep = validated_slug_path_depth(17);
    let _query_too_deep = validated_query_path_depth(17);
    let _mismatch = validated_total(12, 13);
    let _overflow = checked_pair_sum(18446744073709551615, 1);
    let _slug_over_budget = admitted_slug_summary(0, 26, 26, 0, 25);
    let _query_over_budget = admitted_query_summary(0, 26, 26, 0, 25);
}
