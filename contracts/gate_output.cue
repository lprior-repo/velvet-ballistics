package validation

#ContractMeta: {
	schema_version: string
	kind: "gate_output"
}

#GateOutput: #ContractMeta & {
	gate_kind: string
	gate_name: string
	status: "pass" | "fail" | "skipped"
	why_failed?: {
		hint: string
		repair_command: string
	}
}
