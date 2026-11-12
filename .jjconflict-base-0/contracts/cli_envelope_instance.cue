package validation

// CLI envelope contract — defines the xtask command interface
kind: "cli_envelope"
schema_version: "1.0.0"

#CLIEnvelope: {
	command: "cargo xtask"
	args: ["--help", "--version"]
	exit_codes: [0, 1, 2]
}
