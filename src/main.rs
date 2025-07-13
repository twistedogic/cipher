use anyhow::{Context, Result};
use clap::Parser;
use cipher::{epub_to_markdown, get_embeddings};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    epub_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let markdown_chunks = epub_to_markdown(&args.epub_path).context("Failed to convert EPUB to Markdown")?;
    let embeddings = get_embeddings(markdown_chunks).await?;
    for embedding in embeddings {
        println!("Embedding for chunk: {:?}", embedding);
    }
    Ok(())
}
