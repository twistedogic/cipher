use anyhow::Result;
use cipher::epub_to_markdown;

#[test]
fn test_epub_to_markdown() -> Result<()> {
    let markdown_chunks = epub_to_markdown("testdata/pg35542.epub")?;
    assert!(!markdown_chunks.is_empty());
    Ok(())
}
