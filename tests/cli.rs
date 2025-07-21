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
#[ignore] // Requires Ollama to be running
fn test_cli_index_command() {
    let mut cmd = Command::cargo_bin("cipher").unwrap();
    cmd.args(&["index", "testdata/pg35542.epub", "--output", "test_cli_store.json"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Vectorstore created"));
    
    // Clean up
    std::fs::remove_file("test_cli_store.json").ok();
}

#[test]
#[ignore] // Requires Ollama to be running  
fn test_cli_search_command() {
    // First create a vectorstore
    let mut cmd = Command::cargo_bin("cipher").unwrap();
    cmd.args(&["index", "testdata/pg35542.epub", "--output", "test_cli_search.json"]);
    cmd.assert().success();
    
    // Then search it
    let mut cmd = Command::cargo_bin("cipher").unwrap();
    cmd.args(&["search", "--store-path", "test_cli_search.json", "adventure", "--top-k", "3"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Search Results"));
    
    // Clean up
    std::fs::remove_file("test_cli_search.json").ok();
}

#[test]
#[ignore] // Requires Ollama to be running
fn test_cli_rag_command() {
    // First create a vectorstore
    let mut cmd = Command::cargo_bin("cipher").unwrap();
    cmd.args(&["index", "testdata/pg35542.epub", "--output", "test_cli_rag.json"]);
    cmd.assert().success();
    
    // Then query it with RAG
    let mut cmd = Command::cargo_bin("cipher").unwrap();
    cmd.args(&["rag", "--store-path", "test_cli_rag.json", "What is this book about?", "--top-k", "3"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Answer:"));
    
    // Clean up
    std::fs::remove_file("test_cli_rag.json").ok();
}
