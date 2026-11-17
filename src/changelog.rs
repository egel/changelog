//! Changelog rendering and in-place editing of CHANGELOG.md.

use crate::cli::Cli;
use crate::commits::{classify, Entry, ParsedCommit};
use crate::config::Config;
use crate::git::{commits_between, CommitInfo};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Section order defined by Keep a Changelog 1.0.0.
const SECTION_ORDER: [Entry; 6] = [
    Entry::Added,
    Entry::Changed,
    Entry::Deprecated,
    Entry::Removed,
    Entry::Fixed,
    Entry::Security,
];

pub const CHANGELOG_FILE: &str = "CHANGELOG.md";

/// Collect and classify commits, grouped by section.
pub fn collect_grouped(
    commits: &[CommitInfo],
    config: &Config,
) -> BTreeMap<Entry, Vec<ParsedCommit>> {
    let mut grouped: BTreeMap<Entry, Vec<ParsedCommit>> = BTreeMap::new();
    for commit in commits {
        if let Some(parsed) = classify(commit, config) {
            grouped.entry(parsed.entry.clone()).or_default().push(parsed);
        }
    }
    grouped
}

/// Render the `[Unreleased]` section markdown.
pub fn render_unreleased(grouped: &BTreeMap<Entry, Vec<ParsedCommit>>, title: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("## {}\n\n", title));
    for entry in SECTION_ORDER {
        if let Some(items) = grouped.get(&entry) {
            if !items.is_empty() {
                out.push_str(&format!("### {}\n", entry.heading()));
                for item in items {
                    for line in render_item(item) {
                        out.push_str(&line);
                        out.push('\n');
                    }
                }
                out.push('\n');
            }
        }
    }

    if let Some(items) = grouped.get(&Entry::Uncategorized) {
        if !items.is_empty() {
            out.push_str("### Uncategorized\n");
            for item in items {
                for line in render_item(item) {
                    out.push_str(&line);
                    out.push('\n');
                }
            }
            out.push('\n');
        }
    }

    out
}

fn render_item(commit: &ParsedCommit) -> Vec<String> {
    let mut lines = Vec::new();
    if commit.breaking {
        lines.push(format!("- **BREAKING:** {}", commit.subject));
        if let Some(desc) = &commit.breaking_description {
            lines.push(format!("  {}", desc.trim_end_matches('.')));
        }
    } else {
        lines.push(format!("- {}", commit.subject));
    }
    lines
}

/// `suggest` subcommand: semver bump suggestion from commit types.
pub fn run_suggest(repo_path: &Path) -> Result<()> {
    // Latest tag must be a version tag to base the bump on.
    let repo = git2::Repository::open(repo_path)?;
    let (tag_name, _) = crate::git::latest_tag(&repo)?
        .ok_or_else(|| anyhow::anyhow!("no tags found in repository"))?;

    let current = semver::Version::parse(tag_name.trim_start_matches('v'))
        .map_err(|_| anyhow::anyhow!("latest tag '{}' is not a semantic version", tag_name))?;

    let commits = commits_between(repo_path, None, None)?;
    let config = Config::load(repo_path)?;
    let mut bump = Bump::Patch;
    for commit in &commits {
        match classify(commit, &config) {
            None => continue,
            Some(parsed) if parsed.breaking => bump = Bump::Major,
            Some(parsed) if parsed.entry == Entry::Added => bump = bump.max(Bump::Minor),
            Some(parsed) if parsed.entry == Entry::Fixed => bump = bump.max(Bump::Patch),
            Some(parsed) if parsed.entry == Entry::Removed => bump = bump.max(Bump::Minor),
            _ => {}
        }
    }

    let next = semver::Version {
        major: current.major + if bump == Bump::Major { 1 } else { 0 },
        minor: if bump == Bump::Major {
            0
        } else {
            current.minor + if bump == Bump::Minor { 1 } else { 0 }
        },
        patch: if bump == Bump::Patch { current.patch + 1 } else { 0 },
        pre: semver::Prerelease::EMPTY,
        build: semver::BuildMetadata::EMPTY,
    };
    println!("{}", next);
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Bump {
    Patch,
    Minor,
    Major,
}
/// Default command: generate the [Unreleased] section from git commits.
pub fn run_generate(cli: &Cli, repo_path: &Path) -> Result<()> {
    let config = Config::load(repo_path)?;
    let commits = commits_between(repo_path, cli.from.as_deref(), cli.to.as_deref())?;
    let grouped = collect_grouped(&commits, &config);
    let markdown = render_unreleased(&grouped, "[Unreleased]");
    print!("{}", markdown);
    Ok(())
}

/// `init` subcommand: scaffold a fresh CHANGELOG.md.
pub fn run_init(repo_path: &Path, force: bool, with_history: bool, cli: &Cli) -> Result<()> {
    let path = repo_path.join(CHANGELOG_FILE);
    if path.exists() && !force {
        anyhow::bail!(
            "{} already exists; use --force to overwrite it, or run the default command to fill [Unreleased]",
            path.display()
        );
    }

    let mut content = scaffold_header().to_string();

    if with_history {
        // Header only; the history fills [Unreleased] below.
        let config = Config::load(repo_path)?;
        let commits = commits_between(repo_path, cli.from.as_deref(), cli.to.as_deref())?;
        let grouped = collect_grouped(&commits, &config);
        content.push_str(&render_unreleased(&grouped, "[Unreleased]"));
    } else {
        content.push_str(&empty_unreleased());
    }

    std::fs::write(&path, content)
        .with_context(|| format!("failed to write {}", path.display()))?;
    eprintln!("Created {}", path.display());
    Ok(())
}

/// Header lines of a fresh CHANGELOG.md, without the [Unreleased] section.
fn scaffold_header() -> &'static str {
    "# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

"
}

/// Empty [Unreleased] section with all Keep a Changelog headings.
fn empty_unreleased() -> String {
    let mut out = String::from("## [Unreleased]\n\n");
    for heading in [
        "Added",
        "Changed",
        "Deprecated",
        "Removed",
        "Fixed",
        "Security",
    ] {
        out.push_str(&format!("### {}\n", heading));
    }
    out.push('\n');
    out
}

/// `--in-place`: splice the generated section into the existing CHANGELOG.md.
pub fn run_in_place(cli: &Cli, repo_path: &Path) -> Result<()> {
    let path = locate_changelog(repo_path)?;
    let existing = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let config = Config::load(repo_path)?;
    let commits = commits_between(repo_path, cli.from.as_deref(), cli.to.as_deref())?;
    let grouped = collect_grouped(&commits, &config);

    // With --version, the new content goes into a release section; otherwise Unreleased.
    let (title, updated) = match &cli.release {
        Some(version) => {
            let today = chrono_date();
            let new_section = empty_unreleased();
            let release = replace_unreleased_with_version(
                &existing,
                &format!("{} - {}", version, today),
                &new_section,
            )?;
            (String::new(), release)
        }
        None => {
            let markdown = render_unreleased(&grouped, "[Unreleased]");
            match replace_unreleased(&existing, &markdown) {
                Ok(updated) => (String::new(), updated),
                Err(_) => {
                    anyhow::bail!(
                        "no '## [Unreleased]' section found in {}; run 'changelog init' first",
                        path.display()
                    )
                }
            }
        }
    };
    let _ = title;

    if cli.dry_run {
        print_diff(&existing, &updated);
        eprintln!("--dry-run: no changes written");
        return Ok(());
    }

    std::fs::write(&path, updated)
        .with_context(|| format!("failed to write {}", path.display()))?;
    eprintln!("Updated {}", path.display());
    Ok(())
}

/// Locate CHANGELOG.md in the repo root.
fn locate_changelog(repo_path: &Path) -> Result<PathBuf> {
    let path = repo_path.join(CHANGELOG_FILE);
    if path.exists() {
        Ok(path)
    } else {
        anyhow::bail!("{} not found; run 'changelog init' first", path.display())
    }
}

/// Replace the `[Unreleased]` section content with `new_section`.
fn replace_unreleased(existing: &str, new_section: &str) -> Result<String> {
    match find_unreleased_range(existing) {
        Some((start, end)) => {
            let mut updated = String::with_capacity(existing.len() + new_section.len());
            updated.push_str(&existing[..start]);
            updated.push_str(new_section);
            updated.push_str(&existing[end..]);
            Ok(updated)
        }
        None => Err(anyhow::anyhow!("no Unreleased section")),
    }
}

/// Rename `[Unreleased]` to a release heading, insert a fresh Unreleased above it.
fn replace_unreleased_with_version(existing: &str, release_heading: &str, new_unreleased: &str) -> Result<String> {
    let (start, end) = find_unreleased_range(existing)
        .ok_or_else(|| anyhow::anyhow!("no '## [Unreleased]' section found; run 'changelog init' first"))?;
    let unreleased_body = existing[start..end].to_string();
    let release_body = unreleased_body.replace("## [Unreleased]", &format!("## {}", release_heading));

    let mut updated = String::with_capacity(existing.len() + release_body.len() + new_unreleased.len());
    updated.push_str(&existing[..start]);
    updated.push_str(&release_body);
    updated.push_str(new_unreleased);
    updated.push_str(&existing[end..]);
    Ok(updated)
}

/// Find the byte range of the `## [Unreleased]` section (heading inclusive,
/// next `## ` heading exclusive). The end index points at the next `## `.
fn find_unreleased_range(content: &str) -> Option<(usize, usize)> {
    let start = content.find("## [Unreleased]")?;
    let after = &content[start + 1..];
    let end = match after.find("\n## ") {
        Some(pos) => start + 1 + pos + 1, // past the newline, at the next heading
        None => content.len(),
    };
    Some((start, end))
}

/// LCS-based line diff for --dry-run output.
fn print_diff(existing: &str, updated: &str) {
    let old: Vec<&str> = existing.lines().collect();
    let new: Vec<&str> = updated.lines().collect();
    let n = old.len();
    let m = new.len();

    // LCS length table.
    let mut table = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i][j] = if old[i] == new[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }

    // Walk the table, printing changed lines with context lines around hunks.
    let (mut i, mut j) = (0usize, 0usize);
    let mut in_hunk = false;
    let mut gap = 0usize;
    while i < n && j < m {
        if old[i] == new[j] {
            if in_hunk {
                println!("  {}", old[i]);
                gap += 1;
                if gap >= 3 {
                    in_hunk = false;
                }
            }
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            if !in_hunk {
                println!("@@ changelog @@");
                in_hunk = true;
            }
            gap = 0;
            println!("-{}", old[i]);
            i += 1;
        } else {
            if !in_hunk {
                println!("@@ changelog @@");
                in_hunk = true;
            }
            gap = 0;
            println!("+{}", new[j]);
            j += 1;
        }
    }
    for line in &old[i..] {
        if !in_hunk {
            println!("@@ changelog @@");
            in_hunk = true;
        }
        println!("-{}", line);
    }
    for line in &new[j..] {
        if !in_hunk {
            println!("@@ changelog @@");
            in_hunk = true;
        }
        println!("+{}", line);
    }
}

/// Today's date as YYYY-MM-DD.
pub fn chrono_date() -> String {
    // Avoid pulling chrono for one date: use std::time and manual formatting via
    // a tiny days-since-epoch conversion.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let days = secs / 86_400;
    let (y, m, d) = civil_from_days(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// Howard Hinnant's civil_from_days algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splice_replaces_unreleased_body() {
        let existing = "header\n\n## [Unreleased]\n\n### Added\n- old thing\n\n## [1.0.0]\n### Added\n- v1\n";
        let new_section = "## Unreleased\n\n### Fixed\n- new fix\n\n";
        let updated = replace_unreleased(existing, new_section).unwrap();
        assert!(updated.contains("### Fixed\n- new fix"));
        assert!(!updated.contains("old thing"));
        assert!(updated.contains("## [1.0.0]\n### Added\n- v1"));
    }

    #[test]
    fn unreleased_at_end_of_file() {
        let existing = "header\n\n## [Unreleased]\n\n### Added\n- thing\n";
        let new_section = "## Unreleased\n\n### Fixed\n- fix\n\n";
        let updated = replace_unreleased(existing, new_section).unwrap();
        assert!(updated.ends_with("### Fixed\n- fix\n\n"));
    }

    #[test]
    fn version_release_renames_and_inserts() {
        let existing = "header\n\n## [Unreleased]\n\n### Added\n- thing\n\n";
        let new_unreleased = "## Unreleased\n\n### Fixed\n- fix\n\n";
        let updated = replace_unreleased_with_version(existing, "1.2.3 - 2026-11-17", new_unreleased).unwrap();
        assert!(updated.contains("## 1.2.3 - 2026-11-17"));
        assert!(updated.contains("### Added\n- thing"));
        assert!(updated.contains("### Fixed\n- fix"));
    }
}
