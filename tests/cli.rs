use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir; // For creating a temporary directory for LanceDB

fn get_target_debug_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("cipher");
    path
}

#[test]
#[ignore] // This test can be slow due to model downloads and embedding generation.
          // It also requires network access for the first run.
fn test_epub_processing_and_lancedb_creation() -> Result<()> {
    let test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
    let executable_path = get_target_debug_path();

    // Ensure the executable exists (it should after a build if tests are run via `cargo test`)
    if !executable_path.exists() {
        println!(
            "Executable not found at {:?}, attempting to build...",
            executable_path
        );
        let build_status = Command::new("cargo")
            .arg("build")
            .current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
            .status()?;
        if !build_status.success() {
            panic!(
                "Failed to build the project before testing. Executable not found at {:?}.",
                executable_path
            );
        }
        println!("Build successful.");
    }

    if !test_data_dir.exists() {
        panic!("Test data directory not found at {:?}", test_data_dir);
    }

    // Use the smallest EPUB for faster testing
    let test_epub_path = test_data_dir.join("pg35542.epub"); // Or find the smallest one
    if !test_epub_path.exists() {
        panic!("Test EPUB file not found at {:?}", test_epub_path);
    }

    // Create a temporary directory for LanceDB output for this test run
    let temp_db_dir = tempdir().context("Failed to create temp dir for LanceDB")?;
    let temp_db_path = temp_db_dir.path().to_path_buf();
    let table_name = "test_epub_chunks_cli"; // Use a distinct table name for CLI test
    let ollama_model_name = "nomic-embed-text"; // Common model, ensure it's pulled in Ollama
    let ollama_embedding_dim = "768"; // Dimension for nomic-embed-text

    println!("Testing with EPUB: {:?}", test_epub_path);
    println!("Using temp LanceDB path: {:?}", temp_db_path);
    println!(
        "Using Ollama model: {} (dim: {})",
        ollama_model_name, ollama_embedding_dim
    );

    let mut cmd = Command::new(&executable_path);
    cmd.arg("--epub-path")
        .arg(&test_epub_path)
        .arg("--db-path")
        .arg(&temp_db_path)
        .arg("--table-name")
        .arg(table_name)
        .arg("--ollama-model")
        .arg(ollama_model_name)
        .arg("--embedding-dim")
        .arg(ollama_embedding_dim);

    // Optionally, set a specific Ollama URL if the default http://localhost:11434 is not desired for tests
    // .arg("--ollama-url").arg("http://my-test-ollama:11434")

    let output = cmd.output().expect("Failed to execute command");

    // Print stdout and stderr for debugging, especially if the test fails
    if !output.stdout.is_empty() {
        println!("Stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        // Stderr can be noisy with download progress, etc.
        // Only print if the command failed, or if verbosity is desired.
        // eprintln!("Stderr:\n{}", String::from_utf8_lossy(&output.stderr));
    }

    assert!(
        output.status.success(),
        "Command failed. Stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify that the LanceDB directory and table files were created
    // A LanceDB table usually creates a directory like `table_name.lance`
    let table_data_path = temp_db_path.join(format!("{}.lance", table_name));
    assert!(
        table_data_path.exists(),
        "LanceDB table directory not found at {:?}",
        table_data_path
    );
    assert!(
        table_data_path.is_dir(),
        "LanceDB table path is not a directory: {:?}",
        table_data_path
    );

    // Further checks could involve opening the DB with lancedb crate and verifying content,
    // but that adds more complexity to this CLI test. For now, existence is a good sign.
    println!("LanceDB table directory found at {:?}", table_data_path);

    // temp_db_dir will be automatically cleaned up when it goes out of scope.
    Ok(())
}
