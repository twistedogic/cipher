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
fn test_cli_convert_command() {
    let mut cmd = Command::cargo_bin("cipher").unwrap();
    cmd.args(&["convert", "testdata/pg35542.epub"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Generated"));
}

#[test]
fn test_cli_index_command() {
    let mut cmd = Command::cargo_bin("cipher").unwrap();
    cmd.args(&["index", "testdata/pg35542.epub", "--output", "test_cli_store.json"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Vectorstore created"));
    
    // Clean up
    std::fs::remove_file("test_cli_store.json").ok();
}
