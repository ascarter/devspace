use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("devspace").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Personal development environment manager"));
}

#[test]
fn test_version() {
    let mut cmd = Command::cargo_bin("devspace").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("devspace"));
}

#[test]
fn test_app_list() {
    let mut cmd = Command::cargo_bin("devspace").unwrap();
    cmd.arg("app")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"));
}

#[test]
fn test_config_status() {
    let mut cmd = Command::cargo_bin("devspace").unwrap();
    cmd.arg("config")
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"));
}

#[test]
fn test_profile_list() {
    let mut cmd = Command::cargo_bin("devspace").unwrap();
    cmd.arg("profile")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"));
}

#[test]
fn test_doctor() {
    let mut cmd = Command::cargo_bin("devspace").unwrap();
    cmd.arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"));
}

#[test]
fn test_status() {
    let mut cmd = Command::cargo_bin("devspace").unwrap();
    cmd.arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"));
}
