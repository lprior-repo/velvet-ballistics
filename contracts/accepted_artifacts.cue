package validation

#ContractMeta: {
	schema_version: string
	kind: "accepted_artifacts"
}

#AcceptedArtifacts: #ContractMeta & {
	artifact_types: [...string]
	metadata_required: [...string]
}
