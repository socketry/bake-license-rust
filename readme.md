# `bake-license`

`bake-license` provides a Bake task for maintaining the MIT license file and
copyright headers in Rust projects.

## Motivation

Updating license text and copyright headers by hand is repetitive. This task
uses the repository's Git history to keep them consistent with its authorship.

## Usage

Add `bake-license` to the private `bake/` task package:

```toml
[dependencies]
bake-license = "0.1"
```

Regenerate task links and run the updater:

```sh
cargo bake --regenerate
cargo bake license:update
```

The task updates `license.md` and tracked Rust source headers, honoring
`.mailmap` and `.git-blame-ignore-revs`. It skips bot accounts ending in
`[bot]`. Existing Rust documentation comments are retained. Commit new files
before running the task if they should receive copyright headers.

## Releasing

Prepare a release with `cargo bake cargo:version:patch` (or `minor`, `major`,
or `bump --version X.Y.Z`), then run `cargo bake cargo:release` and open a
pull request. After review and merge, GitHub Actions publishes the release
when the configured `crates-io` environment approves it. See the
[Cargo publishing guide](https://github.com/socketry/bake-cargo-rust/blob/main/context/publishing.md).

## Releases

<!-- bake-readme:releases:start -->
See [releases.md](releases.md) for the full release history.

### v0.1.3

- Preserve heading-like text inside HTML blocks when removing generated license sections.

### v0.1.2

- Create or update GitHub Releases after successful crates.io publication.
- Keep release versioning tasks compatible with the current local crate.

### v0.1.1

- Switch the runtime dependency from `socketry-bake` to `bake` 0.17.0.

- Add Bake Agent Context tasks to the project's development executable.
- Link the shared Rust context guidance from the README.
<!-- bake-readme:releases:end -->

## See Also

- [Bake](https://github.com/socketry/bake-rust) — composable development tasks
  for Rust projects.
- [socketry-project](https://github.com/socketry/socketry-project-rust) — shared
  project tasks and conventions for Socketry Rust crates.

## Contributing

Please open an issue or pull request on [GitHub](https://github.com/socketry/bake-license-rust).
