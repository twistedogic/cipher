use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_target_debug_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("epub_to_markdown_rs"); // Name of the executable
    path
}


#[test]
fn test_epub_to_markdown_conversion() -> Result<()> {
    let test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("testdata");
    let executable_path = get_target_debug_path();

    // Ensure the executable exists (it should after a build)
    if !executable_path.exists() {
        // Attempt to build if not found (e.g., when running tests individually)
        let build_status = Command::new("cargo")
            .arg("build")
            .current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR"))) // Run from project root
            .status()?;
        if !build_status.success() {
            panic!("Failed to build the project before testing. Executable not found at {:?}.", executable_path);
        }
    }

    if !test_data_dir.exists() {
        panic!("Test data directory not found at {:?}", test_data_dir);
    }

    for entry in fs::read_dir(&test_data_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "epub") {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            println!("Testing with file: {}", file_name);

            let output = Command::new(&executable_path)
                .arg(&path)
                .output()
                .expect("Failed to execute command");

            // Print stdout and stderr for debugging
            if !output.stdout.is_empty() {
                // println!("Stdout:\n{}", String::from_utf8_lossy(&output.stdout));
            }
            if !output.stderr.is_empty() {
                eprintln!("Stderr:\n{}", String::from_utf8_lossy(&output.stderr));
            }

            assert!(output.status.success(), "Command failed for {}. Stderr: {}", file_name, String::from_utf8_lossy(&output.stderr));
        }
    }
    Ok(())
}
