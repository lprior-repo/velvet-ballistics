package validation

// UI tokens contract — defines the design token schema
kind: "ui_tokens"
schema_version: "1.0.0"

#UITokens: {
	token_set: "velvet_ui_tokens"
	properties: {
		primary_color: {
			type: "color"
			value: "#000000"
		}
		button_padding: {
			type: "spacing"
			value: "8px"
		}
	}
}
