use assert_cmd::Command;
use predicates::prelude::*;

fn shell() -> Command {
    Command::cargo_bin("codecrafters-shell").expect("binary built")
}

#[test]
fn echo_prints_arguments() {
    shell()
        .write_stdin("echo hello world\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello world"));
}

#[test]
fn variables_expand() {
    shell()
        .write_stdin("X=shellvar\necho $X\n")
        .assert()
        .stdout(predicate::str::contains("shellvar"));
}

#[test]
fn single_quotes_block_expansion() {
    shell()
        .write_stdin("X=5\necho '$X'\n")
        .assert()
        .stdout(predicate::str::contains("$X"));
}

#[test]
fn last_status_is_available() {
    shell()
        .write_stdin("false\necho status:$?\n")
        .assert()
        .stdout(predicate::str::contains("status:1"));
}

#[test]
fn command_not_found_sets_127() {
    shell()
        .write_stdin("definitely_not_a_command_xyz\necho code:$?\n")
        .assert()
        .stderr(predicate::str::contains("command not found"))
        .stdout(predicate::str::contains("code:127"));
}

#[test]
fn or_runs_on_failure() {
    shell()
        .write_stdin("false || echo recovered\n")
        .assert()
        .stdout(predicate::str::contains("recovered"));
}

#[test]
fn and_skips_on_failure() {
    shell()
        .write_stdin("false && echo never\n")
        .assert()
        .stdout(predicate::str::contains("never").not());
}

#[test]
fn pipeline_passes_output() {
    shell()
        .write_stdin("echo piped | cat\n")
        .assert()
        .stdout(predicate::str::contains("piped"));
}

#[test]
fn export_reaches_child_processes() {
    shell()
        .write_stdin("export RUSTY_FOO=bar\nsh -c 'echo $RUSTY_FOO'\n")
        .assert()
        .stdout(predicate::str::contains("bar"));
}

#[test]
fn exit_status_is_passed_through() {
    shell()
        .write_stdin("exit 3\n")
        .assert()
        .code(3);
}