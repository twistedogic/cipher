use anyhow::Result;
use cipher::{
    create_vectorstore_from_epub, query_vectorstore, rag_query, VectorStore,
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
    let store = create_vectorstore_from_epub(TEST_EPUB_PATH, TEST_STORE_PATH).await?;
    
    // Verify the store was created
    assert!(!store.chunks.is_empty(), "Vectorstore should contain chunks");
    assert!(Path::new(TEST_STORE_PATH).exists(), "Vectorstore file should be created");
    
    // Verify each chunk has valid data
    for chunk in &store.chunks {
        assert!(!chunk.id.is_empty(), "Chunk should have an ID");
        assert!(!chunk.content.trim().is_empty(), "Chunk should have content");
        assert!(!chunk.embedding.is_empty(), "Chunk should have embeddings");
        assert!(chunk.metadata.contains_key("source"), "Chunk should have source metadata");
        assert!(chunk.metadata.contains_key("chunk_index"), "Chunk should have chunk_index metadata");
    }
    
    println!("Created vectorstore with {} chunks", store.chunks.len());
    
    // Clean up
    let _ = fs::remove_file(TEST_STORE_PATH);
    
    Ok(())
}

#[tokio::test]
async fn test_load_and_query_vectorstore() -> Result<()> {
    // Clean up any existing test store
    let _ = fs::remove_file(TEST_STORE_PATH);
    
    // Create vectorstore
    let _store = create_vectorstore_from_epub(TEST_EPUB_PATH, TEST_STORE_PATH).await?;
    
    // Load the vectorstore from file
    let loaded_store = VectorStore::load_from_file(TEST_STORE_PATH)?;
    assert!(!loaded_store.chunks.is_empty(), "Loaded store should contain chunks");
    
    // Test querying the vectorstore
    let query = "main character";
    let results = query_vectorstore(&loaded_store, query, 3).await?;
    
    assert!(!results.is_empty(), "Query should return results");
    assert!(results.len() <= 3, "Should return at most 3 results");
    
    // Verify results have valid similarity scores and content
    for (score, content) in &results {
        assert!(*score >= 0.0 && *score <= 1.0, "Similarity score should be between 0 and 1");
        assert!(!content.trim().is_empty(), "Result content should not be empty");
    }
    
    // Results should be sorted by similarity (highest first)
    for i in 1..results.len() {
        assert!(results[i-1].0 >= results[i].0, "Results should be sorted by similarity score");
    }
    
    println!("Query '{}' returned {} results", query, results.len());
    for (i, (score, content)) in results.iter().enumerate() {
        println!("Result {}: Score {:.3}, Content preview: {}...", 
                 i+1, score, &content.chars().take(100).collect::<String>());
    }
    
    // Clean up
    let _ = fs::remove_file(TEST_STORE_PATH);
    
    Ok(())
}

#[tokio::test]
async fn test_rag_query_end_to_end() -> Result<()> {
    // Clean up any existing test store
    let _ = fs::remove_file(TEST_STORE_PATH);
    
    // Create vectorstore
    let store = create_vectorstore_from_epub(TEST_EPUB_PATH, TEST_STORE_PATH).await?;
    println!("Created vectorstore with {} chunks for RAG test", store.chunks.len());
    
    // Test RAG query
    let query = "Who is the main character?";
    let answer = rag_query(TEST_STORE_PATH, query, 3).await?;
    
    assert!(!answer.trim().is_empty(), "RAG query should return a non-empty answer");
    println!("Query: {}", query);
    println!("Answer: {}", answer);
    
    // Test another query
    let query2 = "What is the setting of the story?";
    let answer2 = rag_query(TEST_STORE_PATH, query2, 3).await?;
    
    assert!(!answer2.trim().is_empty(), "Second RAG query should return a non-empty answer");
    println!("Query: {}", query2);
    println!("Answer: {}", answer2);
    
    // Clean up
    let _ = fs::remove_file(TEST_STORE_PATH);
    
    Ok(())
}

#[tokio::test]
async fn test_similarity_search_accuracy() -> Result<()> {
    // Clean up any existing test store
    let _ = fs::remove_file(TEST_STORE_PATH);
    
    // Create vectorstore
    let store = create_vectorstore_from_epub(TEST_EPUB_PATH, TEST_STORE_PATH).await?;
    
    // Test that similar queries return similar results
    let query1 = "character";
    let query2 = "protagonist";
    
    let results1 = query_vectorstore(&store, query1, 5).await?;
    let results2 = query_vectorstore(&store, query2, 5).await?;
    
    assert!(!results1.is_empty(), "First query should return results");
    assert!(!results2.is_empty(), "Second query should return results");
    
    // Test that exact content match returns high similarity
    if let Some(first_chunk) = store.chunks.first() {
        let exact_query = &first_chunk.content[..std::cmp::min(100, first_chunk.content.len())];
        let exact_results = query_vectorstore(&store, exact_query, 1).await?;
        
        if !exact_results.is_empty() {
            assert!(exact_results[0].0 > 0.8, "Exact content match should have high similarity score");
        }
    }
    
    println!("Similarity search test completed successfully");
    
    // Clean up
    let _ = fs::remove_file(TEST_STORE_PATH);
    
    Ok(())
}

#[tokio::test]
async fn test_vectorstore_persistence() -> Result<()> {
    // Clean up any existing test store
    let _ = fs::remove_file(TEST_STORE_PATH);
    
    // Create and save vectorstore
    let original_store = create_vectorstore_from_epub(TEST_EPUB_PATH, TEST_STORE_PATH).await?;
    let original_chunk_count = original_store.chunks.len();
    
    // Load the vectorstore
    let loaded_store = VectorStore::load_from_file(TEST_STORE_PATH)?;
    
    // Verify data integrity
    assert_eq!(loaded_store.chunks.len(), original_chunk_count, "Loaded store should have same number of chunks");
    
    for (original, loaded) in original_store.chunks.iter().zip(loaded_store.chunks.iter()) {
        assert_eq!(original.id, loaded.id, "Chunk IDs should match");
        assert_eq!(original.content, loaded.content, "Chunk content should match");
        assert_eq!(original.embedding.len(), loaded.embedding.len(), "Embedding dimensions should match");
        assert_eq!(original.metadata, loaded.metadata, "Metadata should match");
    }
    
    println!("Vectorstore persistence test passed with {} chunks", original_chunk_count);
    
    // Clean up
    let _ = fs::remove_file(TEST_STORE_PATH);
    
    Ok(())
}

#[test]
fn test_epub_processing_integration() -> Result<()> {
    // Test that EPUB processing works correctly
    let markdown_chunks = epub_to_markdown(TEST_EPUB_PATH)?;
    
    assert!(!markdown_chunks.is_empty(), "Should extract markdown chunks from EPUB");
    
    // Verify chunks contain actual content (not just whitespace)
    let non_empty_chunks: Vec<_> = markdown_chunks
        .iter()
        .filter(|chunk| !chunk.trim().is_empty())
        .collect();
    
    assert!(!non_empty_chunks.is_empty(), "Should have non-empty chunks");
    
    println!("Successfully processed EPUB into {} chunks ({} non-empty)", 
             markdown_chunks.len(), non_empty_chunks.len());
    
    Ok(())
}

#[tokio::test]
async fn test_embedding_generation() -> Result<()> {
    let test_text = "This is a test sentence for embedding generation.";
    let embedding = get_single_embedding(test_text).await?;
    
    assert!(!embedding.is_empty(), "Embedding should not be empty");
    assert!(embedding.len() > 100, "Embedding should have reasonable dimension");
    
    // Test that different texts produce different embeddings
    let test_text2 = "This is a completely different sentence about something else.";
    let embedding2 = get_single_embedding(test_text2).await?;
    
    assert_eq!(embedding.len(), embedding2.len(), "Embeddings should have same dimension");
    assert_ne!(embedding, embedding2, "Different texts should produce different embeddings");
    
    println!("Generated embeddings with dimension: {}", embedding.len());
    
    Ok(())
}