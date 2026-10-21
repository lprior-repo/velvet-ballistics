# Workflow
Source -> canonical compile with contracts -> IR reconstruction -> shared gates
including action completeness -> idempotency declaration check -> canonical digest
check -> budget computation/policy validation -> immutable checked candidate.
Any failure returns a typed error and produces no checked candidate. Admission,
persistence, durable acknowledgement and recovery belong to downstream transitions.
