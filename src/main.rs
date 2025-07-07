use anyhow::{Context, Result};
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
    for (idx, spine_item_id) in spine_ids.iter().enumerate() {
        match doc.get_resource(spine_item_id) {
            Ok(content_bytes_vec) => {
                let html_content = String::from_utf8_lossy(&content_bytes_vec);
                let markdown = html2md::parse_html(&html_content);
                println!("\n--- Chapter {} ---\n", idx + 1);
                println!("{}", markdown);
            }
            Err(e) => {
                eprintln!("Warning: Could not read content for spine item {}: {}", spine_item_id, e);
            }
        }
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
