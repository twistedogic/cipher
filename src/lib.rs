use anyhow::{Result};
use epub::doc::EpubDoc;
use std::path::Path;
use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use uuid::Uuid;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkData {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f64>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStore {
    pub chunks: Vec<ChunkData>,
    pub index: HashMap<String, usize>,
}

impl VectorStore {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            index: HashMap::new(),
        }
    }

    pub fn add_chunk(&mut self, content: String, embedding: Vec<f64>, metadata: HashMap<String, String>) -> String {
        let id = Uuid::new_v4().to_string();
        let chunk = ChunkData {
            id: id.clone(),
            content,
            embedding,
            metadata,
        };
        
        let index = self.chunks.len();
        self.chunks.push(chunk);
        self.index.insert(id.clone(), index);
        
        id
    }

    pub fn similarity_search(&self, query_embedding: &[f64], top_k: usize) -> Vec<(f64, &ChunkData)> {
        let mut similarities: Vec<(f64, &ChunkData)> = self.chunks
            .iter()
            .map(|chunk| {
                let similarity = cosine_similarity(query_embedding, &chunk.embedding);
                (similarity, chunk)
            })
            .collect();

        similarities.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        similarities.into_iter().take(top_k).collect()
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> Result<Self> {
        let json = fs::read_to_string(path)?;
        let store: VectorStore = serde_json::from_str(&json)?;
        Ok(store)
    }
}

fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

pub async fn get_embeddings(markdown_chunks: Vec<String>) -> Result<Vec<Vec<f64>>> {
    let ollama = Ollama::default();
    let mut embeddings = Vec::new();

    for chunk in markdown_chunks {
        if chunk.trim().is_empty() {
            continue;
        }

        let res = ollama.generate_embeddings("mxbai-embed-large".to_string(), chunk.to_string(), None).await;

        if let Ok(res) = res {
            embeddings.push(res.embeddings);
        } else {
            eprintln!("Failed to generate embeddings: {:?}", res);
        }
    }

    Ok(embeddings)
}

pub async fn get_single_embedding(text: &str) -> Result<Vec<f64>> {
    let ollama = Ollama::default();
    
    let res = ollama.generate_embeddings("mxbai-embed-large".to_string(), text.to_string(), None).await?;
    Ok(res.embeddings)
}

pub async fn create_vectorstore_from_epub(epub_path: &str, store_path: &str) -> Result<VectorStore> {
    let markdown_chunks = epub_to_markdown(epub_path)?;
    let embeddings = get_embeddings(markdown_chunks.clone()).await?;
    
    let mut store = VectorStore::new();
    
    for (i, (chunk, embedding)) in markdown_chunks.into_iter().zip(embeddings.into_iter()).enumerate() {
        if chunk.trim().is_empty() {
            continue;
        }
        
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), epub_path.to_string());
        metadata.insert("chunk_index".to_string(), i.to_string());
        
        store.add_chunk(chunk, embedding, metadata);
    }
    
    store.save_to_file(store_path)?;
    Ok(store)
}

pub async fn query_vectorstore(store: &VectorStore, query: &str, top_k: usize) -> Result<Vec<(f64, String)>> {
    let query_embedding = get_single_embedding(query).await?;
    let results = store.similarity_search(&query_embedding, top_k);
    
    Ok(results.into_iter().map(|(score, chunk)| (score, chunk.content.clone())).collect())
}

pub async fn rag_query(store_path: &str, query: &str, top_k: usize) -> Result<String> {
    let store = VectorStore::load_from_file(store_path)?;
    let relevant_chunks = query_vectorstore(&store, query, top_k).await?;
    
    let context: String = relevant_chunks
        .iter()
        .map(|(score, content)| format!("(Score: {:.3}) {}", score, content))
        .collect::<Vec<_>>()
        .join("\n\n");
    
    let ollama = Ollama::default();
    
    let prompt = format!(
        "Based on the following context from an EPUB book, answer the question: {}\n\nContext:\n{}\n\nAnswer:",
        query, context
    );
    
    let request = GenerationRequest::new("llama3.1".to_string(), prompt);
    let response = ollama.generate(request).await?;
    Ok(response.response)
}
