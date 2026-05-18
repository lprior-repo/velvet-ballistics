package validation

#ContractMeta: {
	schema_version: string
	kind: "diagnostics"
}

#Diagnostics: #ContractMeta & {
	error_codes: [...string]
	render_format: "text" | "json"
}
