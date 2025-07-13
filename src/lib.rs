use anyhow::{Result};
use epub::doc::EpubDoc;
use std::path::Path;
use ollama_rs::Ollama;
use ollama_rs::generation::options::GenerationOptions;

pub fn epub_to_markdown(path_str: &str) -> Result<Vec<String>> {
    let path = Path::new(path_str);
    let mut doc = EpubDoc::new(path).map_err(|e| anyhow::anyhow!("Failed to open EPUB file: {}", e))?;

    let mut markdown_chunks = Vec::new();

    let spine_ids: Vec<String> = doc.spine.iter().cloned().collect();
    for (_idx, spine_item_id) in spine_ids.iter().enumerate() {
        if let Ok(content_bytes_vec) = doc.get_resource(spine_item_id) {
            let html_content = String::from_utf8_lossy(&content_bytes_vec);
            let markdown = html2md::parse_html(&html_content);
            markdown_chunks.push(markdown);
        }
    }

    Ok(markdown_chunks)
}

pub async fn get_embeddings(markdown_chunks: Vec<String>) -> Result<Vec<Vec<f64>>> {
    let ollama = Ollama::default();
    let mut embeddings = Vec::new();
    let options = GenerationOptions::default();

    for chunk in markdown_chunks {
        if chunk.trim().is_empty() {
            continue;
        }

        let res = ollama.generate_embeddings("mxbai-embed-large".to_string(), chunk.to_string(), Some(options.clone())).await;

        if let Ok(res) = res {
            embeddings.push(res.embeddings);
        } else {
            eprintln!("Failed to generate embeddings: {:?}", res);
        }
    }

    Ok(embeddings)
}
