# V200 CLI build and V201 interrupted canonical CI

The CLI build passed on source747a8830613ca9f3a7a3fe513a32b5b45562d71a. V201 ran `moon ci` against d0645504 with concurrency1. Its full native test summary reports14728 passed and27 skipped. The overall CI process was killed by systemd-oomd while baseline Kani was still active. This is NO VERDICT, not acceptance or a passing CI gate.

Exact argv, output, before/after source maps, binary hashes and journal evidence are preserved. The old Moon ciReport was not used. The only tracked tree change at termination was the generated Verus trust-scan report with identical line multiset and changed ordering; its patch is retained. Full F1 and main landing remain pending. See receipt.json for scope and limitations.

## Corrected stage classifications

Raw stderr also contains a missing `hardened` profile error(line357) and stale benchmark-evidence rejection(line462). These are RED stages, not successful checks; Moon duration/completion markers are insufficient. Original logs are retained unchanged. Overall acceptance remains blocked by these failures and interrupted baseline Kani.
