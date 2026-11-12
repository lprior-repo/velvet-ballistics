package validation

#ContractMeta: {
	schema_version: "1.0.0"
	kind: "accepted_artifacts"
	...
}

#AcceptedArtifacts: #ContractMeta & {
	artifact_types: [...string]
	metadata_required: [...string]
}
