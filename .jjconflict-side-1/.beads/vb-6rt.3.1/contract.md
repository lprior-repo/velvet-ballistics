# vb-6rt.3.1 contract

Given sibling mappings named left and right with the same field name value, the nine mapped event nodes have the independently measured byte offsets and one-based line/column coordinates specified in nested_mapping_node_indices_and_spans. Both the lookup method and free span_for_node function must return each exact expected SourceSpan, including distinct locations for the repeated field name.

Given the literal source `x: |\n  café\n  end\n`, utf8_block_scalar_span requires scalar event index 2 to span bytes 7 through 19, from line 2 column 3 to line 4 column 1. The pinned saphyr-parser 0.0.6 scan_block_scalar routine starts this token at the first content character after the header and indentation; this golden follows that token convention rather than requiring the block indicator to be included. Offsets are UTF-8 bytes and the exclusive end includes the final newline for this concrete source.

The first unmapped index 9 in the nested example and indices 9999 and u32::MAX in the simple example must return None through both lookup entry points. out_of_range_indices_do_not_panic covers the latter pair.

Production code remains unchanged. These three registered native scenarios qualify the named location invariants; separate Sol CODE, independent Sol TEST, and integrated gates are required. They do not establish full source-map, parser, or formal acceptance.
