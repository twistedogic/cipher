use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use epub::doc::EpubDoc;
use ollama_rs::generation::embeddings::EmbeddingsRequest;
use ollama_rs::Ollama;
use std::path::{Path, PathBuf};

use arrow_array::{FixedSizeListArray, Float32Array, Int32Array, RecordBatch, StringArray}; // Corrected Int32Array import
use arrow_schema::{DataType, Field, Schema};
use lancedb::connection::Connection;
use lancedb::table::NewTableBuilder;
use std::sync::Arc;

// Structure for top-level CLI
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

// Enum for subcommands
#[derive(Subcommand, Debug)]
enum Commands {
    /// Ingest an EPUB file into the vector store
    Ingest(IngestArgs),
    /// Query the vector store using RAG
    Query(QueryArgs),
}

// Arguments for the Ingest subcommand
#[derive(Parser, Debug)]
struct IngestArgs {
    /// Path to the EPUB file
    #[clap(short, long)]
    epub_path: String,

    /// Path to the LanceDB database directory
    #[clap(short, long, default_value = ".lancedb")]
    db_path: String,

    /// Table name in LanceDB
    #[clap(short, long, default_value = "epub_chunks")]
    table_name: String,

    /// Ollama server base URL (e.g., http://localhost:11434)
    #[clap(long, default_value = "http://localhost:11434")]
    ollama_url: String,

    /// Ollama embedding model name (e.g., nomic-embed-text)
    #[clap(long, default_value = "nomic-embed-text")]
    ollama_embedding_model: String,

    /// Embedding dimension for the chosen Ollama embedding model
    #[clap(long)]
    embedding_dim: i32,
}

// Arguments for the Query subcommand
#[derive(Parser, Debug)]
struct QueryArgs {
    /// The user's query
    #[clap(short, long)]
    query: String,

    /// Path to the LanceDB database directory
    #[clap(short, long, default_value = ".lancedb")]
    db_path: String,

    /// Table name in LanceDB
    #[clap(short, long, default_value = "epub_chunks")]
    table_name: String,

    /// Ollama server base URL (e.g., http://localhost:11434)
    #[clap(long, default_value = "http://localhost:11434")]
    ollama_url: String,

    /// Ollama embedding model name (for embedding the query)
    #[clap(long, default_value = "nomic-embed-text")]
    ollama_embedding_model: String,

    /// Embedding dimension for the Ollama embedding model (must match ingested data)
    #[clap(long)]
    embedding_dim: i32,

    /// Ollama generation model name (e.g., llama3, mistral)
    #[clap(long, default_value = "llama3")] // Or another sensible default
    ollama_generation_model: String,

    /// Number of top-K chunks to retrieve for context
    #[clap(short = 'k', long, default_value_t = 3)]
    top_k: usize,
}

async fn process_epub_and_embed(args: IngestArgs, ollama: Arc<Ollama>) -> Result<()> {
    let epub_path = Path::new(&args.epub_path);
    let mut doc = EpubDoc::new(epub_path)
        .map_err(|e| anyhow::anyhow!("Failed to open EPUB file '{}': {}", args.epub_path, e))?;

    let db_uri = PathBuf::from(&args.db_path);
    if !db_uri.exists() {
        std::fs::create_dir_all(&db_uri)
            .with_context(|| format!("Failed to create database directory at {:?}", db_uri))?;
    }
    let conn = Connection::connect(&db_uri.to_string_lossy()).await?;
    println!("Connected to LanceDB at {:?}", db_uri);

    let schema = Arc::new(Schema::new(vec![
        Field::new("text", DataType::Utf8, false),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                args.embedding_dim,
            ),
            true,
        ),
        Field::new("source_epub", DataType::Utf8, true),
        Field::new("chapter_idx", DataType::Int32, true),
        Field::new("chunk_idx_in_chapter", DataType::Int32, true),
    ]));

    let tbl_result = conn.open_table(&args.table_name).await;
    let mut tbl = match tbl_result {
        Ok(table) => {
            println!("Opened existing table '{}'", args.table_name);
            table
        }
        Err(_) => {
            println!("Table '{}' not found, creating new one.", args.table_name);
            NewTableBuilder::new(conn.clone(), &args.table_name, schema.clone()) // Pass cloned schema
                .create()
                .await?
        }
    };

    let epub_filename = epub_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    if let Some(titles) = doc.metadata.get("title") {
        if let Some(title) = titles.first() {
            println!("Processing EPUB: {}", title);
=======
use clap::Parser;
use epub::doc::EpubDoc;
use std::path::Path;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    epub_path: String,
}

fn epub_to_markdown(path_str: &str) -> Result<()> {
    let path = Path::new(path_str);
    let mut doc = EpubDoc::new(path).map_err(|e| anyhow::anyhow!("Failed to open EPUB file: {}", e))?;

    if let Some(titles) = doc.metadata.get("title") {
        if let Some(title) = titles.first() {
            println!("Title: {}", title);
        }
    }
    if let Some(creators) = doc.metadata.get("creator") {
        if let Some(creator) = creators.first() {
            println!("Creator: {}", creator);
        }
    }
    if let Some(languages) = doc.metadata.get("language") {
        if let Some(lang) = languages.first() {
            println!("Language: {}", lang);
        }
    }

    let spine_ids: Vec<String> = doc.spine.iter().cloned().collect();
    println!("\nProcessing content (spine items)...\n");

    for (chapter_idx_usize, spine_item_id) in spine_ids.iter().enumerate() {
        let chapter_idx = chapter_idx_usize as i32;
        match doc.get_resource(spine_item_id) {
            Ok(content_bytes_vec) => {
                let html_content = String::from_utf8_lossy(&content_bytes_vec);
                if html_content.trim().is_empty() {
                    continue;
                }
                let markdown_text = html2md::parse_html(&html_content);
                if markdown_text.trim().is_empty() {
                    continue;
                }

                println!(
                    "--- Chapter {} (Spine Item: {}) ---",
                    chapter_idx + 1,
                    spine_item_id
                );
                let text_chunks = chunk_markdown(&markdown_text);
                if text_chunks.is_empty() {
                    println!("No text chunks found in this chapter.");
                    continue;
                }

                println!(
                    "Generating embeddings for {} chunks via Ollama...",
                    text_chunks.len()
                );

                let mut current_embeddings_for_db: Vec<Vec<f32>> = Vec::new();
                let mut current_texts_for_db: Vec<String> = Vec::new();

                for text_chunk in text_chunks.iter() {
                    if text_chunk.trim().is_empty() {
                        continue;
                    }
                    let request = EmbeddingsRequest::new(
                        args.ollama_embedding_model.clone(),
                        text_chunk.clone(),
                    );
                    match ollama.generate_embeddings(request).await {
                        Ok(response) => {
                            let embedding_f32: Vec<f32> =
                                response.embeddings.iter().map(|&x| x as f32).collect();
                            if embedding_f32.len() == args.embedding_dim as usize {
                                current_embeddings_for_db.push(embedding_f32);
                                current_texts_for_db.push(text_chunk.to_string());
                            } else {
                                eprintln!(
                                    "Warning: Ollama model '{}' returned embedding of dimension {} for chunk '{}...', but CLI specified dimension {}. Skipping this chunk.",
                                    args.ollama_embedding_model,
                                    embedding_f32.len(),
                                    text_chunk.chars().take(30).collect::<String>(),
                                    args.embedding_dim
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "Error generating Ollama embedding for chunk '{}': {:?}",
                                text_chunk, e
                            );
                        }
                    }
                }

                if !current_texts_for_db.is_empty() {
                    println!(
                        "Embeddings generated for chapter {}. Preparing to save to DB.",
                        chapter_idx + 1
                    );
                    let mut text_col_builder = arrow_array::builder::StringBuilder::new();
                    let mut embedding_col_builder = arrow_array::builder::FixedSizeListBuilder::new(
                        arrow_array::builder::Float32Builder::new(),
                        args.embedding_dim,
                    );
                    let mut source_epub_col_builder = arrow_array::builder::StringBuilder::new();
                    let mut chapter_idx_col_builder = arrow_array::builder::Int32Builder::new();
                    let mut chunk_idx_in_chapter_col_builder =
                        arrow_array::builder::Int32Builder::new();

                    for (idx, text_chunk) in current_texts_for_db.iter().enumerate() {
                        if let Some(embedding_f32) = current_embeddings_for_db.get(idx) {
                            text_col_builder.append_value(text_chunk);
                            source_epub_col_builder.append_value(&epub_filename);
                            chapter_idx_col_builder.append_value(chapter_idx);
                            chunk_idx_in_chapter_col_builder.append_value(idx as i32);

                            let value_builder = embedding_col_builder.values();
                            // This check is redundant if filtering during embedding generation worked perfectly
                            if embedding_f32.len() == args.embedding_dim as usize {
                                for val in embedding_f32 {
                                    value_builder.append_value(*val);
                                }
                                embedding_col_builder.append(true);
                            } else {
                                // This case means an embedding with wrong dimension slipped through.
                                // Appending null to maintain row consistency.
                                eprintln!("Error: Mismatched embedding dimension at RecordBatch creation for chunk: {}. Expected {}, got {}. Appending null.", text_chunk.chars().take(30).collect::<String>(), args.embedding_dim, embedding_f32.len());
                                embedding_col_builder.append_null();
                            }
                        }
                    }

                    if text_col_builder.len() > 0 {
                        let batch_schema = schema.clone();
                        let record_batch = RecordBatch::try_new(
                            batch_schema,
                            vec![
                                Arc::new(text_col_builder.finish()),
                                Arc::new(embedding_col_builder.finish()),
                                Arc::new(source_epub_col_builder.finish()),
                                Arc::new(chapter_idx_col_builder.finish()),
                                Arc::new(chunk_idx_in_chapter_col_builder.finish()),
                            ],
                        )
                        .with_context(|| "Failed to create RecordBatch")?;

                        tbl.add(Arc::new(record_batch), None).await?;
                        println!(
                            "Added {} chunks from chapter {} to LanceDB.",
                            current_texts_for_db.len(),
                            chapter_idx + 1
                        );
                    }
                }
                println!("--- End of Chapter {} ---\n", chapter_idx + 1);
            }
            Err(e) => {
                eprintln!(
                    "Warning: Could not read content for spine item {}: {}",
                    spine_item_id, e
                );
            }
        }
    }
    Ok(())
}

fn chunk_markdown(markdown: &str) -> Vec<String> {
    use pulldown_cmark::{Event, Parser, Tag};
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    for event in Parser::new(markdown) {
        match event {
            Event::Start(Tag::Paragraph) => current_chunk.clear(),
            Event::End(Tag::Paragraph) => {
                if !current_chunk.trim().is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                }
                current_chunk.clear();
            }
            Event::Text(text) => current_chunk.push_str(&text),
            Event::Start(Tag::Heading(_, _, _))
            | Event::Start(Tag::List(_))
            | Event::Start(Tag::BlockQuote) => {
                if !current_chunk.trim().is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                }
                current_chunk.clear();
            }
            Event::End(Tag::Heading(_, _, _))
            | Event::End(Tag::List(_))
            | Event::End(Tag::BlockQuote) => {
                if !current_chunk.trim().is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                }
                current_chunk.clear();
            }
            _ => {}
        }
    }
    if !current_chunk.trim().is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }
    chunks
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest(args) => {
            println!(
                "Initializing Ollama client for Ingest with URL: {}",
                args.ollama_url
            );
            let ollama = Arc::new(Ollama::from_url(args.ollama_url.clone()));
            process_epub_and_embed(args, ollama).await?;
        }
        Commands::Query(args) => {
            println!(
                "Initializing Ollama client for Query with URL: {}",
                args.ollama_url
            );
            let ollama = Arc::new(Ollama::from_url(args.ollama_url.clone()));
            handle_query_command(args, ollama).await?;
        }
    }
    Ok(())
}

async fn handle_query_command(args: QueryArgs, ollama: Arc<Ollama>) -> Result<()> {
    println!("Received query: '{}'", args.query);
    println!(
        "Using Ollama embedding model: {}",
        args.ollama_embedding_model
    );
    println!("Expected embedding dimension: {}", args.embedding_dim);

    // 1. Generate embedding for the query
    let query_embedding_request =
        EmbeddingsRequest::new(args.ollama_embedding_model.clone(), args.query.clone());
    let query_embedding_response = ollama.generate_embeddings(query_embedding_request).await?;

    let query_embedding_f64 = query_embedding_response.embeddings;
    let query_embedding_f32: Vec<f32> = query_embedding_f64.iter().map(|&x| x as f32).collect();

    if query_embedding_f32.len() != args.embedding_dim as usize {
        return Err(anyhow::anyhow!(
            "Query embedding dimension mismatch. Expected {}, got {}. Model: '{}'",
            args.embedding_dim,
            query_embedding_f32.len(),
            args.ollama_embedding_model
        ));
    }

    println!(
        "Generated query embedding (first 3/{} dims): [{:.3}, {:.3}, {:.3}, ...]",
        query_embedding_f32.len(),
        query_embedding_f32.get(0).unwrap_or(&0.0),
        query_embedding_f32.get(1).unwrap_or(&0.0),
        query_embedding_f32.get(2).unwrap_or(&0.0)
    );

    // TODO:
    // 2. Connect to LanceDB and open table (similar to ingest)
    // 3. Perform similarity search using the query_embedding_f32
    // 4. Retrieve top-K chunks
    // 5. Formulate prompt with context and query
    // 6. Call Ollama generation model
    // 2. Connect to LanceDB and open table
    let db_uri = PathBuf::from(&args.db_path);
    let conn = Connection::connect(&db_uri.to_string_lossy())
        .await
        .with_context(|| format!("Failed to connect to LanceDB at {:?}", db_uri))?;

    let tbl = conn.open_table(&args.table_name).await.with_context(|| {
        format!(
            "Failed to open table '{}' in LanceDB at {:?}",
            args.table_name, db_uri
        )
    })?;

    // 3. Perform similarity search
    println!(
        "Performing similarity search in LanceDB table '{}'...",
        args.table_name
    );
    let search_results = tbl
        .search(&query_embedding_f32) // Pass Vec<f32>
        .limit(args.top_k)
        .select(&["text", "source_epub", "chapter_idx", "chunk_idx_in_chapter"]) // Select columns to retrieve
        .execute_stream() // Returns a stream of RecordBatch
        .await
        .with_context(|| "Failed to execute search query in LanceDB")?;

    // The execute_stream() method returns a QueryResultStream which itself is a Stream of RecordBatch.
    // We need to collect these results.
    use futures::stream::TryStreamExt; // For try_collect
    let record_batches: Vec<RecordBatch> = search_results
        .try_collect()
        .await
        .with_context(|| "Failed to collect search results from LanceDB")?;

    let mut retrieved_chunks_text: Vec<String> = Vec::new();

    println!("\nRetrieved Chunks (Top {}):", args.top_k);
    if record_batches.is_empty() {
        println!("No chunks found matching the query.");
    } else {
        for batch in record_batches {
            let text_array = batch
                .column_by_name("text")
                .ok_or_else(|| anyhow::anyhow!("'text' column not found in search results"))?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| {
                    anyhow::anyhow!("Failed to downcast 'text' column to StringArray")
                })?;

            // You can also retrieve other metadata columns like "source_epub", "chapter_idx" if needed
            // let source_epub_array = batch.column_by_name("source_epub")...
            // let chapter_idx_array = batch.column_by_name("chapter_idx")...

            for i in 0..text_array.len() {
                if retrieved_chunks_text.len() < args.top_k {
                    // Ensure we don't exceed top_k due to batching
                    let text_val = text_array.value(i).to_string();
                    println!(
                        "---\nChunk {}:\n{}\n---",
                        retrieved_chunks_text.len() + 1,
                        text_val
                    );
                    retrieved_chunks_text.push(text_val);
                } else {
                    break;
                }
            }
            if retrieved_chunks_text.len() >= args.top_k {
                break;
            }
        }
    }

    if retrieved_chunks_text.is_empty() {
        println!("No relevant chunks found in the database for the query.");
        // Decide if to proceed to LLM or not. For now, let's say we don't.
        return Ok(());
    }

    // TODO:
    // 4. (Done) Retrieve top-K chunks

    // 5. Formulate prompt with context and query
    // Prepare context from retrieved chunks
    let context_string = retrieved_chunks_text.join("\n\n---\n\n"); // Simple join with separator

    println!("\n--- Formulated Context for LLM ---");
    println!("{}\n---------------------------------", context_string);

    // 6. Formulate the prompt for the generation model
    let prompt = format!(
        "You are a helpful assistant. Answer the following question based only on the provided context. If the context does not contain the answer, state that you don't know based on the context provided.\n\nContext:\n---\n{}\n---\n\nQuestion: {}\n\nAnswer:",
        context_string,
        args.query
    );

    println!("\n--- Generated Prompt for LLM ---");
    println!("{}\n--------------------------------", prompt);

    // 7. Call Ollama generation model with this prompt
    println!(
        "\n--- Sending prompt to Ollama generation model: {} ---",
        args.ollama_generation_model
    );

    // Depending on ollama-rs version, this might be `generate_chat_completion` or `generate_completion`
    // For newer versions, it's often a specific request struct.
    // Let's assume a simple completion for now, or adjust if using chat models specifically.
    // ollama_rs::generation::completion::request::GenerationRequest is for completions.
    // ollama_rs::generation::chat::request::ChatMessageRequest for chat.
    // For a general prompt, GenerationRequest is usually suitable.
    use ollama_rs::generation::completion::request::GenerationRequest;

    let gen_request = GenerationRequest::new(args.ollama_generation_model.clone(), prompt.clone());
    // Match on the result of sending the request
    match ollama.generate_completion(gen_request).await {
        Ok(response) => {
            // Assuming response.response is the field containing the generated text.
            // Check the specific structure of `GenerationResponse` from `ollama-rs`.
            // For ollama-rs 0.1.7, it's likely `response.response`.
            println!("\n--- LLM Response ---");
            println!("{}", response.response);
        }
        Err(e) => {
            eprintln!(
                "Error generating response from Ollama model '{}': {:?}",
                args.ollama_generation_model, e
            );
            // Optionally, return an error here or handle it as needed
            // return Err(anyhow::anyhow!("Ollama generation failed: {}", e));
        }
    }

    // 8. Print response (Done above)
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    // Unit tests for chunk_markdown
    #[test]
    fn test_chunk_markdown_empty() {
        let markdown = "";
        let chunks = chunk_markdown(markdown);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_markdown_single_paragraph() {
        let markdown = "This is a single paragraph.";
        let chunks = chunk_markdown(markdown);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "This is a single paragraph.");
    }

    #[test]
    fn test_chunk_markdown_multiple_paragraphs() {
        let markdown = "Paragraph one.\n\nParagraph two.\n\nParagraph three.";
        let chunks = chunk_markdown(markdown);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], "Paragraph one.");
        assert_eq!(chunks[1], "Paragraph two.");
        assert_eq!(chunks[2], "Paragraph three.");
    }

    #[test]
    fn test_chunk_markdown_with_headings() {
        let markdown = "# Heading 1\n\nParagraph under H1.\n\n## Heading 2\n\nParagraph under H2.";
        let chunks = chunk_markdown(markdown);
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0], "Heading 1");
        assert_eq!(chunks[1], "Paragraph under H1.");
        assert_eq!(chunks[2], "Heading 2");
        assert_eq!(chunks[3], "Paragraph under H2.");
    }

    #[test]
    fn test_chunk_markdown_list_items() {
        let markdown = "* Item 1\n* Item 2\n* Item 3";
        let chunks = chunk_markdown(markdown);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], "Item 1");
        assert_eq!(chunks[1], "Item 2");
        assert_eq!(chunks[2], "Item 3");
    }

    #[test]
    fn test_chunk_markdown_text_then_heading() {
        let markdown = "Some leading text.\n\n# Heading";
        let chunks = chunk_markdown(markdown);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0], "Some leading text.");
        assert_eq!(chunks[1], "Heading");
    }

    #[test]
    fn test_chunk_markdown_heading_then_text() {
        let markdown = "# Heading\n\nSome trailing text.";
        let chunks = chunk_markdown(markdown);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0], "Heading");
        assert_eq!(chunks[1], "Some trailing text.");
    }

    #[test]
    fn test_chunk_markdown_mixed_content() {
        let markdown =
            "Intro text.\n\n# Title\n\nContent p1.\n\n* List item 1\n* List item 2\n\nContent p2.";
        let chunks = chunk_markdown(markdown);
        assert_eq!(chunks.len(), 6);
        assert_eq!(chunks[0], "Intro text.");
        assert_eq!(chunks[1], "Title");
        assert_eq!(chunks[2], "Content p1.");
        assert_eq!(chunks[3], "List item 1");
        assert_eq!(chunks[4], "List item 2");
        assert_eq!(chunks[5], "Content p2.");
    }

    let mut ncx_resource_ids: Vec<String> = Vec::new();
    for (id, (_path, media_type)) in doc.resources.iter() {
        if media_type == "application/x-dtbncx+xml" {
            ncx_resource_ids.push(id.clone());
        }
    }

    let mut ncx_found = false;
    for ncx_id in ncx_resource_ids {
        match doc.get_resource(&ncx_id) {
            Ok(ncx_bytes) => {
                let ncx_content = String::from_utf8_lossy(&ncx_bytes);
                println!("\n--- NCX (Table of Contents) ---");
                println!("{}", ncx_content);
                ncx_found = true;
                break;
            }
            Err(e) => {
                eprintln!("\n--- Error accessing NCX resource with ID '{}': {} ---", ncx_id, e);
            }
        }
    }

    if !ncx_found {
        println!("\n--- No NCX (Table of Contents) with media type 'application/x-dtbncx+xml' found in resources ---");
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    epub_to_markdown(&args.epub_path).context("Failed to convert EPUB to Markdown")?;
    Ok(())
}
