use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

fn write_snapshot_envelope(
    root: &std::path::Path,
    owner: &str,
    source: &str,
) -> std::path::PathBuf {
    let cas_root = root.join("cas");
    let blob_digest = "11".repeat(32);
    let cas_path = format!("{}/{}", &blob_digest[..2], &blob_digest[2..]);
    let blob_path = cas_root.join(&cas_path);
    fs::create_dir_all(blob_path.parent().expect("CAS blob parent")).expect("create CAS shard");
    fs::write(&blob_path, source).expect("write pinned source blob");
    let envelope_path = root.join("source-snapshot-envelope.v1.json");
    fs::write(
        &envelope_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaId": "asp.exact-source-snapshot-envelope.v1",
            "schemaVersion": "1",
            "providerId": "rs-harness-test",
            "sourceSnapshot": {
                "schemaId": "asp.source-snapshot.v1",
                "schemaVersion": "1",
                "algorithm": "blake3-merkle-v1",
                "rootDigest": "22".repeat(32),
                "sourceKind": "filesystem",
                "leafCount": 1,
                "providerDigest": "33".repeat(32),
            },
            "casRoot": cas_root,
            "owners": [{
                "path": owner,
                "snapshotLeafDigest": "44".repeat(32),
                "blobDigest": blob_digest,
                "casPath": cas_path,
            }],
        }))
        .expect("encode source snapshot envelope"),
    )
    .expect("write source snapshot envelope");
    envelope_path
}

#[test]
fn query_code_rejects_trailing_root_and_catalog_accepts_positional_workspace() {
    let Some(bin) = option_env!("CARGO_BIN_EXE_rs-harness") else {
        return;
    };
    let root = tempfile::tempdir().expect("temp root");
    fs::create_dir_all(root.path().join("src")).expect("create src");
    let source = "pub fn target() {}\n";
    fs::write(root.path().join("src/lib.rs"), source).expect("write fixture");
    let envelope_path = write_snapshot_envelope(root.path(), "src/lib.rs", source);

    let current = Command::new(bin)
        .args([
            "query",
            "--from-hook",
            "direct-source-read",
            "--selector",
            "rust://src/lib.rs#item/function/target",
            "--source-snapshot-envelope",
        ])
        .arg(&envelope_path)
        .args(["--workspace"])
        .arg(root.path())
        .arg("--code")
        .current_dir(root.path())
        .output()
        .expect("run current query command");

    assert!(
        current.status.success(),
        "current command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&current.stdout),
        String::from_utf8_lossy(&current.stderr)
    );
    assert_eq!(
        String::from_utf8(current.stdout)
            .expect("exact source code output is UTF-8")
            .trim(),
        "pub fn target() {}"
    );

    let stale = Command::new(bin)
        .args([
            "query",
            "--from-hook",
            "direct-source-read",
            "--selector",
            "rust://src/lib.rs#item/function/target",
            "--source-snapshot-envelope",
        ])
        .arg(&envelope_path)
        .args(["--code"])
        .arg(root.path())
        .current_dir(root.path())
        .output()
        .expect("run stale query command");

    assert!(
        !stale.status.success(),
        "stale command unexpectedly succeeded"
    );
    assert!(
        String::from_utf8_lossy(&stale.stderr).contains("rust query requires an exact --selector"),
        "stderr={}",
        String::from_utf8_lossy(&stale.stderr)
    );
}

#[test]
fn query_names_only_rejects_workspace_term_discovery() {
    let Some(bin) = option_env!("CARGO_BIN_EXE_rs-harness") else {
        return;
    };
    let root = tempfile::tempdir().expect("temp root");
    fs::create_dir_all(root.path().join("src")).expect("create src");
    fs::write(root.path().join("src/lib.rs"), "pub fn run_install() {}\n").expect("write fixture");

    let output = Command::new(bin)
        .args(["query", "--term", "run_install", "--workspace"])
        .arg(root.path())
        .arg("--names-only")
        .current_dir(root.path())
        .output()
        .expect("run ambiguous query command");

    assert!(
        !output.status.success(),
        "ambiguous command unexpectedly succeeded: stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("query requires an exact --selector"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("asp rust search owner"), "stderr={stderr}");
}

#[test]
fn search_exact_owner_names_only_does_not_scan_workspace_context() {
    let Some(bin) = option_env!("CARGO_BIN_EXE_rs-harness") else {
        return;
    };
    let root = tempfile::tempdir().expect("temp root");
    fs::create_dir_all(root.path().join("src")).expect("create src");
    fs::create_dir_all(root.path().join("tests")).expect("create tests");
    fs::write(
        root.path().join("src/lib.rs"),
        "pub fn target_symbol() {}\n",
    )
    .expect("write source fixture");
    for index in 0..800 {
        fs::write(
            root.path()
                .join("tests")
                .join(format!("fixture_{index}.rs")),
            format!(
                "#[test]\nfn generated_test_{index}() {{\n    assert_eq!({}, {});\n}}\n",
                index, index
            ),
        )
        .expect("write test fixture");
    }

    let query_args = [
        "search",
        "owner",
        "src/lib.rs",
        "items",
        "--query",
        "target_symbol",
        "--names-only",
        "--workspace",
    ];
    let warmup = Command::new(bin)
        .args(query_args)
        .arg(root.path())
        .current_dir(root.path())
        .output()
        .expect("warm exact owner names-only search");
    assert!(
        warmup.status.success(),
        "warm exact owner names-only search failed: stdout={} stderr={}",
        String::from_utf8_lossy(&warmup.stdout),
        String::from_utf8_lossy(&warmup.stderr)
    );

    let started_at = Instant::now();
    let output = Command::new(bin)
        .args(query_args)
        .arg(root.path())
        .current_dir(root.path())
        .output()
        .expect("run exact owner names-only search");
    let elapsed = started_at.elapsed();

    assert!(
        output.status.success(),
        "exact owner names-only search failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "exact owner names-only search scanned too much workspace context: elapsed={elapsed:?}; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("target_symbol"),
        "stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
}
