---- MODULE vb_t6hx_scan_limit ----
EXTENDS Naturals, TLC

CONSTANTS MaxLimit, MaxFixtureRows
VARIABLES limit, fixtureRows, emitted, phase

Init == /\ limit \in 1..MaxLimit /\ fixtureRows \in 0..MaxFixtureRows
        /\ emitted = 0 /\ phase = "Scanning"

Emit == /\ phase = "Scanning" /\ emitted < limit /\ emitted < fixtureRows
        /\ emitted' = emitted + 1 /\ UNCHANGED <<limit, fixtureRows, phase>>
Stop == /\ phase = "Scanning" /\ (emitted = limit \/ emitted = fixtureRows)
        /\ phase' = "Done" /\ UNCHANGED <<limit, fixtureRows, emitted>>
Stutter == /\ phase = "Done" /\ UNCHANGED <<limit, fixtureRows, emitted, phase>>

Next == Emit \/ Stop \/ Stutter
RowsNeverExceedLimit == emitted <= limit
TypeOK == /\ limit \in 1..MaxLimit /\ fixtureRows \in 0..MaxFixtureRows
          /\ emitted \in 0..MaxLimit /\ phase \in {"Scanning", "Done"}

====
