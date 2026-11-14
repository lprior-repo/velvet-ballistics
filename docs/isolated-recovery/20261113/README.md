# Isolated source recovery checkpoint

This checkpoint preserves repository source recovered from `/home/lewis/src/isoloated/` at the user's request. It is a recovery archive; its contents are not accepted production changes and have not passed a combined release gate. Original workspaces were not modified.

The scan covered 198 directories: 193 with a valid Git HEAD, four empty Git worktrees with recoverable JJ commit references, and one directory containing only local evidence. The archive contains 82 nonempty tracked-source patches and 16 nonempty untracked-source manifests. `refs.tsv` records 99 distinct preserved commit tips and their remote branches under `codex/preserve-1113/`.

Archive SHA-256: `888f09eef2d2ae0d33bb92d232d1eb54b88b4e139e57d41dcd2a5512a69120cf`.

Each `snapshots/<workspace>/` directory contains the original `head.txt`, a NUL-delimited Git status in `status.z`, a binary-capable `changes.patch` against HEAD, a NUL-delimited untracked-source manifest in `untracked.z`, and those untracked files in `untracked.tar.gz`. The four empty worktrees have `jj-head.txt` instead. The archive covers ordinary source and repository documents under `crates`, `scripts`, `xtask`, `contracts`, `verification`, `fuzz`, `docs`, and `rules`, plus the Cargo manifests, lockfile, Moon project file, and master contract. Runtime databases, build outputs, local evidence, fuzz corpus files, conflict-copy directories, and untracked credential files are excluded. Existing instruction/configuration edits were not integrated into main.

Recover a workspace in a new isolated Git/JJ workspace using its recorded remote commit tip or original HEAD. Extract the outer archive into a persistent recovery directory, inspect the patch and original status, apply `changes.patch` with Git's binary patch support, then extract `untracked.tar.gz` into that recovery workspace. Review all recovered files and run the applicable repository gates before landing. Keep the archive and preserved branches until recovery is complete.

The large shipping integrations remain unaccepted because their recorded combined tests failed. Their preserved branches retain the original commit history, including review probes and counterfactuals that must never be mistaken for approved implementations.
