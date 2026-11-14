# Isolated source recovery checkpoint

This checkpoint preserves repository source recovered from `/home/lewis/src/isoloated/` at the user's request. It is a recovery archive; its contents are not accepted production changes and have not passed a combined release gate. Original workspaces were not modified.

The scan covered 198 directories: 193 with a valid Git HEAD, four empty Git worktrees with recoverable JJ commit references, and one directory containing only local evidence. Initial snapshots contain 82 nonempty ordinary-source patches and 16 nonempty untracked-source manifests. A supplemental scan captured all tracked paths in 190 available workspaces, with 83 nonempty patches. `v217`, `v226`, and `v227` became unavailable before that supplemental scan; their initial snapshots remain included. `refs.tsv` records 99 distinct preserved commit tips and their remote branches under `codex/preserve-1113/`.

Archive SHA-256: `cc881617552d09a389ebba903b8540dc8309826462e5d8aba03b8defb46a70f7`.

Each `snapshots/<workspace>/` directory contains the original `head.txt`, a NUL-delimited Git status in `status.z`, a binary-capable `changes.patch` against that recorded HEAD, a NUL-delimited untracked-source manifest in `untracked.z`, and those untracked files in `untracked.tar.gz`. Where present, `complete-tracked.patch` supersedes `changes.patch` and includes tracked configuration, instructions, bead documents, and root tooling tests against the same recorded HEAD. Supplemental untracked manifests and archives cover root tooling tests, GitHub configuration, and bead Markdown files. The four empty worktrees have recoverable `jj-head.txt` references. These snapshots were taken while other sessions could still be active.

The initial source capture covers `crates`, `scripts`, `xtask`, `contracts`, `verification`, `fuzz`, `docs`, and `rules`, plus the Cargo manifests, lockfile, Moon project file, and master contract. Runtime databases, build outputs, local evidence, fuzz corpus files, conflict-copy directories, and untracked `.env*` files are excluded. Existing instruction/configuration edits were preserved only as recovery material.

Recover a workspace in a new isolated Git/JJ workspace using its recorded remote commit tip or original HEAD. Extract the outer archive into a persistent recovery directory, inspect the patch and original status, apply `complete-tracked.patch` where available or otherwise `changes.patch` with Git's binary patch support, then extract the untracked archives into that recovery workspace. Apply only one tracked patch. Review all recovered files and run the applicable repository gates before landing. Keep the archive and preserved branches until recovery is complete.

The large shipping integrations remain unaccepted because their recorded combined tests failed. Their preserved branches retain the original commit history, including review probes and counterfactuals that must never be mistaken for approved implementations.
