package validation

#ContractMeta: {
	schema_version: "1.0.0"
	kind: "evidence_bundle"
	...
}

#EvidenceBundle: #ContractMeta & {
	gates_required: [...string]
	evidence_shape: {
		gate_name: string
		exit_code: number
		status: "passed" | "failed" | "skipped"
	}
}
