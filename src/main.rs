use anyhow::{Context, Result};
use clap::Parser;
use epub::doc::EpubDoc;
use ollama_rs::generation::embeddings::EmbeddingsRequest;
use ollama_rs::Ollama; // Added Ollama
use std::path::{Path, PathBuf}; // For generating embeddings

// LanceDB specific imports
use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray}; // For creating RecordBatch
use arrow_schema::{DataType, Field, Schema}; // For schema definition
use lancedb::connection::Connection;
use lancedb::table::NewTableBuilder; // For schema definition and table creation
use std::sync::Arc; // For Arc<Schema> and Arc<Field>

const EMBEDDING_DIM: i32 = 384; // For AllMiniLmL6V2

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
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

    /// Ollama embedding model name
    #[clap(long, default_value = "nomic-embed-text")]
    ollama_model: String,

    /// Embedding dimension for the chosen Ollama model
    #[clap(long)] // Making it mandatory for now, user must specify
    embedding_dim: i32,
}

async fn process_epub_and_embed(args: Args, ollama: Arc<Ollama>) -> Result<()> {
    let epub_path = Path::new(&args.epub_path);
    let mut doc = EpubDoc::new(epub_path)
        .map_err(|e| anyhow::anyhow!("Failed to open EPUB file '{}': {}", args.epub_path, e))?;

    // Connect to LanceDB
    let db_uri = PathBuf::from(&args.db_path);
    if !db_uri.exists() {
        std::fs::create_dir_all(&db_uri)
            .with_context(|| format!("Failed to create database directory at {:?}", db_uri))?;
    }
    let conn = Connection::connect(&db_uri.to_string_lossy()).await?;
    println!("Connected to LanceDB at {:?}", db_uri);

    // Define schema for LanceDB table
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
        Field::new("chapter_idx", DataType::Int32, true), // Using Int32 for chapter index
        Field::new("chunk_idx_in_chapter", DataType::Int32, true), // Using Int32 for chunk index
    ]));

    // Create or open table
    // Try to open first, if it fails, create it.
    let tbl_result = conn.open_table(&args.table_name).await;
    let mut tbl = match tbl_result {
        Ok(table) => {
            println!("Opened existing table '{}'", args.table_name);
            // Potentially verify schema compatibility here if needed
            table
        }
        Err(_) => {
            println!("Table '{}' not found, creating new one.", args.table_name);
            NewTableBuilder::new(conn.clone(), &args.table_name, schema.clone())
                .create()
                .await?
        }
    };

    // Metadata from EPUB
    let epub_filename = epub_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    if let Some(titles) = doc.metadata.get("title") {
        if let Some(title) = titles.first() {
            println!("Processing EPUB: {}", title);
        }
    }

    let spine_ids: Vec<String> = doc.spine.iter().cloned().collect();

    println!("\nProcessing content (spine items)...\n");

    for (chapter_idx_usize, spine_item_id) in spine_ids.iter().enumerate() {
        let chapter_idx = chapter_idx_usize as i32; // Convert to i32 for schema
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

                // let chunk_strs: Vec<&str> = text_chunks.iter().map(String::as_str).collect(); // Not needed for one-by-one

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
                    // Generate embedding for each chunk individually
                    // In future, ollama-rs might support batching for generate_embeddings.
                    // For now, one by one.
                    let request =
                        EmbeddingsRequest::new(args.ollama_model.clone(), text_chunk.clone());
                    match ollama.generate_embeddings(request).await {
                        Ok(response) => {
                            // ollama-rs returns Vec<f64>, convert to Vec<f32> for LanceDB schema
                            let embedding_f32: Vec<f32> =
                                response.embeddings.iter().map(|&x| x as f32).collect();

                            // TODO: Check embedding_f32.len() against EMBEDDING_DIM. This is crucial.
                            // Validate embedding dimension before adding to batch
                            if embedding_f32.len() == args.embedding_dim as usize {
                                current_embeddings_for_db.push(embedding_f32);
                                current_texts_for_db.push(text_chunk.to_string());
                            } else {
                                eprintln!(
                                    "Warning: Ollama model '{}' returned embedding of dimension {} for chunk '{}...', but CLI specified dimension {}. Skipping this chunk.",
                                    args.ollama_model,
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
                            // Decide how to handle: skip chunk, use zero vector, etc. For now, skip.
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
                            chunk_idx_in_chapter_col_builder.append_value(idx as i32); // Use current index in this batch

                            let value_builder = embedding_col_builder.values();
                            if embedding_f32.len() == args.embedding_dim as usize {
                                for val in embedding_f32 {
                                    value_builder.append_value(*val);
                                }
                                embedding_col_builder.append(true);
                            } else {
                                // This case should ideally be prevented by checks during embedding generation loop
                                eprintln!(
                                        "Critical Error: Embedding for chunk '{}...' has dimension {} but schema expects {}. This should not happen if pre-validated.",
                                        text_chunk.chars().take(30).collect::<String>(),
                                        embedding_f32.len(),
                                        args.embedding_dim
                                    );
                                // This will likely lead to a panic when building the RecordBatch or inserting into LanceDB
                                // if not all rows have consistent embedding lengths for FixedSizeList.
                                // A more robust solution would be to filter out such rows *before* this stage.
                                // For now, appending null or an empty list to make FixedSizeListBuilder happy if it supports it,
                                // or more simply, ensure this condition is caught and handled earlier.
                                // Let's assume the earlier embedding generation loop only adds correctly-dimensioned embeddings
                                // to `current_embeddings_for_db`. If not, this part is problematic.
                                // For this step, the focus is using args.embedding_dim.
                                // The validation logic is in the embedding loop.
                                // If an embedding of wrong dim got here, it's an issue with prior logic.
                                // We will proceed assuming valid dimensions here from the previous loop.
                                // If an invalid one sneaks through, LanceDB/Arrow will error out, which is an indicator.
                                embedding_col_builder.append_null(); // This might be one way if the builder supports it for FixedSizeList
                            }
                        }
                    }

                    if text_col_builder.len() > 0 {
                        let record_batch = RecordBatch::try_new(
                            schema.clone(),
                            vec![
                                Arc::new(text_col_builder.finish()),
                                Arc::new(embedding_col_builder.finish()),
                                Arc::new(source_epub_col_builder.finish()),
                                Arc::new(chapter_idx_col_builder.finish()),
                                Arc::new(chunk_idx_in_chapter_col_builder.finish()),
                            ],
                        )?;

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
            // Simplified: ignoring inline elements like Code, Html, SoftBreak, HardBreak etc. for now
            // or assuming they are part of the text collected by Event::Text.
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
    let args = Args::parse();

    // Initialize Ollama client once
    println!("Initializing Ollama client with URL: {}", args.ollama_url);
    let ollama = Arc::new(Ollama::from_url(args.ollama_url.clone()));

    // Optionally, check if the specified ollama_model is available on the server
    // This requires an async call. For robustness, good to uncomment and test.
    // let local_models = ollama.list_local_models().await?;
    // if !local_models.iter().any(|m| m.name == args.ollama_model) {
    //     return Err(anyhow::anyhow!("Ollama model '{}' not found on the server. Please pull it first.", args.ollama_model));
    // }
    // println!("Using Ollama model: {}", args.ollama_model);

    process_epub_and_embed(args, ollama).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // html2md usually produces clean paragraphs, so pulldown-cmark should see them.
        // Actual output of html2md might be <p>Paragraph one.</p><p>Paragraph two.</p>
        // Let's assume pulldown cmark handles standard Markdown paragraph separation.
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
        // Current chunking logic: heading text becomes a chunk, paragraph text becomes a chunk.
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0], "Heading 1");
        assert_eq!(chunks[1], "Paragraph under H1.");
        assert_eq!(chunks[2], "Heading 2");
        assert_eq!(chunks[3], "Paragraph under H2.");
    }

    #[test]
    fn test_chunk_markdown_list_items() {
        // The current chunker might treat the whole list as one chunk or list items individually
        // if they contain paragraphs, or text between list markers.
        // Let's test a simple list. The current logic will likely make the list content one chunk.
        let markdown = "* Item 1\n* Item 2\n* Item 3";
        let chunks = chunk_markdown(markdown);
        // The refined chunker separates text blocks by Start/End of List/Heading/Paragraph
        // Text nodes are collected.
        // For "* Item 1\n* Item 2", pulldown-cmark produces:
        // Start(List(None))
        //  Start(Item)
        //   Start(Paragraph)
        //    Text("Item 1")
        //   End(Paragraph)
        //  End(Item)
        //  Start(Item)
        //   Start(Paragraph)
        //    Text("Item 2")
        //   End(Paragraph)
        //  End(Item)
        // End(List(None))
        // So, "Item 1" and "Item 2" should become separate chunks due to End(Paragraph).
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
}
