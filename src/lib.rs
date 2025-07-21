use anyhow::Result;
use epub::doc::EpubDoc;
use ollama_rs::Ollama;
use ollama_rs::generation::completion::request::GenerationRequest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkData {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStore {
    pub chunks: Vec<ChunkData>,
    pub embedding_dim: usize,
}

impl VectorStore {
    pub fn new(embedding_dim: usize) -> Self {
        Self {
            chunks: Vec::new(),
            embedding_dim,
        }
    }

    pub fn add_chunk(&mut self, content: String, embedding: Vec<f32>, metadata: HashMap<String, String>) {
        let chunk = ChunkData {
            id: Uuid::new_v4().to_string(),
            content,
            embedding,
            metadata,
        };
        self.chunks.push(chunk);
    }

    pub fn save_to_file(&self, file_path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(file_path, json)?;
        Ok(())
    }

    pub fn load_from_file(file_path: &str) -> Result<Self> {
        let json = fs::read_to_string(file_path)?;
        let store: VectorStore = serde_json::from_str(&json)?;
        Ok(store)
    }

    pub fn search(&self, query_embedding: &[f32], top_k: usize) -> Vec<(f32, String)> {
        let mut similarities: Vec<(f32, String)> = self
            .chunks
            .iter()
            .map(|chunk| {
                let similarity = cosine_similarity(query_embedding, &chunk.embedding);
                (similarity, chunk.content.clone())
            })
            .collect();

        similarities.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        similarities.truncate(top_k);
        similarities
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

pub fn epub_to_markdown(epub_path: &str) -> Result<Vec<String>> {
    let mut doc = EpubDoc::new(epub_path)?;
    let mut markdown_chunks = Vec::new();

    for spine_id in doc.spine.clone() {
        if let Ok(content) = doc.get_resource_str(&spine_id) {
            let markdown = html2md::parse_html(&content);
            if !markdown.trim().is_empty() {
                // Split into chunks by paragraphs
                let chunks: Vec<String> = markdown
                    .split("\n\n")
                    .filter(|chunk| !chunk.trim().is_empty() && chunk.len() > 50)
                    .map(|s| s.to_string())
                    .collect();
                
                markdown_chunks.extend(chunks);
            }
        }
    }

    Ok(markdown_chunks)
}

pub async fn get_embeddings(markdown_chunks: Vec<String>) -> Result<Vec<Vec<f32>>> {
    let ollama = Ollama::default();
    let mut embeddings = Vec::new();

    for chunk in markdown_chunks {
        if chunk.trim().is_empty() {
            continue;
        }

        let res = ollama.generate_embeddings("mxbai-embed-large".to_string(), chunk.to_string(), None).await?;
        // Convert f64 to f32
        let f32_embedding: Vec<f32> = res.embeddings.into_iter().map(|x| x as f32).collect();
        embeddings.push(f32_embedding);
    }

    Ok(embeddings)
}

pub async fn get_single_embedding(text: &str) -> Result<Vec<f32>> {
    let ollama = Ollama::default();
    
    let res = ollama.generate_embeddings("mxbai-embed-large".to_string(), text.to_string(), None).await?;
    // Convert f64 to f32
    let f32_embedding: Vec<f32> = res.embeddings.into_iter().map(|x| x as f32).collect();
    Ok(f32_embedding)
}

pub async fn create_vectorstore_from_epub(epub_path: &str, store_path: &str) -> Result<VectorStore> {
    println!("Converting EPUB to markdown chunks...");
    let markdown_chunks = epub_to_markdown(epub_path)?;
    println!("Generated {} markdown chunks", markdown_chunks.len());

    println!("Generating embeddings...");
    let embeddings = get_embeddings(markdown_chunks.clone()).await?;
    println!("Generated {} embeddings", embeddings.len());

    let embedding_dim = embeddings.first().map(|e| e.len()).unwrap_or(0);
    let mut store = VectorStore::new(embedding_dim);
    
    for (i, (chunk, embedding)) in markdown_chunks.iter().zip(embeddings.iter()).enumerate() {
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), epub_path.to_string());
        metadata.insert("chunk_index".to_string(), i.to_string());
        
        store.add_chunk(chunk.clone(), embedding.clone(), metadata);
    }

    store.save_to_file(store_path)?;
    println!("Vectorstore created successfully at: {}", store_path);

    Ok(store)
}

pub async fn query_vectorstore(store_path: &str, query: &str, top_k: usize) -> Result<Vec<(f32, String)>> {
    let query_embedding = get_single_embedding(query).await?;
    let store = VectorStore::load_from_file(store_path)?;
    Ok(store.search(&query_embedding, top_k))
}

pub async fn rag_query(store_path: &str, query: &str, top_k: usize) -> Result<String> {
    let relevant_chunks = query_vectorstore(store_path, query, top_k).await?;
    
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
