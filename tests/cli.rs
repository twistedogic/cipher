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
          // It also requires network access for the first run and a running Ollama instance.
fn test_epub_processing_and_lancedb_creation() -> Result<()> {
    let test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
    let executable_path = get_target_debug_path();

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

    let test_epub_path = test_data_dir.join("pg35542.epub");
    if !test_epub_path.exists() {
        panic!("Test EPUB file not found at {:?}", test_epub_path);
    }

    let temp_db_dir = tempdir().context("Failed to create temp dir for LanceDB")?;
    let temp_db_path = temp_db_dir.path().to_path_buf();
    let table_name = "test_epub_chunks_cli";
    let ollama_model_name = "nomic-embed-text";
    let ollama_embedding_dim = "768";

    println!("Testing with EPUB: {:?}", test_epub_path);
    println!("Using temp LanceDB path: {:?}", temp_db_path);
    println!(
        "Using Ollama model: {} (dim: {})",
        ollama_model_name, ollama_embedding_dim
    );

    let mut cmd = Command::new(&executable_path);
    cmd.arg("ingest") // Use the "ingest" subcommand
        .arg("--epub-path")
        .arg(&test_epub_path)
        .arg("--db-path")
        .arg(&temp_db_path)
        .arg("--table-name")
        .arg(table_name)
        .arg("--ollama-embedding-model") // Corrected argument name
        .arg(ollama_model_name)
        .arg("--embedding-dim")
        .arg(ollama_embedding_dim);

    // Assuming default ollama_url is http://localhost:11434, suitable for most local test setups.
    // If a different URL is needed: .arg("--ollama-url").arg("http://my-test-ollama:11434")

    let output = cmd.output().expect("Failed to execute command");

    if !output.stdout.is_empty() {
        println!("Stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        // eprintln!("Stderr:\n{}", String::from_utf8_lossy(&output.stderr));
    }

    assert!(
        output.status.success(),
        "Command failed. Stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

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

    println!("LanceDB table directory found at {:?}", table_data_path);
    Ok(())
}

#[test]
#[ignore] // Requires pre-ingested DB and running Ollama with embedding & generation models.
fn test_rag_query_command() -> Result<()> {
    let executable_path = get_target_debug_path();
    let temp_db_dir = tempdir().context("Failed to create temp dir for LanceDB")?;
    let temp_db_path = temp_db_dir.path().to_path_buf();

    let test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
    let test_epub_path = test_data_dir.join("pg35542.epub"); // Use a small, known EPUB

    let ingest_table_name = "rag_test_ingest";
    let ollama_embedding_model = "nomic-embed-text"; // Must match what's available in Ollama
    let ollama_embedding_dim = "768";
    let ollama_generation_model = "phi3:mini"; // A small model for faster testing, ensure it's pulled in Ollama

    // Step 1: Ingest data first to have something to query
    println!("Ingesting data for RAG test...");
    let mut ingest_cmd = Command::new(&executable_path);
    ingest_cmd
        .arg("ingest")
        .arg("--epub-path")
        .arg(&test_epub_path)
        .arg("--db-path")
        .arg(&temp_db_path)
        .arg("--table-name")
        .arg(ingest_table_name)
        .arg("--ollama-embedding-model")
        .arg(ollama_embedding_model)
        .arg("--embedding-dim")
        .arg(ollama_embedding_dim);
    // .arg("--ollama-url").arg("http://localhost:11434"); // Default is usually fine

    let ingest_output = ingest_cmd
        .output()
        .expect("Ingest command failed to execute");
    if !ingest_output.status.success() {
        println!(
            "Ingest Stdout:\n{}",
            String::from_utf8_lossy(&ingest_output.stdout)
        );
        eprintln!(
            "Ingest Stderr:\n{}",
            String::from_utf8_lossy(&ingest_output.stderr)
        );
    }
    assert!(
        ingest_output.status.success(),
        "Ingestion step failed for RAG test."
    );
    println!("Ingestion complete.");

    // Step 2: Run the query command
    let user_query = "What is this document about?"; // A generic query
    println!("Running RAG query: '{}'", user_query);

    let mut query_cmd = Command::new(&executable_path);
    query_cmd
        .arg("query")
        .arg("--query")
        .arg(user_query)
        .arg("--db-path")
        .arg(&temp_db_path)
        .arg("--table-name")
        .arg(ingest_table_name)
        .arg("--ollama-embedding-model")
        .arg(ollama_embedding_model)
        .arg("--embedding-dim")
        .arg(ollama_embedding_dim)
        .arg("--ollama-generation-model")
        .arg(ollama_generation_model)
        .arg("--top-k")
        .arg("2"); // Retrieve 2 chunks for context

    let query_output = query_cmd.output().expect("Query command failed to execute");

    println!(
        "Query Stdout:\n{}",
        String::from_utf8_lossy(&query_output.stdout)
    );
    if !query_output.stderr.is_empty() {
        eprintln!(
            "Query Stderr:\n{}",
            String::from_utf8_lossy(&query_output.stderr)
        );
    }
    assert!(query_output.status.success(), "RAG query command failed.");

    // Basic check: Ensure stdout (which should contain LLM response) is not empty
    assert!(
        !query_output.stdout.is_empty(),
        "RAG query command produced no output to stdout."
    );

    println!("RAG query test completed successfully.");
    // temp_db_dir is cleaned up automatically
    Ok(())
}
