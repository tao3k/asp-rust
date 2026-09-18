use std::fs;
use std::path::Path;

use asp_rust::{
    AspRustConfig, RustOwnerResponsibility, RustVerificationProfileHint,
    RustVerificationSkillBinding, RustVerificationTaskKind, default_asp_rust_config,
};

pub(super) fn public_api_profile_config() -> AspRustConfig {
    default_asp_rust_config().with_verification_profile_hint(RustVerificationProfileHint::new(
        "src/api.rs",
        [RustOwnerResponsibility::PublicApi],
    ))
}

pub(super) fn latency_sensitive_performance_config() -> AspRustConfig {
    default_asp_rust_config()
        .with_verification_profile_hint(RustVerificationProfileHint::new(
            "src/api.rs",
            [RustOwnerResponsibility::LatencySensitive],
        ))
        .with_verification_skill_binding(
            RustVerificationTaskKind::Performance,
            RustVerificationSkillBinding::new("rust-verification-performance")
                .with_adapter("criterion"),
        )
}

pub(super) fn write_manifest(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
}

pub(super) fn write_api_project(root: &Path) {
    write_manifest(root, "verification-api");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! API owner.\npub fn handle_request() {}\n",
    )
    .expect("write api");
}

pub(super) fn write_external_dependency_project(root: &Path) {
    write_manifest(root, "verification-external");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain owner.\nuse std::fs;\npub fn read_state() {}\n",
    )
    .expect("write domain");
}

pub(super) fn write_branch_project(root: &Path) {
    write_manifest(root, "verification-branch");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod domain;\n").expect("write lib");
    fs::write(
        root.join("src/domain.rs"),
        "//! Domain branch.\nmod alpha;\nmod beta;\nmod gamma;\n",
    )
    .expect("write domain");
    fs::write(root.join("src/domain/alpha.rs"), "//! Alpha.\n").expect("write alpha");
    fs::write(root.join("src/domain/beta.rs"), "//! Beta.\n").expect("write beta");
    fs::write(root.join("src/domain/gamma.rs"), "//! Gamma.\n").expect("write gamma");
}

pub(super) fn write_workspace_with_api_members(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n",
    )
    .expect("write workspace manifest");
    write_workspace_api_member(root, "api");
    write_workspace_api_member(root, "worker");
}

fn write_workspace_api_member(root: &Path, name: &str) {
    let package_root = root.join("crates").join(name);
    fs::create_dir_all(package_root.join("src")).expect("create package src");
    write_manifest(&package_root, name);
    fs::write(
        package_root.join("src/lib.rs"),
        "//! Test crate.\nmod api;\n",
    )
    .expect("write lib");
    fs::write(
        package_root.join("src/api.rs"),
        format!("//! {name} API owner.\npub fn handle_request() {{}}\n"),
    )
    .expect("write api");
}

pub(super) fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let root_text = root.display().to_string();
    rendered
        .replace(&root_text, "$TEMP")
        .replace(&root_text.replace('\\', "/"), "$TEMP")
}
