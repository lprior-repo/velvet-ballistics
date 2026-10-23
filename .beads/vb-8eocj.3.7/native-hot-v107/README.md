# Hot region native qualification and compatibility failures

V103 a16e62bf2f480d1ef9be080ac9292f9d43e6eab5:4 helper test functions PASS (six bad states, successful reset, PC/latch and Pending preservation). These are finite helper cases, not full-driver source acceptance. V104 same source strict3lib Clippy FAIL unnecessary lazy error construction. V105 c56322f1cd3cb387007d283c58096e118786d1ac strict3lib Clippy PASS after one mechanical ok_or replacement.

V106 7a0b55e672a9449f0dde03add0cea097d3ffd71d: compiler505 and core2193 PASS, runtime1989 PASS/9 FAIL. All nine failures concern existing direct-primitive reentry compatibility. Seven earlier constructor fixtures now validate with their original behavioral assertions retained; runtime uninitialized-slot assertions strengthened to exact slot0. No test ignored or loosened. Library gate remains RED pending compatibility repair.

V107 same7a0b55 source: mixed canonical source/constructor target aggregate100 PASS, exit0 1.974s, same source maps. This does not include the two still-RED multi-step source fixtures.

No whole F1/F2, complete nested execution, recovered error, canonical CI or main landing claim.
