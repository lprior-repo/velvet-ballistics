---- MODULE ArtifactDigest ----
EXTENDS Naturals, FiniteSets, TLC

\* Obligations: TLA-ARTIFACT-002
\* Model of artifact digest computation and verification.
\* Stored artifact digest must always match sha256 of stored IR bytes.
\*
\* The core invariant: storedDigest equals the hash of irBytes.
\* ComputeDigest abstracts the blake3 hash as a modular sum.

VARIABLES storedDigest, irBytes, computedDigest, artifactState

vars == <<storedDigest, irBytes, computedDigest, artifactState>>

\* Bounded domain for tractable model checking.
ByteDomain == 0..3

Init ==
    /\ artifactState = "Pending"
    /\ storedDigest \in ByteDomain
    /\ irBytes \in ByteDomain
    /\ computedDigest \in ByteDomain

\* Abstraction of blake3: digest = (sum of IR bytes + 1) % 4.
ComputeDigest(ir) == (ir + 1) % 4

StoreArtifact ==
    /\ artifactState = "Pending"
    /\ artifactState' = "Stored"
    /\ irBytes' \in ByteDomain
    /\ storedDigest' = ComputeDigest(irBytes')
    /\ computedDigest' = storedDigest'
    /\ UNCHANGED <<artifactState>>

LoadArtifact ==
    /\ artifactState = "Stored"
    /\ artifactState' = "Admitted"
    /\ storedDigest' = storedDigest
    /\ irBytes' = irBytes
    /\ computedDigest' = computedDigest

Stutter == UNCHANGED vars

Next ==
    \/ StoreArtifact
    \/ LoadArtifact
    \/ Stutter

Spec == Init /\ [][Next]_vars

\* ---- Invariants ----

\* Stored artifact digest always matches the computed digest of IR bytes.
DigestMatchesIR ==
    artifactState = "Stored"
        => storedDigest = ComputeDigest(irBytes)

====