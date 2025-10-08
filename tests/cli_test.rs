use assert_cmd::Command;

#[test]
fn test_init_with_auto_detect_shell() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("init")
        .arg("ascarter/dotfiles")
        .assert()
        .success();
}

#[test]
fn test_init_with_explicit_shell() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("init")
        .arg("--shell")
        .arg("bash")
        .arg("ascarter/dotfiles")
        .assert()
        .success();
}

#[test]
fn test_init_no_repo() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("init").assert().success();
}

#[test]
fn test_init_shell_flag_short() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("init")
        .arg("-s")
        .arg("fish")
        .assert()
        .success();
}

#[test]
fn test_init_with_force() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("init")
        .arg("--force")
        .arg("ascarter/dotfiles")
        .assert()
        .success();
}

#[test]
fn test_help_output() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("--help").assert().success();
}

#[test]
fn test_init_help() {
    let mut cmd = Command::cargo_bin("devws").unwrap();
    cmd.arg("init")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("auto-detects from $SHELL"));
}
