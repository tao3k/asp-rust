use std::collections::BTreeSet;
use std::fs;

use asp_rust::{
    RustDeterminismReadinessCategory, RustDeterminismReadinessInput,
    RustDeterminismReadinessStatus, build_rust_determinism_readiness,
};
use tempfile::TempDir;

#[test]
fn p3_determinism_readiness_detects_direct_sources() {
    let temp = TempDir::new().expect("temp dir");
    let src = temp.path().join("src");
    fs::create_dir_all(&src).expect("create src");
    fs::write(
        src.join("lib.rs"),
        "static CACHE: std::sync::OnceLock<String> = std::sync::OnceLock::new();\n\
pub fn sample() {\n\
    let _ = std::time::SystemTime::now();\n\
    let _ = std::env::var(\"HOME\");\n\
    let _ = std::fs::read_to_string(\"Cargo.toml\");\n\
    let _ = rand::random::<u64>();\n\
}\n",
    )
    .expect("write lib");

    let readiness = build_rust_determinism_readiness(RustDeterminismReadinessInput {
        project_root: temp.path().to_path_buf(),
        include_tests: false,
    })
    .expect("readiness");

    assert_eq!(
        readiness.status,
        RustDeterminismReadinessStatus::NeedsInjection
    );
    let categories = readiness
        .observations
        .iter()
        .map(|observation| observation.category)
        .collect::<BTreeSet<_>>();
    assert!(categories.contains(&RustDeterminismReadinessCategory::Clock));
    assert!(categories.contains(&RustDeterminismReadinessCategory::Environment));
    assert!(categories.contains(&RustDeterminismReadinessCategory::Filesystem));
    assert!(categories.contains(&RustDeterminismReadinessCategory::Random));
    assert!(categories.contains(&RustDeterminismReadinessCategory::GlobalState));
    assert_eq!(readiness.project.root, std::path::PathBuf::from("."));
    assert!(
        readiness
            .suggestions
            .iter()
            .any(|suggestion| suggestion.category == RustDeterminismReadinessCategory::Clock)
    );
}
