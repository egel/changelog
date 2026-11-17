//! Git plumbing: resolving refs and collecting commits between references.

use anyhow::{anyhow, Context, Result};
use git2::{Repository, Sort};
use std::path::Path;

pub struct CommitInfo {
    /// Full commit SHA (used later for link references).
    #[allow(dead_code)]
    pub sha: String,
    /// Subject line (first line of the commit message).
    pub subject: String,
    /// Body (everything after the subject, trimmed).
    pub body: String,
}

/// Collect commits between two references, newest first.
pub fn commits_between(repo_path: &Path, from: Option<&str>, to: Option<&str>) -> Result<Vec<CommitInfo>> {
    let repo = Repository::open(repo_path)
        .with_context(|| format!("failed to open git repository at {}", repo_path.display()))?;

    let start = match from {
        Some(ref_name) => resolve(&repo, ref_name)?,
        None => match latest_tag(&repo)? {
            Some(obj) => Some(obj),
            None => {
                anyhow::bail!("no tags found in repository; use --from <ref> to specify a start point")
            }
        },
    };
    let end = match to {
        Some(ref_name) => resolve(&repo, ref_name)?,
        None => Some(head(&repo)?),
    };

    // Range is (start..end]: include `end`, exclude `start`.
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(Sort::TIME | Sort::TOPOLOGICAL)?;
    revwalk.push(end.unwrap().id())?;
    if let Some(start_id) = start.map(|o| o.id()) {
        // hide commits reachable from the start reference
        revwalk.hide(start_id)?;
    }

    let mut commits = Vec::new();
    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        if commit.parent_count() > 1 {
            continue; // skip merge commits
        }
        let message = commit.message().unwrap_or_default();
        let (subject, body) = split_message(message);
        commits.push(CommitInfo {
            sha: oid.to_string(),
            subject,
            body,
        });
    }
    Ok(commits)
}

fn split_message(message: &str) -> (String, String) {
    let mut lines = message.lines();
    let subject = lines.next().unwrap_or_default().trim().to_string();
    let body = lines
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    (subject, body)
}

fn head(repo: &Repository) -> Result<git2::Object<'_>> {
    // Note: repo.head() errors on an unborn branch, which is a good failure mode.
    let head = repo.head().context("failed to resolve HEAD")?;
    let target = head
        .target()
        .ok_or_else(|| anyhow!("HEAD is not a direct reference"))?;
    repo.find_object(target, Some(git2::ObjectType::Any))
        .context("failed to resolve HEAD")
}

/// Resolve a reference name (tag, branch, short SHA) to an object.
fn resolve<'r>(repo: &'r Repository, ref_name: &str) -> Result<Option<git2::Object<'r>>> {
    // Full tag names like "refs/tags/v1.0.0" work via revparse_single directly.
    match repo.revparse_single(ref_name) {
        Ok(obj) => Ok(Some(obj)),
        Err(_) => Err(anyhow!("failed to resolve reference '{}'", ref_name)),
    }
}

/// Find the latest tag reachable from HEAD, sorted by semver when possible,
/// falling back to tagger/commit time.
pub fn latest_tag(repo: &Repository) -> Result<Option<git2::Object<'_>>> {
    let tag_names = repo.tag_names(None)?;
    if tag_names.is_empty() {
        return Ok(None);
    }

    let mut best: Option<(semver::Version, git2::Oid)> = None;
    let mut newest_time: Option<(i64, git2::Oid)> = None;

    for name in tag_names.iter() {
        let name = name.unwrap(); // tags are valid UTF-8 in practice
        let reference = repo.find_reference(&format!("refs/tags/{}", name))?;
        let target = reference.peel_to_commit()?;

        if let Ok(version) = semver::Version::parse(name.trim_start_matches('v')) {
            if best.as_ref().is_none_or(|(v, _)| version > *v) {
                best = Some((version, target.id()));
            }
        } else {
            let time = target.time().seconds();
            if newest_time.is_none_or(|(t, _)| time > t) {
                newest_time = Some((time, target.id()));
            }
        }
    }

    let oid = match (best, newest_time) {
        (Some((_, oid)), _) => Some(oid),
        (None, Some((_, oid))) => Some(oid),
        (None, None) => None,
    };
    match oid {
        Some(oid) => repo
            .find_object(oid, Some(git2::ObjectType::Any))
            .map(Some)
            .map_err(Into::into),
        None => Ok(None),
    }
}
