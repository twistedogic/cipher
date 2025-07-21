# Cipher - EPUB RAG System

A Rust-based system for processing EPUB files and creating a Retrieval-Augmented Generation (RAG) system using Ollama embeddings and local vectorstore.

## Features

- Convert EPUB files to markdown chunks
- Generate embeddings using Ollama's `mxbai-embed-large` model
- Store embeddings in a local JSON-based vectorstore
- Perform similarity search on stored content
- RAG-based question answering using `llama3.1` model

## Prerequisites

- Rust (latest stable version)
- Ollama installed and running locally
- Required Ollama models:
  - `mxbai-embed-large` (for embeddings)
  - `llama3.1` (for text generation)

## Installation

```bash
# Clone the repository
git clone <repository-url>
cd cipher

# Build the project
cargo build --release
```

## Setup Ollama

```bash
# Install Ollama (follow instructions at https://ollama.ai)
# Pull required models
ollama pull mxbai-embed-large
ollama pull llama3.1
```

## Usage

### Convert EPUB to Markdown (with embeddings info)

```bash
cargo run -- convert testdata/pg35542.epub
```

### Create Vectorstore from EPUB

```bash
cargo run -- index testdata/pg35542.epub --output my_book.json
```

### Query the Vectorstore (RAG)

```bash
cargo run -- query my_book.json "What is the main theme of the book?"
```

### Available Commands

- `convert <epub_path>` - Convert EPUB to markdown and show chunk information
- `index <epub_path> --output <store_path>` - Create vectorstore from EPUB
- `query <store_path> <query>` - Perform RAG query on vectorstore

## Library Usage

```rust
use cipher::{epub_to_markdown, create_vectorstore_from_epub, rag_query};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Convert EPUB to markdown chunks
    let chunks = epub_to_markdown("book.epub")?;
    
    // Create vectorstore with embeddings
    let store = create_vectorstore_from_epub("book.epub", "book_store.json").await?;
    
    // Query the vectorstore
    let answer = rag_query("book_store.json", "What is the main theme?", 3).await?;
    println!("Answer: {}", answer);
    
    Ok(())
}
```

## Implementation Details

### Vectorstore Structure

The vectorstore is implemented as a JSON file containing:
- **Chunks**: Text content with embeddings and metadata
- **Index**: UUID-based lookup for efficient access
- **Similarity Search**: Cosine similarity for finding relevant content

### RAG Pipeline

1. **Document Processing**: EPUB → HTML → Markdown chunks
2. **Embedding Generation**: Chunks → Vector embeddings via Ollama
3. **Storage**: Embeddings + metadata stored in JSON vectorstore
4. **Query Processing**: Query → Embedding → Similarity search → Context
5. **Answer Generation**: Context + Query → LLM response

### Key Components

- `VectorStore`: Main storage structure with similarity search
- `ChunkData`: Individual text chunks with embeddings and metadata
- `epub_to_markdown()`: EPUB processing pipeline
- `create_vectorstore_from_epub()`: End-to-end indexing
- `rag_query()`: Complete RAG query pipeline

## Testing

```bash
# Run all tests (requires Ollama running)
cargo test

# Run tests without Ollama integration
cargo test --test cli
cargo test --test epub_to_markdown
```

## Integration Tests

The project includes comprehensive integration tests in `tests/vectorstore_integration.rs`:

- **EPUB Processing**: Validates EPUB to markdown conversion
- **Embedding Generation**: Tests Ollama embedding API integration
- **Vectorstore Operations**: Create, save, load, and query operations
- **Similarity Search**: Accuracy of cosine similarity matching
- **End-to-End RAG**: Complete pipeline from EPUB to answer

## Configuration

### Ollama Configuration

By default, the system connects to Ollama at `http://localhost:11434`. The models used are:
- **Embedding Model**: `mxbai-embed-large`
- **Generation Model**: `llama3.1`

### Vectorstore Format

The vectorstore uses JSON format for simplicity and portability:

```json
{
  "chunks": [
    {
      "id": "uuid-string",
      "content": "text content",
      "embedding": [0.1, 0.2, ...],
      "metadata": {
        "source": "book.epub",
        "chunk_index": "0"
      }
    }
  ],
  "index": {
    "uuid-string": 0
  }
}
```

## Error Handling

The system includes comprehensive error handling for:
- EPUB file reading errors
- Ollama API connection issues
- Vectorstore file I/O operations
- Embedding generation failures

## Performance Considerations

- **Chunking**: Optimal chunk sizes for both embedding and retrieval
- **Similarity Search**: Efficient cosine similarity computation
- **Memory Usage**: Streaming processing for large EPUBs
- **Storage**: Compact JSON representation

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

This project is open source and available under the MIT License.