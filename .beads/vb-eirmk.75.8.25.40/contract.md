# SourceSpan coordinate normalization

The current pinned parser supplies character offsets and zero-based start columns. SourceSpan documents UTF-8 byte offsets and one-based line/column coordinates. Convert at the shared event_span_to_source_span boundary used by every ordinary and semantic source-map path, with exact Unicode regressions and existing ASCII behavior preserved apart from the corrected start column. Keep the low-level EventSpan conversion, public signatures, parser/profile/trigger acceptance, serialization, dependencies and gates unchanged. Do not claim this repairs every standalone EventSpan API consumer.

Retain the failing four-trigger matrix .39 unchanged, including earlier repeated words and Unicode. Add an ordinary source-map multibyte scalar case with exact byte range and one-based columns. Existing YAML and compiler suites plus independent code/test review are required. No GPU or benchmark work.
