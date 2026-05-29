---- MODULE vb_aoah_reopen_after_migration_no_rerun ----
EXTENDS Naturals, Sequences, TLC

\* Obligation: PO-022
\* Claim: Reopen current store does not rerun migration.
\* Bounded domains: u16 storage versions, u64-like finite counters, <= MaxRecords records.

CONSTANTS CurrentVersion, OldVersion, FutureVersion, MaxRecords, MaxBytes
VARIABLES phase, old_records, current_records, bytes, writes, migrated, verified, manifest_version, migration_runs, audit

Versions == {OldVersion, CurrentVersion, FutureVersion}
Bounded == /\ OldVersion \in 0..65535
           /\ CurrentVersion \in 0..65535
           /\ FutureVersion \in 0..65535
           /\ MaxRecords \in 0..4
           /\ MaxBytes \in 0..16

Init == /\ Bounded
        /\ phase = "OldStore"
        /\ old_records \in 0..MaxRecords
        /\ current_records = 0
        /\ bytes = 0
        /\ writes = 0
        /\ migrated = FALSE
        /\ verified = FALSE
        /\ manifest_version = OldVersion
        /\ migration_runs = 0
        /\ audit = <<>>

RuntimeOpenOld == /\ phase = "OldStore"
                  /\ manifest_version = OldVersion
                  /\ phase' = "MigrationRequired"
                  /\ writes' = writes
                  /\ UNCHANGED <<old_records, current_records, bytes, migrated, verified, manifest_version, migration_runs, audit>>

StartMigration == /\ phase \in {"MigrationRequired", "OldStore"}
                  /\ old_records > 0
                  /\ phase' = "Migrating"
                  /\ migration_runs' = migration_runs + 1
                  /\ UNCHANGED <<old_records, current_records, bytes, writes, migrated, verified, manifest_version, audit>>

CopyOne == /\ phase = "Migrating"
           /\ old_records > 0
           /\ bytes + 1 <= MaxBytes
           /\ old_records' = old_records - 1
           /\ current_records' = current_records + 1
           /\ bytes' = bytes + 1
           /\ writes' = writes + 1
           /\ UNCHANGED <<phase, migrated, verified, manifest_version, migration_runs, audit>>

OverflowErr == /\ phase = "Migrating"
               /\ bytes + 1 > MaxBytes
               /\ phase' = "BoundedError"
               /\ UNCHANGED <<old_records, current_records, bytes, writes, migrated, verified, manifest_version, migration_runs, audit>>

Verify == /\ phase = "Migrating"
          /\ old_records = 0
          /\ phase' = "Verified"
          /\ verified' = TRUE
          /\ migrated' = TRUE
          /\ UNCHANGED <<old_records, current_records, bytes, writes, manifest_version, migration_runs, audit>>

AdvanceManifest == /\ phase = "Verified"
                   /\ verified = TRUE
                   /\ manifest_version' = CurrentVersion
                   /\ phase' = "CurrentStore"
                   /\ audit' = Append(audit, "advanced")
                   /\ UNCHANGED <<old_records, current_records, bytes, writes, migrated, verified, migration_runs>>

CleanupSuccess == /\ phase = "CurrentStore"
                  /\ old_records = 0
                  /\ phase' = "CleanupSucceeded"
                  /\ UNCHANGED <<old_records, current_records, bytes, writes, migrated, verified, manifest_version, migration_runs, audit>>

OpenCurrent == /\ phase \in {"CurrentStore", "CleanupSucceeded"}
               /\ manifest_version = CurrentVersion
               /\ phase' = "CurrentStore"
               /\ migration_runs' = migration_runs
               /\ UNCHANGED <<old_records, current_records, bytes, writes, migrated, verified, manifest_version, audit>>

EmptyNoop == /\ phase = "OldStore"
             /\ old_records = 0
             /\ phase' = "NoopVerified"
             /\ verified' = TRUE
             /\ audit' = Append(audit, "empty-noop")
             /\ UNCHANGED <<old_records, current_records, bytes, writes, migrated, manifest_version, migration_runs>>

Stutter == UNCHANGED <<phase, old_records, current_records, bytes, writes, migrated, verified, manifest_version, migration_runs, audit>>

Next == RuntimeOpenOld \/ StartMigration \/ CopyOne \/ OverflowErr \/ Verify \/ AdvanceManifest \/ CleanupSuccess \/ OpenCurrent \/ EmptyNoop \/ Stutter

Spec == Init /\ [][Next]_<<phase, old_records, current_records, bytes, writes, migrated, verified, manifest_version, migration_runs, audit>>

TypeOK == /\ phase \in {"OldStore", "MigrationRequired", "Migrating", "BoundedError", "Verified", "CurrentStore", "CleanupSucceeded", "NoopVerified"}
          /\ old_records \in 0..MaxRecords
          /\ current_records \in 0..MaxRecords
          /\ bytes \in 0..MaxBytes
          /\ writes \in 0..MaxRecords
          /\ manifest_version \in Versions
          /\ migration_runs \in 0..MaxRecords

NoSideEffects == phase = "MigrationRequired" => writes = 0
NoAdvanceBeforeVerified == manifest_version = CurrentVersion => verified = TRUE
CleanupSuccessEmpty == phase = "CleanupSucceeded" => old_records = 0
NoRerunAfterSuccess == manifest_version = CurrentVersion => migration_runs <= 1
EmptyNoopAudited == phase = "NoopVerified" => Len(audit) = 1

====
