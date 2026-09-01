use std::path::{Path, PathBuf};

/// Emit the immutable provider source digest consumed by ASP Rust cache keys.
pub fn emit_provider_contract_digest() {
    let root = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo provides the package manifest root"),
    );
    let mut inputs = vec![root.join("Cargo.toml"), root.join("build.rs")];
    collect_files(&root.join("src"), &mut inputs);
    inputs.sort();

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"asp-rust.provider-contract.v1\0");
    for path in inputs {
        println!("cargo:rerun-if-changed={}", path.display());
        let relative = path
            .strip_prefix(&root)
            .expect("provider input belongs to root");
        let bytes = std::fs::read(&path).unwrap_or_else(|error| {
            panic!("read ASP Rust provider input {}: {error}", path.display())
        });
        let relative_bytes = relative.as_os_str().as_encoded_bytes();
        hasher.update(&(relative_bytes.len() as u64).to_be_bytes());
        hasher.update(relative_bytes);
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
    }
    println!(
        "cargo:rustc-env=ASP_RUST_PROVIDER_DIGEST=blake3-256:{}",
        hasher.finalize().to_hex()
    );
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) {
    println!("cargo:rerun-if-changed={}", directory.display());
    let mut entries = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("read provider directory {}: {error}", directory.display()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| panic!("read provider directory entry: {error}"));
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .unwrap_or_else(|error| panic!("inspect provider input {}: {error}", path.display()));
        if file_type.is_dir() {
            collect_files(&path, files);
        } else if file_type.is_file() {
            files.push(path);
        }
    }
}
