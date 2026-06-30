package validation

#ContractMeta: {
	schema_version: "1.0.0"
	kind: "ui_tokens"
	...
}

#UITokens: #ContractMeta & {
	token_set: string
	properties: {
		[name=string]: {
			type: "color" | "spacing" | "typography" | "shadow" | "radius"
			value: string
		}
	}
}
