//! Entry point: parse CLI args and dispatch.

mod changelog;
mod cli;
mod commits;
mod git;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let repo_path = find_repo_root()?;

    match &cli.command {
        Some(cli::Command::Init { force, with_history }) => {
            changelog::run_init(&repo_path, *force, *with_history, &cli)
        }
        Some(cli::Command::Suggest) => changelog::run_suggest(&repo_path),
        None => {
            if cli.in_place {
                changelog::run_in_place(&cli, &repo_path)
            } else {
                changelog::run_generate(&cli, &repo_path)
            }
        }
    }
}

/// Walk up from the current directory to the git repository root.
fn find_repo_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let mut dir = cwd.as_path();
    loop {
        if dir.join(".git").exists() {
            return Ok(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => anyhow::bail!("not inside a git repository (no .git found from {} upwards)", cwd.display()),
        }
    }
}
