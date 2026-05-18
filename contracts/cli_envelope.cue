package validation

#ContractMeta: {
	schema_version: string
	kind: "cli_envelope"
}

#CLIEnvelope: #ContractMeta & {
	command: string
	args: [...string]
	exit_codes: [...number]
}
