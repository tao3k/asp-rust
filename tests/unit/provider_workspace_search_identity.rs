//! Unit role: provider-owned workspace snapshot compatibility vectors.

use asp_rust::provider_workspace_search_identity::WorkspaceSnapshot;

#[test]
fn matches_content_identity_workspace_snapshot_vectors() {
    let vectors = [
        (
            Vec::<(&str, &str)>::new(),
            "c96be11df61a2a474535fa2bcf48204bdb033a3768264da92abceea2025f5ce5",
        ),
        (
            vec![("src/lib.rs", "00")],
            "b4b3ffff9e452dd4a453f1420f2e294b1efe170e930a9bde595b4b93c83a009f",
        ),
        (
            vec![("src/lib.rs", "00"), ("src/main.rs", "11")],
            "bc9a0053fdccdf68bd1d3a1f8e2bb6e0e876ad8238092d3eff971dcf05e96675",
        ),
        (
            vec![("src/main.rs", "11"), ("src/lib.rs", "00")],
            "bc9a0053fdccdf68bd1d3a1f8e2bb6e0e876ad8238092d3eff971dcf05e96675",
        ),
    ];

    for (files, expected) in vectors {
        assert_eq!(
            WorkspaceSnapshot::from_file_hashes(files).root_digest(),
            expected
        );
    }
}

#[test]
fn normalizes_equivalent_snapshot_paths() {
    let canonical = WorkspaceSnapshot::from_file_hashes([("src/lib.rs", "00")]);
    for path in [
        "./src/lib.rs",
        "src/./lib.rs",
        "src\\lib.rs",
        "tmp/../src/lib.rs",
    ] {
        assert_eq!(
            WorkspaceSnapshot::from_file_hashes([(path, "00")]),
            canonical
        );
    }
}
