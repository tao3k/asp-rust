fn install_recipe(justfile: &str) -> String {
    let justfile = justfile.replace("\r\n", "\n");
    justfile
        .split_once("install:\n")
        .expect("Rust provider install recipe")
        .1
        .split_once("\n\n")
        .map_or_else(|| justfile.clone(), |(recipe, _)| recipe.to_owned())
}

#[test]
fn development_install_uses_the_dedicated_optimized_runtime_profile() {
    let descriptor: serde_json::Value = serde_json::from_str(include_str!(
        "../../provider/asp-provider-workspace-install.json"
    ))
    .expect("Rust provider workspace descriptor");
    assert_eq!(
        descriptor.pointer("/workspaceArtifact/root"),
        Some(&serde_json::Value::String(
            "languages/asp-rust/target/provider-runtime/asp-rust".to_owned()
        ))
    );

    let install = install_recipe(include_str!("../../Justfile"));
    assert!(install.contains("cargo build --offline --profile provider-runtime --features"));
    assert!(!install.contains("target/debug"));
    assert!(!install.contains("CARGO_TARGET_DIR"));
}

#[test]
fn install_recipe_is_checkout_line_ending_independent() {
    let justfile = "default:\r\n    just --list\r\n\r\ninstall:\r\n    cargo build --offline --profile provider-runtime --features provider-server\r\n\r\nnext:\r\n";
    assert_eq!(
        install_recipe(justfile),
        "    cargo build --offline --profile provider-runtime --features provider-server"
    );
}
