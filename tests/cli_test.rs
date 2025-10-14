use assert_cmd::Command;
use git2::{Repository, Signature};
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
#[serial]
fn test_init_creates_template() {
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("dws").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .env("SHELL", "/bin/zsh")
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("template profile"));

    // Verify workspace was created
    assert!(temp.path().join("dws").exists());
    let profile_root = temp.path().join("dws/profiles/default");
    assert!(profile_root.exists());
    assert!(profile_root.join("README.md").exists());
    assert!(profile_root.join("config/zsh/.zshrc").exists());
    assert!(profile_root.join("dws.toml").exists());
    assert!(temp.path().join("dws/config.toml").exists());
}

#[test]
#[serial]
fn test_init_with_explicit_shell() {
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("dws").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .arg("init")
        .arg("--shell")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "workspace initialized (profile 'default', shell bash)",
        ));

    assert!(temp.path().join("dws").exists());
}

#[test]
#[serial]
fn test_init_shell_flag_short() {
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("dws").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .arg("init")
        .arg("-s")
        .arg("fish")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "workspace initialized (profile 'default', shell fish)",
        ));

    assert!(temp.path().join("dws").exists());
}

#[test]
#[serial]
fn test_init_without_shell_env_fails() {
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("dws").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .env_remove("SHELL")
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "SHELL environment variable not set",
        ));
}

#[test]
#[serial]
fn test_init_existing_workspace() {
    let temp = TempDir::new().unwrap();

    // First init
    let mut cmd = Command::cargo_bin("dws").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .env("SHELL", "/bin/zsh")
        .arg("init")
        .assert()
        .success();

    // Second init with different shell
    let mut cmd = Command::cargo_bin("dws").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .arg("init")
        .arg("-s")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("Updating template profile"));
}

#[test]
fn test_help_output() {
    let mut cmd = Command::cargo_bin("dws").unwrap();
    cmd.arg("--help").assert().success();
}

#[test]
fn test_init_help() {
    let mut cmd = Command::cargo_bin("dws").unwrap();
    cmd.arg("init")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("auto-detects from $SHELL"));
}

#[test]
#[serial]
fn test_clone_and_use_profile() {
    let temp = TempDir::new().unwrap();

    // Prepare source repository
    let source_temp = TempDir::new().unwrap();
    let source_repo_path = source_temp.path().join("work-repo");
    fs::create_dir_all(&source_repo_path).unwrap();
    let repo = Repository::init(&source_repo_path).unwrap();
    fs::write(source_repo_path.join("README.md"), "work profile").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("README.md")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    // Initialize default profile
    Command::cargo_bin("dws")
        .unwrap()
        .env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .env("SHELL", "/bin/zsh")
        .arg("init")
        .assert()
        .success();

    // Clone new profile
    Command::cargo_bin("dws")
        .unwrap()
        .env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .arg("clone")
        .arg(source_repo_path.to_string_lossy().to_string())
        .arg("--profile")
        .arg("work")
        .assert()
        .success();

    assert!(temp.path().join("dws/profiles/work/README.md").exists());

    // Activate cloned profile
    Command::cargo_bin("dws")
        .unwrap()
        .env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .arg("use")
        .arg("work")
        .assert()
        .success();

    let config_contents = fs::read_to_string(temp.path().join("dws/config.toml")).unwrap();
    assert!(config_contents.contains("active_profile = \"work\""));

    // List profiles and ensure active indicated
    Command::cargo_bin("dws")
        .unwrap()
        .env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .arg("profiles")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Active work")
                .and(predicate::str::contains("Profile default")),
        );
}

#[test]
#[serial]
fn test_use_missing_profile_fails() {
    let temp = TempDir::new().unwrap();

    Command::cargo_bin("dws")
        .unwrap()
        .env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .env("SHELL", "/bin/zsh")
        .arg("init")
        .assert()
        .success();

    Command::cargo_bin("dws")
        .unwrap()
        .env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .arg("use")
        .arg("missing")
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let entry_path = entry.path();
        let dest_path = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir_all(&entry_path, &dest_path);
        } else {
            fs::copy(entry_path, dest_path).unwrap();
        }
    }
}

fn init_template_repo(path: &Path) {
    let template_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/profile");
    copy_dir_all(&template_dir, path);

    let repo = Repository::init(path).unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();
}

#[test]
#[serial]
fn test_reset_requires_clean_repo_without_force() {
    let temp = TempDir::new().unwrap();
    let source_temp = TempDir::new().unwrap();
    let source_repo_path = source_temp.path().join("work-profile");
    init_template_repo(&source_repo_path);

    Command::cargo_bin("dws")
        .unwrap()
        .env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .env("SHELL", "/bin/zsh")
        .arg("init")
        .arg(source_repo_path.to_string_lossy().to_string())
        .arg("--profile")
        .arg("work")
        .assert()
        .success();

    let profile_root = temp.path().join("dws/profiles/work");
    fs::write(profile_root.join("README.md"), "modified readme").unwrap();

    Command::cargo_bin("dws")
        .unwrap()
        .env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .arg("reset")
        .assert()
        .failure()
        .stderr(predicate::str::contains("uncommitted changes"));
}

#[test]
#[serial]
fn test_reset_force_discards_changes() {
    let temp = TempDir::new().unwrap();
    let source_temp = TempDir::new().unwrap();
    let source_repo_path = source_temp.path().join("force-profile");
    init_template_repo(&source_repo_path);

    Command::cargo_bin("dws")
        .unwrap()
        .env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .env("SHELL", "/bin/zsh")
        .arg("init")
        .arg(source_repo_path.to_string_lossy().to_string())
        .arg("--profile")
        .arg("force")
        .assert()
        .success();

    let profile_root = temp.path().join("dws/profiles/force");
    fs::write(profile_root.join("README.md"), "modified content").unwrap();
    fs::write(profile_root.join("extra.txt"), "temporary file").unwrap();

    Command::cargo_bin("dws")
        .unwrap()
        .env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .arg("reset")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("workspace reset complete."));

    assert!(!profile_root.join("extra.txt").exists());
    let readme = fs::read_to_string(profile_root.join("README.md")).unwrap();
    assert_ne!(readme.trim(), "modified content");
    assert!(temp.path().join("state/dws/dws.lock").exists());
}

#[test]
#[serial]
fn test_reset_confirmation_can_abort() {
    let temp = TempDir::new().unwrap();
    let source_temp = TempDir::new().unwrap();
    let source_repo_path = source_temp.path().join("confirm-profile");
    init_template_repo(&source_repo_path);

    Command::cargo_bin("dws")
        .unwrap()
        .env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .env("SHELL", "/bin/zsh")
        .arg("init")
        .arg(source_repo_path.to_string_lossy().to_string())
        .arg("--profile")
        .arg("confirm")
        .assert()
        .success();

    Command::cargo_bin("dws")
        .unwrap()
        .env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .env("HOME", temp.path())
        .arg("reset")
        .write_stdin("n\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Reset cancelled."));
}
