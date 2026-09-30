# Bake License

`bake-license` provides reusable license and copyright maintenance tasks for
Bake projects. Its `license:update` task generates `license.md` from Git history,
updates copyright headers in tracked Rust source files, and removes the License
section from `readme.md`.

Add the crate to an unpublished `bake/` task binary and reference it once so its
task registration is linked:

```toml
[dependencies]
bake-license = { version = "0.1" }
```

```rust,ignore
use bake_license as _;
```

Then run:

```sh
cargo bake license:update
```

The task uses Git author names and dates from the current branch, honoring
`.mailmap` and excluding revisions listed in `.git-blame-ignore-revs`. It skips
bot accounts ending in `[bot]`. Existing Rust documentation comments are retained
when updating file headers. Git history does not include uncommitted or
untracked files, so commit new files before expecting them to receive a header.

The task executable also links Bake Agent Context. Run
`cargo bake agent:context:install` to install context from dependencies such as
`bake`; generated `.agents/context/` files are ignored by Git. Shared
Rust guidance lives in [Bake Agent Context](https://github.com/socketry/bake-agent-context-rust/blob/main/context/rust.md).
