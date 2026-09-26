use std::fs;
use std::path::Path;

use asp_rust::{
    AspRustConfig, RustOwnerResponsibility, RustVerificationDependencySignal,
    default_asp_rust_config,
};

use crate::verification::support::write_manifest;

pub(super) fn project_dependency_signal_config() -> AspRustConfig {
    default_asp_rust_config()
        .with_verification_dependency_signal(RustVerificationDependencySignal::new(
            "axum",
            [
                RustOwnerResponsibility::ExternalDependency,
                RustOwnerResponsibility::AvailabilityCritical,
            ],
        ))
        .with_verification_dependency_signal(RustVerificationDependencySignal::new(
            "arrow-flight",
            [
                RustOwnerResponsibility::ExternalDependency,
                RustOwnerResponsibility::AvailabilityCritical,
            ],
        ))
        .with_verification_dependency_signal(RustVerificationDependencySignal::new(
            "sha2",
            [RustOwnerResponsibility::SecurityBoundary],
        ))
        .with_verification_dependency_signal(RustVerificationDependencySignal::new(
            "rayon",
            [RustOwnerResponsibility::LatencySensitive],
        ))
        .with_verification_dependency_signal(RustVerificationDependencySignal::new(
            "tokio",
            [RustOwnerResponsibility::LatencySensitive],
        ))
}

pub(super) fn write_responsibility_fixture(root: &Path) {
    write_manifest_with_dependencies(
        root,
        "verification-profile-index",
        "[dependencies]\naxum = \"0.1\"\nsha2 = \"0.1\"\nrayon = \"0.1\"\n",
    );
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod api;\nmod auth;\nmod search;\nmod storage;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! API owner.\nuse axum::Router;\npub fn router() -> Router { unimplemented!() }\n",
    )
    .expect("write api");
    fs::write(
        root.join("src/storage.rs"),
        "//! Storage owner.\nuse std::fs;\npub fn load() -> String { String::new() }\n",
    )
    .expect("write storage");
    fs::write(
        root.join("src/auth.rs"),
        "//! Auth owner.\nuse sha2::Digest;\npub fn check_token(token: String) -> bool { !token.is_empty() }\n",
    )
    .expect("write auth");
    fs::write(
        root.join("src/search.rs"),
        "//! Search owner.\nuse rayon::prelude::*;\npub fn search() {}\n",
    )
    .expect("write search");
}

pub(super) fn write_duckdb_fixture(root: &Path) {
    write_manifest_with_dependencies(
        root,
        "verification-profile-duckdb",
        "[dependencies]\nduckdb = \"0.1\"\n",
    );
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod engine;\n").expect("write lib");
    fs::write(
        root.join("src/engine.rs"),
        "//! Engine owner.\nuse duckdb::Connection;\npub fn connect() -> Connection { unimplemented!() }\n",
    )
    .expect("write engine");
}

pub(super) fn write_nested_branch_fixture(root: &Path) {
    write_manifest_with_dependencies(
        root,
        "verification-profile-nested-branch",
        "[dependencies]\naxum = \"0.1\"\n",
    );
    fs::create_dir_all(root.join("src/gateway/studio")).expect("create studio");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod gateway;\n").expect("write lib");
    fs::write(
        root.join("src/gateway/mod.rs"),
        "//! Gateway owner.\nmod studio;\n",
    )
    .expect("write gateway mod");
    fs::write(
        root.join("src/gateway/studio/mod.rs"),
        "//! Studio owner.\nmod handlers;\n",
    )
    .expect("write studio mod");
    fs::write(
        root.join("src/gateway/studio/handlers.rs"),
        "//! Studio handlers.\nuse axum::Router;\npub fn router() -> Router { unimplemented!() }\n",
    )
    .expect("write handlers");
}

pub(super) fn write_facade_export_fixture(root: &Path) {
    write_manifest(root, "verification-profile-facade");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\nmod api;\npub use api::Api;\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! API owner.\npub struct Api;\npub fn new_api() -> Api { Api }\n",
    )
    .expect("write api");
}

pub(super) fn write_branch_fixture(root: &Path) {
    write_manifest_with_dependencies(
        root,
        "verification-profile-branch",
        "[dependencies]\narrow_flight = { package = \"arrow-flight\", version = \"0.1\" }\naxum = \"0.1\"\ntokio = \"0.1\"\n",
    );
    fs::create_dir_all(root.join("src/gateway")).expect("create gateway");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod gateway;\n").expect("write lib");
    fs::write(
        root.join("src/gateway/mod.rs"),
        "//! Gateway owner.\nmod handlers;\n",
    )
    .expect("write gateway mod");
    fs::write(
        root.join("src/gateway/handlers.rs"),
        "//! Gateway handlers.\nuse arrow_flight::FlightInfo;\nuse axum::Router;\nuse tokio::task::JoinHandle;\npub fn router() -> Router { unimplemented!() }\n",
    )
    .expect("write handlers");
}

pub(super) fn write_api_fixture(root: &Path) {
    write_manifest_with_dependencies(
        root,
        "verification-profile-api",
        "[dependencies]\naxum = \"0.1\"\n",
    );
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! API owner.\nuse axum::Router;\npub fn router() -> Router { unimplemented!() }\n",
    )
    .expect("write api");
}

pub(super) fn write_renamed_dependency_fixture(root: &Path) {
    write_manifest_with_dependencies(
        root,
        "verification-profile-renamed-dependency",
        "[dependencies]\nflight = { package = \"arrow-flight\", version = \"0.1\", optional = true, features = [\"flight-sql\"] }\n",
    );
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! API owner.\nuse flight::FlightInfo;\npub fn flight_info() -> FlightInfo { unimplemented!() }\n",
    )
    .expect("write api");
}

pub(super) fn write_manifest_with_dependencies(root: &Path, name: &str, dependencies: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n{dependencies}"
        ),
    )
    .expect("write manifest");
}
