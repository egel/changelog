//! Optional `.changelogrc.toml` configuration: custom type mappings and
//! subject exclusions.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// Built-in type → section mapping (Conventional Commits subset).
pub const DEFAULT_TYPE_MAP: &[(&str, &str)] = &[
    ("feat", "Added"),
    ("fix", "Fixed"),
    ("perf", "Changed"),
    ("refactor", "Changed"),
    ("revert", "Removed"),
    ("remove", "Removed"),
    ("deprecated", "Deprecated"),
    ("security", "Security"),
];

/// Commit types never shown in the changelog (unless overridden in config).
pub const DEFAULT_SKIP_TYPES: &[&str] = &["chore", "docs", "style", "ci", "test", "tests", "build"];

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Custom Conventional Commit type → section mapping.
    /// Section is one of Added/Changed/Deprecated/Removed/Fixed/Security/
    /// Uncategorized/Skip (case-insensitive).
    #[serde(default)]
    types: std::collections::BTreeMap<String, String>,

    /// Commit subjects matching any of these substrings are excluded.
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl Config {
    /// Load `.changelogrc.toml` from the repository root, if present.
    pub fn load(repo_path: &Path) -> Result<Config> {
        let path = repo_path.join(".changelogrc.toml");
        if !path.exists() {
            return Ok(Config::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))
    }

    /// Resolve a commit type to a changelog section.
    /// Returns `None` when the type should be skipped.
    pub fn section_for_type(&self, commit_type: &str) -> Option<&'static str> {
        if let Some(section) = self.types.get(commit_type) {
            return match section.to_ascii_lowercase().as_str() {
                "added" => Some("Added"),
                "changed" => Some("Changed"),
                "deprecated" => Some("Deprecated"),
                "removed" => Some("Removed"),
                "fixed" => Some("Fixed"),
                "security" => Some("Security"),
                "uncategorized" => Some("Uncategorized"),
                "skip" | "skipped" => None,
                other => {
                    // Unknown section name: fall back to Uncategorized.
                    eprintln!(
                        "warning: unknown section '{}' in .changelogrc.toml for type '{}'; treating as Uncategorized",
                        other, commit_type
                    );
                    Some("Uncategorized")
                }
            };
        }
        for (t, section) in DEFAULT_TYPE_MAP {
            if *t == commit_type {
                return Some(section);
            }
        }
        if DEFAULT_SKIP_TYPES.contains(&commit_type) {
            return None;
        }
        Some("Uncategorized")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let cfg = Config::default();
        assert_eq!(cfg.section_for_type("feat"), Some("Added"));
        assert_eq!(cfg.section_for_type("fix"), Some("Fixed"));
        assert_eq!(cfg.section_for_type("chore"), None);
        assert_eq!(cfg.section_for_type("weird"), Some("Uncategorized"));
    }

    #[test]
    fn custom_mapping() {
        let cfg: Config = toml::from_str(
            r#"
[types]
"feat" = "Changed"
"exp" = "Skip"
"ops" = "Added"
"#,
        )
        .unwrap();
        assert_eq!(cfg.section_for_type("feat"), Some("Changed"));
        assert_eq!(cfg.section_for_type("exp"), None);
        assert_eq!(cfg.section_for_type("ops"), Some("Added"));
    }

    #[test]
    fn unknown_section_warns_and_falls_back() {
        let cfg: Config = toml::from_str("[types]\nfeat = \"WhatNow\"").unwrap();
        assert_eq!(cfg.section_for_type("feat"), Some("Uncategorized"));
    }
}
