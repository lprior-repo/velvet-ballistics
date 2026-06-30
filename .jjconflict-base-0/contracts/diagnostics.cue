package validation

#ContractMeta: {
	schema_version: "1.0.0"
	kind: "diagnostics"
	...
}

#Diagnostics: #ContractMeta & {
	error_codes: [...string]
	render_format: "text" | "json"
}
