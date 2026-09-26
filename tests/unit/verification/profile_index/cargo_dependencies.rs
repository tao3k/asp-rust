use asp_rust::{
    RustOwnerResponsibility, RustVerificationDependencySignal,
    build_rust_verification_profile_index, build_rust_verification_profile_index_with_config,
    default_asp_rust_config, render_rust_verification_profile_index,
    render_rust_verification_profile_index_json,
};
use tempfile::TempDir;

use super::fixtures::{write_duckdb_fixture, write_renamed_dependency_fixture};

#[test]
fn third_party_dependency_signals_are_cargo_configured() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_duckdb_fixture(root);

    let default_index = build_rust_verification_profile_index(root).expect("profile index");
    let default_candidate = default_index.active_candidates()[0];
    assert_eq!(
        default_candidate.suggested_responsibilities,
        [RustOwnerResponsibility::PublicApi].into()
    );
    let default_rendered = render_rust_verification_profile_index(&default_index);
    assert!(
        default_rendered.contains("unconfigured_dependency_roots=duckdb"),
        "{default_rendered}"
    );
    assert!(
        !default_rendered.contains("|fact: dependency_roots="),
        "{default_rendered}"
    );

    let config = default_asp_rust_config().with_verification_dependency_signal(
        RustVerificationDependencySignal::new(
            "duckdb",
            [
                RustOwnerResponsibility::ExternalDependency,
                RustOwnerResponsibility::Persistence,
            ],
        ),
    );
    let configured_index =
        build_rust_verification_profile_index_with_config(root, &config).expect("profile index");
    let configured_candidate = configured_index.active_candidates()[0];
    assert!(
        configured_candidate
            .suggested_responsibilities
            .contains(&RustOwnerResponsibility::Persistence)
    );
    let configured_rendered = render_rust_verification_profile_index(&configured_index);
    assert!(
        configured_rendered.contains("configured_dependency_roots=duckdb"),
        "{configured_rendered}"
    );
    assert!(
        !configured_rendered.contains("unconfigured_dependency_roots=duckdb"),
        "{configured_rendered}"
    );
    assert!(
        !configured_rendered.contains("|fact: dependency_roots="),
        "{configured_rendered}"
    );
}

#[test]
fn cargo_dependency_signal_matches_renamed_package() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    write_renamed_dependency_fixture(root);

    let config = default_asp_rust_config().with_verification_dependency_signal(
        RustVerificationDependencySignal::new(
            "arrow-flight",
            [
                RustOwnerResponsibility::ExternalDependency,
                RustOwnerResponsibility::AvailabilityCritical,
            ],
        ),
    );
    let index =
        build_rust_verification_profile_index_with_config(root, &config).expect("profile index");
    let candidate = index.active_candidates()[0];
    assert!(
        candidate
            .suggested_responsibilities
            .contains(&RustOwnerResponsibility::AvailabilityCritical)
    );
    let rendered = render_rust_verification_profile_index(&index);
    assert!(
        rendered.contains(
            "configured_dependency_roots=flight->arrow-flight(optional,features=flight-sql)"
        ),
        "{rendered}"
    );
    assert!(!rendered.contains("|fact: dependency_roots="), "{rendered}");

    let json = render_rust_verification_profile_index_json(&index).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse json");
    let evidence = value["candidates"][0]["evidence"]
        .as_array()
        .expect("candidate evidence");
    assert!(evidence.iter().any(|fact| {
        fact["label"] == "dependency_roots"
            && fact["value"] == "flight->arrow-flight(optional,features=flight-sql)"
    }));
}
