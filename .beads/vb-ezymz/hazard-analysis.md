# hazard-analysis

Hazards: accepting malformed trailing bytes; dropping input values; confusing raw-source/compiled/artifact digest; publishing header without artifact/inputs; acknowledging strict before sync; duplicate run overwrites; using arena handles without arena; driving recovery before seeding inputs. Contract C1-C7 define fail-closed guards. Persisted decode is bounded by MAX_RUN_INPUT_BYTES and schema validation by u16 slot bound. No performance claim.

See contract.md C1-C7.
