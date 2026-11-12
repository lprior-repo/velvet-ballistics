# workflow-model

Raw CLI bytes -> compiled numeric workflow -> parsed typed input mapping -> prepared accepted artifact -> staged admission batch -> committed admission -> response. Any preparation/staging failure ends without commit. None is a metadata submission, Journaled is committed with loss window, Strict is SyncAll. Recovery starts with retained inputs before drive.

See contract.md C1-C7.
