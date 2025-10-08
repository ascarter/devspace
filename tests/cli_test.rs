use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use tempfile::TempDir;

#[test]
#[serial]
fn test_init_creates_template() {
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("dws").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("HOME", temp.path())
        .env("SHELL", "/bin/zsh")
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Template created"));

    // Verify workspace was created
    assert!(temp.path().join("dws").exists());
    assert!(temp.path().join("dws/README.md").exists());
    assert!(temp.path().join("dws/config/zsh/.zshrc").exists());
}

#[test]
#[serial]
fn test_init_with_explicit_shell() {
    let temp = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("dws").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
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
        .env("HOME", temp.path())
        .env("SHELL", "/bin/zsh")
        .arg("init")
        .assert()
        .success();

    // Second init with different shell
    let mut cmd = Command::cargo_bin("dws").unwrap();
    cmd.env("XDG_CONFIG_HOME", temp.path())
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("HOME", temp.path())
        .arg("init")
        .arg("-s")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("Using existing workspace"));
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
