# type-contracts

RunInputsRecord has private fields and a fallible constructor taking RunId, artifact digest, source digest, CompiledWorkflow and typed slot pairs. Constructor rejects zero run, duplicate/undeclared/out-of-range slots and kind mismatches. Load validates run identity and envelope checksum; recovery must revalidate schema against bound artifact before use. input_digest is recomputed over canonical slot-sorted typed pairs on decode and workflow validation; validate_admission rejects legacy-unbound, wrong run, artifact or input digest receipts.

See contract.md C1-C7.
