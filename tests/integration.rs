//! End-to-end tests: fixture git repos → CLI behavior, snapshotted with insta.

use git2::{Repository, Signature, Time};
use std::path::{Path, PathBuf};

/// Build a fixture repository with the given commits and optional tag.
struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    /// commits: (message, tag-after-commit-or-None)
    fn new(name: &str, commits: &[(&str, Option<&str>)]) -> Fixture {
        let dir = std::env::temp_dir().join(format!("changelog-test-{}", name));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = Repository::init(&dir).unwrap();
        let sig = Signature::new("T", "t@t.co", &Time::new(1_700_000_000, 0)).unwrap();

        let mut index = repo.index().unwrap();
        for (i, (message, tag)) in commits.iter().enumerate() {
            let file = format!("f{}.txt", i);
            std::fs::write(dir.join(&file), format!("{}\n", i)).unwrap();
            index.add_path(Path::new(&file)).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            let parent = if repo.head().is_ok() {
                vec![repo.head().unwrap().peel_to_commit().unwrap()]
            } else {
                vec![]
            };
            let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
                .unwrap();
            if let Some(tag) = tag {
                let head = repo.head().unwrap().peel_to_commit().unwrap();
                repo.tag(tag, head.as_object(), &sig, tag, false).unwrap();
            }
        }
        Fixture { dir }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn run(args: &[&str], dir: &Path) -> (String, String, bool) {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_changelog"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

#[test]
fn stdout_generation_snapshot() {
    let fx = Fixture::new(
        "stdout",
        &[
            ("feat(api): add users endpoint", None),
            ("fix: handle empty body", None),
            ("feat!: rename config", Some("v0.1.0")),
            ("feat: add export button", None),
        ],
    );
    let (stdout, _, ok) = run(&[], &fx.dir);
    assert!(ok);
    insta::assert_yaml_snapshot!(stdout);
}

#[test]
fn init_with_history_snapshot() {
    let fx = Fixture::new(
        "init-history",
        &[
            ("feat: initial", Some("v0.1.0")),
            ("fix: bugfix", None),
            ("feat!: breaking change", None),
            ("chore: bump deps", None),
        ],
    );
    let (_, stderr, ok) = run(&["init", "--with-history"], &fx.dir);
    assert!(ok, "init failed: {}", stderr);
    let content = std::fs::read_to_string(fx.dir.join("CHANGELOG.md")).unwrap();
    insta::assert_yaml_snapshot!(content);
}

#[test]
fn init_refuses_overwrite() {
    let fx = Fixture::new(
        "init-exists",
        &[("feat: initial", Some("v0.1.0"))],
    );
    run(&["init"], &fx.dir);
    let (stdout, stderr, ok) = run(&["init"], &fx.dir);
    assert!(!ok);
    assert!(stdout.is_empty());
    assert!(stderr.contains("already exists"));
}

#[test]
fn in_place_splices_and_keeps_rest() {
    let fx = Fixture::new(
        "in-place",
        &[
            ("feat: initial", Some("v0.1.0")),
            ("fix: bugfix", None),
        ],
    );
    run(&["init"], &fx.dir);
    let (stdout, stderr, ok) = run(&["--in-place"], &fx.dir);
    assert!(ok, "in-place failed: {}", stderr);
    assert!(stdout.is_empty());
    let content = std::fs::read_to_string(fx.dir.join("CHANGELOG.md")).unwrap();
    assert!(content.contains("### Fixed\n- bugfix"));
    assert!(content.contains("## [1.0.0]") || content.contains("[Unreleased]"));
    insta::assert_yaml_snapshot!(content);
}

#[test]
fn in_place_requires_init() {
    let fx = Fixture::new("in-place-no-file", &[("feat: initial", None)]);
    let (_, stderr, ok) = run(&["--in-place"], &fx.dir);
    assert!(!ok);
    assert!(stderr.contains("changelog init"));
}

#[test]
fn release_mode_snapshot() {
    let fx = Fixture::new(
        "release",
        &[
            ("feat: initial", Some("v0.1.0")),
            ("feat: new thing", None),
            ("fix: bugfix", None),
        ],
    );
    run(&["init"], &fx.dir);
    // Fill [Unreleased] first, as users would before releasing.
    let (_, stderr, ok) = run(&["--in-place"], &fx.dir);
    assert!(ok, "in-place failed: {}", stderr);
    let (_, stderr, ok) = run(&["--in-place", "--release", "0.2.0"], &fx.dir);
    assert!(ok, "release failed: {}", stderr);
    let content = std::fs::read_to_string(fx.dir.join("CHANGELOG.md")).unwrap();
    insta::assert_yaml_snapshot!(content);
}

#[test]
fn suggest_semver() {
    let fx = Fixture::new(
        "suggest",
        &[
            ("feat: initial", Some("v0.1.0")),
            ("fix: bugfix", None),
        ],
    );
    let (stdout, _, ok) = run(&["suggest"], &fx.dir);
    assert!(ok);
    assert_eq!(stdout.trim(), "0.1.1");
}

#[test]
fn config_custom_types_and_exclude() {
    let fx = Fixture::new(
        "config",
        &[
            ("chore: add config", Some("v0.1.0")),
            ("exp: experimental feature", None),
            ("fix: bugfix", None),
            ("refactor: internal refactor of core", None),
        ],
    );
    std::fs::write(
        fx.dir.join(".changelogrc.toml"),
        "exclude = [\"internal refactor\"]\n\n[types]\n\"exp\" = \"Added\"\n",
    )
    .unwrap();
    let (stdout, _, ok) = run(&[], &fx.dir);
    assert!(ok);
    assert!(stdout.contains("- experimental feature"));
    assert!(stdout.contains("- bugfix"));
    assert!(!stdout.contains("internal refactor"));
    assert!(!stdout.contains("Uncategorized"));
}

#[test]
fn no_tags_helpful_error() {
    let fx = Fixture::new("no-tags", &[("feat: initial", None)]);
    let (_, stderr, ok) = run(&[], &fx.dir);
    assert!(!ok);
    assert!(stderr.contains("no tags found"));
}

#[test]
fn all_history_snapshot() {
    let fx = Fixture::new(
        "all-history",
        &[
            ("feat: initial", None),
            ("fix: first bug", Some("v0.1.0")),
            ("feat: second feature", Some("v0.2.0")),
            ("fix: second bug", None),
            ("feat: unreleased thing", None),
        ],
    );
    let (stdout, stderr, ok) = run(&["--all"], &fx.dir);
    assert!(ok, "--all failed: {}", stderr);
    insta::assert_yaml_snapshot!(stdout);
}
