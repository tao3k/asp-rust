use std::path::Path;

use asp_rust::{
    RustOwnerResponsibility, RustVerificationProfileHint,
    assert_asp_rust_cargo_test_clean_with_config, default_asp_rust_config,
};

#[test]
fn asp_rust_package_is_clean_under_its_own_policy() {
    let mut config = default_asp_rust_config().with_verification_profile_hint(
        RustVerificationProfileHint::new("src/lib.rs", [RustOwnerResponsibility::PublicApi])
            .without_verification_tasks()
            .with_rationale(
                "the external unit target owns full ASP Rust policy while build.rs emits only lightweight contract evidence",
            ),
    );
    config.ignored_dir_names.insert("scenarios".to_string());

    assert_asp_rust_cargo_test_clean_with_config(Path::new(env!("CARGO_MANIFEST_DIR")), &config);
}
