# changelog

A Rust CLI that arranges commits between git tags into a
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) formatted
`CHANGELOG.md`. Conventional Commits are parsed automatically
(`feat` → Added, `fix` → Fixed, breaking changes highlighted); non-conforming
commits land in an `Uncategorized` section, and chores/docs/CI commits are
skipped.

## Usage

```sh
changelog                       # latest tag → HEAD, print [Unreleased] to stdout
changelog init                  # scaffold a fresh CHANGELOG.md (refuses to overwrite)
changelog init --with-history   # init + pre-populate [Unreleased] from commits
changelog --from v1.2.0 --to v1.3.0
changelog --in-place            # splice the section into the existing CHANGELOG.md
changelog --in-place --dry-run  # preview the changes without writing
changelog --in-place --release 1.0.0  # rename [Unreleased] → [1.0.0] - today, fresh [Unreleased]
```
