# Cipher - EPUB RAG System with JSON Vectorstore

A high-performance Rust-based system for processing EPUB files and creating a Retrieval-Augmented Generation (RAG) system using Ollama embeddings and a JSON-based vectorstore with cosine similarity search.

## Features

- Convert EPUB files to markdown chunks with intelligent text splitting
- Generate embeddings using Ollama's `mxbai-embed-large` model
- Store embeddings in a JSON-based vectorstore with metadata
- Fast similarity search using cosine similarity
- RAG-based question answering using `llama3.1` model
- Command-line interface for easy interaction

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
# Install Ollama (if not already installed)
curl -fsSL https://ollama.ai/install.sh | sh

# Pull the required models
ollama pull mxbai-embed-large
ollama pull llama3.1
```

## Usage

### Convert EPUB to Markdown (with embeddings info)

```bash
cargo run -- convert path/to/book.epub
```

### Create a Vectorstore

```bash
cargo run -- index path/to/book.epub --output my_vectorstore.json
```

### Search the Vectorstore

```bash
cargo run -- search --store-path my_vectorstore.json "search query" --top-k 5
```

### RAG Query

```bash
cargo run -- rag --store-path my_vectorstore.json "What is this book about?" --top-k 3
```

## Architecture

### Vectorstore Structure

The JSON vectorstore contains:
- **chunks**: Array of text chunks with embeddings
- **embedding_dim**: Dimension of the embedding vectors
- **metadata**: Source information and chunk indices

### Similarity Search

Uses cosine similarity to find the most relevant chunks:
```
similarity = (A · B) / (||A|| × ||B||)
```

### RAG Pipeline

1. **Indexing**: EPUB → Markdown chunks → Embeddings → JSON vectorstore
2. **Retrieval**: Query → Embedding → Similarity search → Top-K chunks
3. **Generation**: Context + Query → Ollama LLM → Answer

## Example

```bash
# Index a book
cargo run -- index testdata/pg35542.epub --output alice.json

# Search for content
cargo run -- search --store-path alice.json "rabbit hole" --top-k 3

# Ask questions about the book
cargo run -- rag --store-path alice.json "Who is the main character?" --top-k 5
```

## Testing

```bash
# Run all tests (requires Ollama)
cargo test

# Run only unit tests
cargo test --lib

# Run integration tests (requires Ollama)
cargo test --test vectorstore_integration
```

## Performance

- **Storage**: Efficient JSON serialization with serde
- **Search**: O(n) cosine similarity with early termination
- **Memory**: In-memory search for fast retrieval
- **Scalability**: Suitable for books up to several thousand chunks

## File Structure

```
src/
├── lib.rs          # Core library functions
└── main.rs         # CLI interface

tests/
├── cli.rs          # CLI integration tests
├── epub_to_markdown.rs  # EPUB processing tests
└── vectorstore_integration.rs  # End-to-end tests

testdata/
└── pg35542.epub    # Sample EPUB file
```