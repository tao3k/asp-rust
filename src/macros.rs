//! Public harness mounting macros.

/// Mount the default ASP Rust into a Cargo test target.
#[macro_export]
macro_rules! asp_rust_gate {
    () => {
        #[test]
        fn enforce_asp_rust_gate() {
            $crate::assert_asp_rust_cargo_test_clean(std::path::Path::new(env!(
                "CARGO_MANIFEST_DIR"
            )));
        }
    };
    (advice = allow) => {
        #[test]
        fn enforce_asp_rust_gate() {
            $crate::assert_asp_rust_clean(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));
        }
    };
}

/// Mount the default ASP Rust Dev Gate inside `src/lib.rs` for Cargo tests.
///
/// This is the primary downstream policy entrypoint. Keep `asp-rust` under
/// `[dev-dependencies]`; normal `cargo build` and downstream package consumers
/// then remain free of the policy provider, while `cargo test` executes it.
///
/// Use this from downstream crates through a dev-dependency:
///
/// ```rust,ignore
/// #[cfg(test)]
/// asp_rust::asp_rust_cargo_test_gate!(config = {
///     asp_rust::default_asp_rust_config()
///         .with_verification_profile_hint(
///             asp_rust::RustVerificationProfileHint::new(
///                 "src/lib.rs",
///                 [asp_rust::RustOwnerResponsibility::PublicApi],
///             ),
///         )
/// });
/// ```
///
/// The `#[cfg(test)]` guard keeps normal `cargo build` free of the
/// dev-dependency, while `cargo test` and `cargo test --lib` both execute this
/// policy gate.
///
/// By default, this Dev Gate fails on non-blocking `Info` advice as an
/// agent repair reminder. Use `advice = allow, config = { ... }` only when a
/// crate needs to keep advisory findings visible in rendered reports
/// without failing cargo tests. That config must also call
/// `with_cargo_test_advice_allow_explanation(...)`, so allowing advice remains an
/// auditable project decision instead of a silent harness escape.
#[macro_export]
macro_rules! asp_rust_cargo_test_gate {
    (mode = warn, config = $config:expr) => {
        #[test]
        fn enforce_asp_rust_gate() {
            let config = $config;
            let report = $crate::run_asp_rust_with_config_for_scope(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
                &config,
                $crate::AspRustRunScope::Package,
            )
            .unwrap_or_else(|error| panic!("{error}"));
            let rendered = $crate::render_asp_rust(&report);
            if !rendered.is_empty() {
                eprintln!("{rendered}");
            }
        }
    };
    (mode = deny, config = $config:expr) => {
        $crate::asp_rust_cargo_test_gate!(config = $config);
    };
    (mode = warn) => {
        $crate::asp_rust_cargo_test_gate!(mode = warn, config = $crate::default_asp_rust_config());
    };
    (mode = deny) => {
        $crate::asp_rust_cargo_test_gate!();
    };
    () => {
        mod asp_rust_cargo_test_gate {
            #[test]
            fn enforce_asp_rust_gate() {
                $crate::assert_asp_rust_cargo_test_clean(std::path::Path::new(env!(
                    "CARGO_MANIFEST_DIR"
                )));
            }
        }
    };
    (config = $config:expr) => {
        #[test]
        fn enforce_asp_rust_gate() {
            let config = $config;
            $crate::assert_asp_rust_cargo_test_clean_with_config(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
                &config,
            );
        }
    };
    (advice = allow) => {
        mod asp_rust_cargo_test_gate {
            #[test]
            fn enforce_asp_rust_gate() {
                $crate::assert_asp_rust_clean(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));
            }
        }
    };
    (advice = allow, config = $config:expr) => {
        #[test]
        fn enforce_asp_rust_gate() {
            let config = $config;
            $crate::assert_asp_rust_clean_with_config(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
                &config,
            );
        }
    };
    (advice = fail, config = $config:expr) => {
        $crate::asp_rust_cargo_test_gate!(config = $config);
    };
    (advice = fail) => {
        $crate::asp_rust_cargo_test_gate!();
    };
    ($config:expr) => {
        $crate::asp_rust_cargo_test_gate!(config = $config);
    };
}

/// Mount a workspace policy as one test-only Dev Gate.
///
/// A workspace Build Support crate should wrap this macro with its shared
/// `AspRustWorkspacePolicy`; members invoke that wrapper from a Cargo test
/// target while depending on Build Support only through `[dev-dependencies]`.
#[macro_export]
macro_rules! asp_rust_workspace_dev_gate {
    (mode = warn, policy = $policy:expr) => {
        #[test]
        fn enforce_asp_rust_workspace_dev_gate() {
            let policy = $policy;
            let report = $crate::evaluate_asp_rust_workspace_policy_from_env(&policy);
            for member in report.members {
                let rendered = $crate::render_asp_rust(&member.report);
                if !rendered.is_empty() {
                    eprintln!("[{}]\n{}", member.crate_label, rendered);
                }
            }
        }
    };
    (mode = deny, policy = $policy:expr) => {
        #[test]
        fn enforce_asp_rust_workspace_dev_gate() {
            let policy = $policy;
            $crate::assert_asp_rust_workspace_policy_from_env(&policy);
        }
    };
    (policy = $policy:expr) => {
        $crate::asp_rust_workspace_dev_gate!(mode = deny, policy = $policy);
    };
}

/// Mount an external source-backed harness file from `src/lib.rs` or `src/main.rs`.
#[macro_export]
macro_rules! asp_rust_source_gate {
    ($path:literal) => {
        #[cfg(test)]
        #[path = $path]
        mod asp_rust_gate;
    };
}
