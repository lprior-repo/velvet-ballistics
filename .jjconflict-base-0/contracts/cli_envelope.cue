package validation

#ContractMeta: {
	schema_version: "1.0.0"
	kind: "cli_envelope"
	...
}

#CLIEnvelope: #ContractMeta & {
	command: string
	args: [...string]
	exit_codes: [...number]
}
