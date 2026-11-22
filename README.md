# changelog

A Rust CLI that arranges commits between git tags into a
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) formatted
`CHANGELOG.md`.

[Conventional Commits](https://www.conventionalcommits.org) are parsed
automatically:

| Commit type | Section |
|---|---|
| `feat` | Added |
| `fix` | Fixed |
| `perf`, `refactor` | Changed |
| `revert`, `remove` | Removed |
| `deprecated` | Deprecated |
| `security` | Security |
| `feat!` / `BREAKING CHANGE:` | Added/Changed with a `**BREAKING:**` marker |
| `chore`, `docs`, `style`, `ci`, `test`, `build` | skipped |
| anything else | Uncategorized |

Commits that share a Conventional Commit scope are grouped under one bullet,
so a handful of `fix(git)` commits reads as one topic:

```markdown
### Fixed

- **git**:
    - add ignore orig files while merging
    - remove broken scopes alias
- **cicd**: add lint check
- handle empty body
```

## Usage

```sh
changelog                             # latest tag → HEAD, print [Unreleased] to stdout
changelog init                        # scaffold a fresh CHANGELOG.md (refuses to overwrite)
changelog init --force                # overwrite an existing file
changelog init --with-history         # init + pre-populate [Unreleased] from commits
changelog --from v1.2.0 --to v1.3.0   # explicit range
changelog --all                       # full history, one section per tag
changelog --in-place                  # splice the section into the existing CHANGELOG.md
changelog --in-place --dry-run        # preview the changes without writing
changelog --in-place --release 1.0.0  # rename [Unreleased] → [1.0.0] - today, fresh [Unreleased]
changelog suggest                     # suggest the next semver version (feat→minor, fix→patch, breaking→major)
```

## Configuration (`.changelogrc.toml`)

```toml
# Exclude commits whose subject contains any of these substrings.
exclude = ["internal refactor"]

# Custom Conventional Commit type → section mapping.
# Sections: Added, Changed, Deprecated, Removed, Fixed, Security,
#           Uncategorized, Skip
[types]
"exp" = "Added"
"ops" = "Skip"
```

## Building

```sh
cargo build --release   # binary at target/release/changelog
cargo test              # unit + integration tests (insta snapshots)
```

## Working on tests

Integration tests are snapshot tests powered by
[insta](https://insta.rs). Each test compares its output against a committed
`.snap` file in `tests/snapshots/`, so those files are part of the repo —
commit them whenever you intentionally change a test's expected output.

When a test fails, insta writes the new output next to the snapshot as
`<name>.snap.new` instead of failing silently. Review the diff, then either
accept it (replace the old snapshot, e.g. with `cargo insta accept` or by
moving the file over the `.snap`) or reject it (delete the file). Never
commit `.snap.new` files — they're pending reviews, not assertions.

