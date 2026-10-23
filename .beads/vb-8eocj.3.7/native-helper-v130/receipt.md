# V130: test-only helper placement

Exact source b729e3a37b0c826a63edb5744d39855d0ee5666b. Local3090 proposed support.rs -> support_tests.rs with only tests.rs path attribute updated. Helper bytes SHA256 789d0d8c84d603c94bbfb270e5ed41cc62e6292fecf54b86de7b37353d531f7f preserved; all assertions, production code, and gate script unchanged.

Native panic-surface exit0 4.069 seconds; full vb_core library strict warnings serial exit0 7.476 seconds, 2205 passed / zero failed ignored filtered. Before/after source maps unchanged SHA256 1d1fbe4ffc05fb1c6cc0e83e9b5d5083309c966c93f320a0026ea507000d67cd. These focused results repair the known scan failure, not whole canonical CI acceptance.
