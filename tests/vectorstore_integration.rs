use anyhow::Result;
use cipher::{
    create_vectorstore_from_epub, query_vectorstore, rag_query,
    get_single_embedding, epub_to_markdown
};
use std::fs;
use std::path::Path;
use tokio;

const TEST_EPUB_PATH: &str = "testdata/pg35542.epub";
const TEST_STORE_PATH: &str = "test_vectorstore.json";

#[tokio::test]
async fn test_create_vectorstore_from_epub() -> Result<()> {
    // Clean up any existing test store
    let _ = fs::remove_file(TEST_STORE_PATH);
    
    // Create vectorstore from EPUB
    let _store = create_vectorstore_from_epub(TEST_EPUB_PATH, TEST_STORE_PATH).await?;
    
    // Verify the store was created
    assert!(Path::new(TEST_STORE_PATH).exists(), "Vectorstore file should be created");
    
    // Clean up
    let _ = fs::remove_file(TEST_STORE_PATH);
    Ok(())
}

#[tokio::test]
async fn test_epub_to_markdown_conversion() -> Result<()> {
    // Test EPUB to markdown conversion
    let chunks = epub_to_markdown(TEST_EPUB_PATH)?;
    
    assert!(!chunks.is_empty(), "Should convert EPUB to non-empty markdown chunks");
    assert!(chunks.iter().any(|chunk| chunk.len() > 10), "Should have substantial content in chunks");
    
    Ok(())
}

#[tokio::test]
#[ignore] // Requires Ollama to be running
async fn test_query_vectorstore() -> Result<()> {
    // Clean up any existing test store
    let _ = fs::remove_dir_all(TEST_STORE_PATH);
    
    // Create vectorstore
    let _store = create_vectorstore_from_epub(TEST_EPUB_PATH, TEST_STORE_PATH).await?;
    
    // Test query
    let results = query_vectorstore(TEST_STORE_PATH, "What is this book about?", 3).await?;
    
    assert!(!results.is_empty(), "Query should return results");
    assert!(results.len() <= 3, "Should return at most 3 results");
    
    // Verify result structure
    for (score, content) in &results {
        assert!(*score >= 0.0, "Score should be non-negative");
        assert!(!content.is_empty(), "Content should not be empty");
    }
    
    // Clean up
    let _ = fs::remove_dir_all(TEST_STORE_PATH);
    Ok(())
}

#[tokio::test]
#[ignore] // Requires Ollama to be running
async fn test_rag_query() -> Result<()> {
    // Clean up any existing test store
    let _ = fs::remove_dir_all(TEST_STORE_PATH);
    
    // Create vectorstore
    let _store = create_vectorstore_from_epub(TEST_EPUB_PATH, TEST_STORE_PATH).await?;
    
    // Test RAG query
    let answer = rag_query(TEST_STORE_PATH, "Who is the main character?", 3).await?;
    
    assert!(!answer.is_empty(), "RAG query should return a non-empty answer");
    assert!(answer.len() > 10, "Answer should be substantial");
    
    // Clean up
    let _ = fs::remove_dir_all(TEST_STORE_PATH);
    Ok(())
}

#[tokio::test]
#[ignore] // Requires Ollama to be running
async fn test_embedding_generation() -> Result<()> {
    let test_text = "This is a test sentence for embedding generation.";
    let embedding = get_single_embedding(test_text).await?;
    
    assert!(!embedding.is_empty(), "Embedding should not be empty");
    assert!(embedding.len() > 100, "Embedding should have reasonable dimension");
    
    // Test that different texts produce different embeddings
    let different_text = "A completely different sentence with different meaning.";
    let different_embedding = get_single_embedding(different_text).await?;
    
    assert_ne!(embedding, different_embedding, "Different texts should produce different embeddings");
    
    Ok(())
}

#[tokio::test]
#[ignore] // Requires Ollama to be running
async fn test_end_to_end_workflow() -> Result<()> {
    // Clean up any existing test store
    let _ = fs::remove_dir_all(TEST_STORE_PATH);
    
    // Step 1: Create vectorstore from EPUB
    println!("Creating vectorstore from EPUB...");
    let _store = create_vectorstore_from_epub(TEST_EPUB_PATH, TEST_STORE_PATH).await?;
    assert!(Path::new(TEST_STORE_PATH).exists(), "Vectorstore should be created");
    
    // Step 2: Test similarity search
    println!("Testing similarity search...");
    let search_results = query_vectorstore(TEST_STORE_PATH, "adventure story", 5).await?;
    assert!(!search_results.is_empty(), "Search should return results");
    
    // Step 3: Test RAG query
    println!("Testing RAG query...");
    let rag_answer = rag_query(TEST_STORE_PATH, "What genre is this book?", 3).await?;
    assert!(!rag_answer.is_empty(), "RAG should return an answer");
    
    println!("End-to-end test completed successfully!");
    println!("Search results: {} chunks found", search_results.len());
    println!("RAG answer length: {} characters", rag_answer.len());
    
    // Clean up
    let _ = fs::remove_dir_all(TEST_STORE_PATH);
    Ok(())
}