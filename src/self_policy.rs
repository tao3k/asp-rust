//! Self-applied project harness mount.
//!
//! Downstream crates should mount the build-script gate so `cargo check` runs
//! parser-native policy. This crate keeps a cargo-test self gate because a
//! package cannot add itself as a build-dependency.

#[cfg(test)]
fn self_apply_harness_config() -> crate::RustHarnessConfig {
    let mut config = crate::default_rust_harness_config().with_verification_profile_hint(
        crate::RustVerificationProfileHint::new(
            "src/lib.rs",
            [crate::RustOwnerResponsibility::PublicApi],
        )
        .without_verification_tasks()
        .with_rationale("self-apply cargo-test gate covers this crate because the preferred cargo-check build gate would require a cyclic self build-dependency"),
    );
    config.ignored_dir_names.insert("scenarios".to_string());
    config
}

#[cfg(test)]
crate::rust_project_harness_cargo_test_gate!(config = self_apply_harness_config());
