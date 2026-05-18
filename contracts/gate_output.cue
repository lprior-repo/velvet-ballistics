package validation

#ContractMeta: {
	schema_version: "1.0.0"
	kind: "gate_output"
	...
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
