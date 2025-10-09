use assert_cmd::Command;
use git2::{Repository, Signature};
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
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
        .stdout(predicate::str::contains("Template"));

    // Verify workspace was created
    assert!(temp.path().join("dws").exists());
    let profile_root = temp.path().join("dws/profiles/default");
    assert!(profile_root.exists());
    assert!(profile_root.join("README.md").exists());
    assert!(profile_root.join("config/zsh/.zshrc").exists());
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
        .stdout(predicate::str::contains("Shell: bash"));

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
        .stdout(predicate::str::contains("Shell: fish"));

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
        .stdout(predicate::str::contains("* work (active)"));
}
