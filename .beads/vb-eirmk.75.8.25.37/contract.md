# Independent plan-validity guards

Exercise the current public PlanValidationReport::is_valid predicate with a valid report and four otherwise-identical reports that each contain exactly one failure class: missing section, duplicate section, missing required term or stale API mention. Each isolated failure must return false. Preserve all existing validator fixtures and assertions; no production helper, required terms, documents, gates, thresholds, schema or dependency changes. This is the narrow recovered test oracle from the named source, not mutation execution or product acceptance.
