//! Conventional Commit parsing: `type(scope)!: subject` with optional body
//! and `BREAKING CHANGE:` footers. See https://www.conventionalcommits.org

use crate::config::Config;
use crate::git::CommitInfo;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Entry {
    Added,
    Changed,
    Deprecated,
    Removed,
    Fixed,
    Security,
    /// Commits that could not be classified.
    Uncategorized,
}

impl Entry {
    /// Keep a Changelog heading.
    pub fn heading(&self) -> &'static str {
        match self {
            Entry::Added => "Added",
            Entry::Changed => "Changed",
            Entry::Deprecated => "Deprecated",
            Entry::Removed => "Removed",
            Entry::Fixed => "Fixed",
            Entry::Security => "Security",
            Entry::Uncategorized => "Uncategorized",
        }
    }

    /// Parse a Keep a Changelog heading back into an Entry.
    pub fn from_heading(heading: &str) -> Entry {
        match heading {
            "Added" => Entry::Added,
            "Changed" => Entry::Changed,
            "Deprecated" => Entry::Deprecated,
            "Removed" => Entry::Removed,
            "Fixed" => Entry::Fixed,
            "Security" => Entry::Security,
            _ => Entry::Uncategorized,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedCommit {
    pub entry: Entry,
    /// Subject line to render in the changelog entry.
    pub subject: String,
    /// True if the commit is marked as a breaking change.
    pub breaking: bool,
    /// Extra description from a BREAKING CHANGE footer, if any.
    pub breaking_description: Option<String>,
    /// Conventional Commit scope (`feat(api)` → `Some("api")`), used to
    /// group related commits under one bullet.
    pub scope: Option<String>,
}

/// Classify a commit into a changelog entry. Returns `None` for commits that
/// should be skipped entirely (config-mapped, chores, docs, CI, etc.).
pub fn classify(commit: &CommitInfo, config: &Config) -> Option<ParsedCommit> {
    // Subject exclusions from config.
    if config
        .exclude
        .iter()
        .any(|pattern| commit.subject.contains(pattern.as_str()))
    {
        return None;
    }

    let conventional = parse_conventional(&commit.subject);

    let (entry, subject, scope) = match &conventional {
        Some(parsed) => {
            let section = config.section_for_type(&parsed.r#type)?;
            (Entry::from_heading(section), parsed.subject.clone(), parsed.scope.clone())
        }
        None => (Entry::Uncategorized, commit.subject.clone(), None),
    };

    let mut breaking = conventional
        .as_ref()
        .is_some_and(|p| p.breaking)
        || commit
            .body
            .lines()
            .any(|l| l.starts_with("BREAKING CHANGE:"));

    let breaking_description = extract_breaking_footer(&commit.body);
    if breaking_description.is_some() {
        breaking = true;
    }

    // The scope is kept separate so the renderer can group commits by it;
    // subject is just the human-readable description.
    let subject = subject.trim_end_matches('.').to_string();

    Some(ParsedCommit {
        entry,
        subject,
        breaking,
        breaking_description,
        scope,
    })
}

struct Parsed {
    r#type: String,
    scope: Option<String>,
    breaking: bool,
    subject: String,
}

fn parse_conventional(subject: &str) -> Option<Parsed> {
    // Format: type[(scope)][!]: subject — the bang comes before the colon.
    let colon = subject.find(": ")?;
    let header = &subject[..colon];
    let (header, breaking) = match header.strip_suffix('!') {
        Some(h) => (h, true),
        None => (header, false),
    };

    let (r#type, scope) = match header.find('(') {
        Some(open) => {
            if open == 0 || !header.ends_with(')') {
                return None;
            }
            let scope_part = &header[open + 1..header.len() - 1];
            let scope = if scope_part.is_empty() {
                None
            } else {
                Some(scope_part.to_string())
            };
            (header[..open].to_string(), scope)
        }
        None => (header.to_string(), None),
    };

    if r#type.is_empty()
        || !r#type
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return None;
    }

    let desc = subject[colon + 2..].trim().to_string();
    if desc.is_empty() {
        return None;
    }

    Some(Parsed {
        r#type,
        scope,
        breaking,
        subject: desc,
    })
}

fn extract_breaking_footer(body: &str) -> Option<String> {
    for line in body.lines() {
        if let Some(desc) = line.strip_prefix("BREAKING CHANGE: ") {
            return Some(desc.trim().to_string());
        }
        if line.starts_with("BREAKING CHANGE:") {
            // Description may start on the same line without a space.
            let desc = line.trim_start_matches("BREAKING CHANGE:").trim();
            if !desc.is_empty() {
                return Some(desc.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(subject: &str, body: &str) -> CommitInfo {
        CommitInfo {
            sha: "abc".into(),
            subject: subject.into(),
            body: body.into(),
        }
    }

    fn cfg() -> Config {
        Config::default()
    }

    #[test]
    fn feat_added() {
        let c = classify(&info("feat(api): add users endpoint", ""), &cfg()).unwrap();
        assert_eq!(c.entry, Entry::Added);
        assert_eq!(c.subject, "add users endpoint");
        assert_eq!(c.scope.as_deref(), Some("api"));
        assert!(!c.breaking);
    }

    #[test]
    fn fix_fixed() {
        let c = classify(&info("fix: handle empty body", ""), &cfg()).unwrap();
        assert_eq!(c.entry, Entry::Fixed);
        assert_eq!(c.subject, "handle empty body");
    }

    #[test]
    fn breaking_bang() {
        let c = classify(&info("feat!: change api", ""), &cfg()).unwrap();
        assert_eq!(c.entry, Entry::Added);
        assert!(c.breaking);
    }

    #[test]
    fn breaking_footer() {
        let c = classify(&info("feat: change api", "some body\nBREAKING CHANGE: config format changed"), &cfg()).unwrap();
        assert!(c.breaking);
        assert_eq!(c.breaking_description.as_deref(), Some("config format changed"));
    }

    #[test]
    fn chore_skipped_to_uncategorized() {
        let c = classify(&info("chore: bump deps", ""), &cfg());
        assert!(c.is_none(), "chores should be skipped");
    }

    #[test]
    fn non_conventional() {
        let c = classify(&info("weird message here", ""), &cfg()).unwrap();
        assert_eq!(c.entry, Entry::Uncategorized);
        assert_eq!(c.subject, "weird message here");
    }
}
