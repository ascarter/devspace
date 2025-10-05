use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Personal development environment manager",
        ));
}

#[test]
fn test_version() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("devws"));
}

#[test]
fn test_doctor() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"));
}

#[test]
fn test_status() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"));
}

#[test]
fn test_env_default_profile() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("env")
        .assert()
        .success()
        .stdout(predicate::str::contains("export PATH="))
        .stdout(predicate::str::contains("export MANPATH="))
        .stdout(predicate::str::contains("fpath="));
}

#[test]
fn test_env_with_profile_arg() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("env")
        .arg("myprofile")
        .assert()
        .success()
        .stdout(predicate::str::contains("/environments/myprofile/"));
}

#[test]
fn test_env_contains_correct_paths() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    let output = cmd.arg("env").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Verify PATH contains bin directory
    assert!(stdout.contains("/bin:$PATH"));

    // Verify MANPATH contains man directory
    assert!(stdout.contains("/share/man:"));

    // Verify fpath contains zsh site-functions (default is zsh)
    assert!(stdout.contains("/share/zsh/site-functions"));
}

#[test]
fn test_env_bash_shell() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("env")
        .arg("--shell")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("export PATH="))
        .stdout(predicate::str::contains("export MANPATH="))
        .stdout(predicate::str::contains("/bin:$PATH"));

    // Bash shouldn't have fpath
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("fpath"));
}

#[test]
fn test_env_fish_shell() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("env")
        .arg("--shell")
        .arg("fish")
        .assert()
        .success()
        .stdout(predicate::str::contains("set -gx PATH"))
        .stdout(predicate::str::contains("set -gx MANPATH"));

    // Fish shouldn't have export
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("export"));
}
