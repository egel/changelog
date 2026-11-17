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
