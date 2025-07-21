use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cipher::{epub_to_markdown, get_embeddings, create_vectorstore_from_epub, rag_query, query_vectorstore};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Convert EPUB to markdown and show embeddings
    Convert {
        epub_path: String,
    },
    /// Create a vectorstore from an EPUB file
    Index {
        epub_path: String,
        #[arg(short, long, default_value = "vectorstore.json")]
        output: String,
    },
    /// Search the vectorstore for similar content
    Search {
        #[arg(short, long, default_value = "vectorstore.json")]
        store_path: String,
        query: String,
        #[arg(short, long, default_value = "5")]
        top_k: usize,
    },
    /// Query the vectorstore using RAG
    Rag {
        #[arg(short, long, default_value = "vectorstore.json")]
        store_path: String,
        query: String,
        #[arg(short, long, default_value = "3")]
        top_k: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    match args.command {
        Commands::Convert { epub_path } => {
            let markdown_chunks = epub_to_markdown(&epub_path).context("Failed to convert EPUB to Markdown")?;
            let embeddings = get_embeddings(markdown_chunks).await?;
            println!("Generated {} embeddings", embeddings.len());
            for (i, embedding) in embeddings.iter().enumerate() {
                println!("Chunk {}: embedding dimension {}", i, embedding.len());
            }
        }
        Commands::Index { epub_path, output } => {
            println!("Creating vectorstore from EPUB: {}", epub_path);
            let _store = create_vectorstore_from_epub(&epub_path, &output).await?;
            println!("Vectorstore created successfully at: {}", output);
        }
        Commands::Search { store_path, query, top_k } => {
            println!("Searching vectorstore: {}", store_path);
            println!("Query: {}", query);
            let results = query_vectorstore(&store_path, &query, top_k).await?;
            println!("\nSearch Results:");
            for (i, (score, content)) in results.iter().enumerate() {
                println!("{}. Score: {:.3}\n{}\n", i + 1, score, content);
            }
        }
        Commands::Rag { store_path, query, top_k } => {
            println!("RAG query on vectorstore: {}", store_path);
            println!("Query: {}", query);
            let answer = rag_query(&store_path, &query, top_k).await?;
            println!("\nAnswer:\n{}", answer);
        }
    }
    
    Ok(())
}
