use std::ffi::OsString;

#[test]
fn exact_query_accepts_source_snapshot_authority_without_projection_fields() {
    let command = super::parse_query(
        [
            "--selector",
            "rust://src/lib.rs#item/function/run",
            "--source-snapshot-envelope",
            "snapshot.json",
        ]
        .into_iter()
        .map(OsString::from),
    )
    .expect("parse typed exact projection query");

    let super::QueryCommand::ExactSource(options) = command else {
        panic!("typed exact projection must route to ExactSource");
    };
    assert_eq!(
        options.source_snapshot_envelope.as_deref(),
        Some(std::path::Path::new("snapshot.json"))
    );
}

#[test]
fn removed_search_route_flags_are_not_reintroduced_by_projection_authority() {
    let result = super::parse_query(
        ["--term", "run", "--asp-provider-id", "rs-harness"]
            .into_iter()
            .map(OsString::from),
    );
    let Err(error) = result else {
        panic!("typed authority must not leak into search routing");
    };
    assert!(error.contains("unexpected argument '--term'"), "{error}");
}

#[test]
fn exact_query_rejects_position_and_path_selectors_before_snapshot_materialization() {
    for selector in [
        "rust://src/lib.rs#syntax:identifier/declaration.name@1:1",
        "src/lib.rs",
        "owner:src/lib.rs",
    ] {
        let result = super::parse_query(
            ["--selector", selector, "--projection", "source"]
                .into_iter()
                .map(OsString::from),
        );
        let Err(error) = result else {
            panic!("non-item selector must not enter exact source projection: {selector}");
        };
        assert!(
            error.contains("rust query requires an exact --selector"),
            "selector={selector} error={error}"
        );
        assert!(
            !error.contains("wrapper-snapshot-required"),
            "selector={selector} error={error}"
        );
    }
}

#[test]
fn exact_query_rejects_from_hook_control_plane_arguments() {
    let result = super::parse_query(
        [
            "--from-hook",
            "item-skeleton",
            "--selector",
            "rust://src/lib.rs#item/function/run",
            "--projection",
            "source",
        ]
        .into_iter()
        .map(OsString::from),
    );
    let Err(error) = result else {
        panic!("provider query must reject client control-plane arguments");
    };
    assert!(
        error.contains("unexpected argument '--from-hook'"),
        "{error}"
    );
}
