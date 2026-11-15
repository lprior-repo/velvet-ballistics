# Trigger key source spans

Recover the useful test intent from preserved source539e1b6c/220c5c18. The current public semantic source map reports the exact trigger key span for manual, schedule, event and webhook container paths. Pin UTF-8 byte offsets and key text for all four, despite earlier repeated trigger words, and contrast an ordinary container span plus absent path. This tests source location tracking only, not parsing a valid workflow or activating trigger ingress. Existing source, parser/trigger behavior, APIs, gates, dependencies and tests remain unchanged.
