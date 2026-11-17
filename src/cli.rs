//! CLI argument definitions.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "changelog",
    version,
    about = "Generate Keep a Changelog formatted changelogs from git commits",
    long_about = "Arranges commits between tags into sections following the \
Keep a Changelog 1.0.0 format. Conventional Commits are parsed automatically; \
unrecognized commits are collected in an 'Uncategorized' section."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Start reference (tag, branch or commit SHA). Defaults to the latest tag.
    #[arg(long, short = 'F')]
    pub from: Option<String>,

    /// End reference (tag, branch or commit SHA). Defaults to HEAD.
    #[arg(long, short = 'T')]
    pub to: Option<String>,

    /// Merge the generated section into the existing CHANGELOG.md instead of
    /// printing it to stdout.
    #[arg(long)]
    pub in_place: bool,

    /// With --in-place: show what would change without writing the file.
    #[arg(long, requires = "in_place")]
    pub dry_run: bool,

    /// Release version: rename [Unreleased] to [x.y.z] - today, start a fresh
    /// [Unreleased] section.
    #[arg(long)]
    pub release: Option<String>,

    /// Generate the full history from all tags. (Later phase.)
    #[arg(long, conflicts_with_all = &["from", "to"])]
    pub all: bool,
}#[derive(Subcommand)]
pub enum Command {
    /// Create a fresh CHANGELOG.md scaffold following Keep a Changelog.
    Init {
        /// Overwrite an existing CHANGELOG.md.
        #[arg(long)]
        force: bool,

        /// Pre-populate the [Unreleased] section from tag..HEAD commits.
        #[arg(long)]
        with_history: bool,
    },

    /// Suggest the next semantic version based on commits since the latest tag.
    Suggest,
}
