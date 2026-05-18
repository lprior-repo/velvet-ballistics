package validation

#ContractMeta: {
	schema_version: string
	kind: "ui_tokens"
}

#UITokens: #ContractMeta & {
	token_set: string
	properties: {
		[name: string]: {
			type: "color" | "spacing" | "typography" | "shadow" | "radius"
			value: string
		}
	}
}
