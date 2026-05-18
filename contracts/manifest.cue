package validation

// Master manifest tracking all contract files and their versions.
contract_registry: {
	[...string]: {
		path: string
		schema_version: string
		kind: "cli_envelope" | "ui_tokens" | "accepted_artifacts" | "evidence_bundle" | "diagnostics" | "gate_output"
		last_validated: string
	}
}
