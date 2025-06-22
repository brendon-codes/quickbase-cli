use std::{
    fs,
    path::{Path, PathBuf},
};

use predicates::prelude::*;
use tempfile::TempDir;

struct TestRepo {
    _temp: TempDir,
    root: PathBuf,
    nested: PathBuf,
}

impl TestRepo {
    fn new() -> Self {
        let temp = TempDir::new().expect("temp repo");
        let root = temp.path().to_path_buf();
        fs::create_dir(root.join(".git")).expect("git marker");
        let nested = root.join("nested").join("workdir");
        fs::create_dir_all(&nested).expect("nested workdir");

        Self {
            _temp: temp,
            root,
            nested,
        }
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn nested(&self) -> &Path {
        &self.nested
    }

    fn command(&self) -> assert_cmd::Command {
        let mut command = assert_cmd::Command::cargo_bin("quickbase").expect("binary exists");
        command.current_dir(&self.nested);
        command
    }
}

#[test]
fn project_skill_files_exist_and_have_valid_frontmatter() {
    for (path, name) in [
        (
            ".codex/skills/quickbase-api/SKILL.md",
            "name: quickbase-api",
        ),
        (
            ".codex/skills/quickbase-cli/SKILL.md",
            "name: quickbase-cli",
        ),
    ] {
        let contents = fs::read_to_string(path).expect("skill file exists");
        assert!(contents.starts_with("---\n"), "{path} has frontmatter");
        assert!(contents.contains(name), "{path} contains {name}");
        assert!(
            contents.contains("description:"),
            "{path} has a description"
        );
    }
}

#[test]
fn agents_mentions_project_skill_triggers() {
    let contents = fs::read_to_string("AGENTS.md").expect("AGENTS.md exists");

    assert!(contents.contains("Use `$quickbase-api`"));
    assert!(contents.contains("Use `$quickbase-cli`"));
}

#[test]
fn local_codex_copies_both_project_skills_into_current_project() {
    let repo = TestRepo::new();

    repo.command()
        .args(["util", "make-skill", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"location\": \"local\""))
        .stdout(predicate::str::contains("\"agent\": \"codex\""))
        .stdout(predicate::str::contains("quickbase-api"))
        .stdout(predicate::str::contains("quickbase-cli"));

    assert_skill_tree(repo.root().join(".codex").join("skills").as_path());
    assert!(
        !repo.nested().join(".codex").exists(),
        "skills are written relative to the repo root"
    );
}

#[test]
fn local_claude_copies_both_project_skills_into_current_project() {
    let repo = TestRepo::new();

    repo.command()
        .args(["util", "make-skill", "claude"])
        .assert()
        .success();

    assert_skill_tree(repo.root().join(".claude").join("skills").as_path());
}

#[test]
fn make_skill_replaces_existing_skill_directories() {
    let repo = TestRepo::new();
    let existing = repo
        .root()
        .join(".codex")
        .join("skills")
        .join("quickbase-api");
    fs::create_dir_all(&existing).expect("existing skill dir");
    fs::write(existing.join("sentinel.txt"), "stale").expect("sentinel");

    repo.command()
        .args(["util", "make-skill", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"agent\": \"codex\""));

    assert!(
        !existing.join("sentinel.txt").exists(),
        "stale files are removed during replacement"
    );
    assert_skill_tree(repo.root().join(".codex").join("skills").as_path());
}

#[test]
fn removed_make_skill_location_args_fail() {
    let repo = TestRepo::new();

    for args in [
        ["util", "make-skill", "local", "codex"],
        ["util", "make-skill", "global", "codex"],
        ["util", "make-skill", "local", "claude"],
        ["util", "make-skill", "global", "claude"],
    ] {
        repo.command()
            .args(args)
            .assert()
            .failure()
            .stderr(predicate::str::contains("unrecognized subcommand"));
    }
}

fn assert_skill_tree(root: &Path) {
    for skill in ["quickbase-api", "quickbase-cli"] {
        let skill_dir = root.join(skill);
        let skill_file = skill_dir.join("SKILL.md");
        assert!(skill_file.exists(), "{} exists", skill_file.display());

        let contents = fs::read_to_string(skill_file).expect("skill contents");
        assert!(contents.starts_with("---\n"));
        assert!(contents.contains(&format!("name: {skill}")));
    }

    assert!(
        root.join("quickbase-api/references/operation-index.md")
            .exists()
    );
    assert!(
        root.join("quickbase-api/references/argument-model.md")
            .exists()
    );
    assert!(
        root.join("quickbase-api/references/quickbase-rest-api.yaml")
            .exists()
    );
    assert!(root.join("quickbase-cli/references/commands.md").exists());
    assert!(root.join("quickbase-cli/references/config.md").exists());
}
