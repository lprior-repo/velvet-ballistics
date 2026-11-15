# Compact integration candidate

This workspace contains the identical seven reviewed source files from raw assurance commit c21793a29b0b595d9f079e8b45dd0eb254e90e9e, plus contracts, execution logs, independent reviews, source hashes and compact archive tarballs. The expanded 415-file raw archives remain preserved in that commit on codex/taxonomy60-bounded-r24 and in /home/lewis/src/isoloated/vb-taxonomy60-r24. They are intentionally absent from this compact candidate: individual sparse Fjall journal files have 64 MiB logical lengths, and GitHub warned about those blobs when the raw assurance branch was published. No original commit or data was rewritten or deleted.

source-manifest.json is byte-identical to the raw assurance receipt and identifies all seven source hashes. The observed 47/47 combined tests, 29/29 existing helper tests, source lint, formatting and 34-verification trusted Verus projection were run on those exact source bytes in the assurance workspace. This compact workspace introduces packaging only; it does not claim a separate native rebuild. Central integration owns joined CI and main publication.

Normal native tests use genuine fresh issuer APIs when the two archive variables are absent. To replay the frozen bytes instead, extract the two tarballs only under an assigned persistent workspace, verify both manifests, and set VB_TAXONOMY_ARCHIVE and VB_TAXONOMY_RETRY_ARCHIVE to the expanded directories. The fixture opens private copies and refuses a missing explicitly requested archive. Example from an assigned isolated workspace root:

```sh
tar -xzf .evidence/taxonomy60/retained-current-main-v1.tar.gz -C .evidence/taxonomy60
tar -xzf .evidence/taxonomy60/retry-retained-current-main-v1.tar.gz -C .evidence/taxonomy60
(cd .evidence/taxonomy60 && sha256sum -c retained-v3.sha256 && sha256sum -c retry-retained-v1.sha256)
```

The compact candidate changes no catalog/verifier source-fingerprint input, cold record encoding, persisted ABI or executor guard. The user requires exhaustive MASTER65 implications, so canonical local-executor completion remains BLOCKED and no canonical taxonomy or cold adapter is claimed. packet.md supplies the bounded behavior and proof limitations; its raw archive paths refer to the preserved assurance packet or to an explicitly extracted copy.
