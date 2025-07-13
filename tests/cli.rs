use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("cipher").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn test_cli_epub_to_markdown() {
    let mut cmd = Command::cargo_bin("cipher").unwrap();
    cmd.arg("testdata/pg35542.epub");
    cmd.assert()
        .success();
}
