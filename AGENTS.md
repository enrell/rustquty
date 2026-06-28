# Agent Instructions

## Complete Release Process

Use this checklist when asked to cut and publish a rustquty release.

1. Verify the latest published versions before choosing the next version.
   - Check both crates:
     - `curl -A 'rustquty-release-script (contact: local)' -sSf https://crates.io/api/v1/crates/rustquty`
     - `curl -A 'rustquty-release-script (contact: local)' -sSf https://crates.io/api/v1/crates/rustquty-core`
   - Confirm `default_version`, `max_version`, and `newest_version`.
   - Choose the next semver version based on the change type.

2. Update release metadata and docs.
   - Bump `rustquty-core/Cargo.toml`.
   - Bump `rustquty/Cargo.toml`.
   - Update the `rustquty-core` dependency version in `rustquty/Cargo.toml`.
   - Run `cargo check` to refresh `Cargo.lock`.
   - Add a new section to `CHANGELOG.md` with the release date.
   - Update `README.md` and `rustquty-core/README.md` when behavior, configuration, CLI output, or public APIs changed.
   - Keep changelog release links in sync with the new `vX.Y.Z` tag.

3. Verify locally before publishing.
   - Run:
     - `cargo test -p rustquty-core`
     - `cargo test -p rustquty`
   - Run publish dry-runs in dependency order:
     - `cargo publish -p rustquty-core --dry-run --allow-dirty`
     - `cargo publish -p rustquty --dry-run --allow-dirty`
   - It is expected for the `rustquty` dry-run to fail before `rustquty-core` is published if it depends on a not-yet-published new `rustquty-core` version. Treat other dry-run failures as blockers.

4. Commit and push.
   - Review `git status --short` and `git diff --stat`.
   - Stage only intended tracked release files; do not accidentally add unrelated untracked files.
   - Commit with a concise release/fix message.
   - Push `main` to all configured project remotes that are kept in sync, currently `origin` and `gitlab`.

5. Publish to crates.io.
   - Publish in dependency order:
     - `cargo publish -p rustquty-core`
     - Wait until crates.io indexes `rustquty-core`.
     - `cargo publish -p rustquty`
   - Do not use `--allow-dirty` for the real publish unless there is a deliberate, documented reason.

6. Tag and push the release.
   - Create the tag after the release commit is pushed:
     - `git tag vX.Y.Z`
   - Push the tag to all configured project remotes:
     - `git push origin vX.Y.Z`
     - `git push gitlab vX.Y.Z`

7. Final verification.
   - Re-check crates.io for both crates and confirm `default_version`, `max_version`, and `newest_version` match the release.
   - Run `git status --short` and confirm only intentional unrelated local files remain.
   - Report the commit hash, tag, pushed remotes, published crate versions, test results, and any remaining worktree notes.
