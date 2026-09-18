pub(super) fn fixture_root(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("unit")
        .join("scenarios")
        .join(name)
}

pub(super) fn write_scenario(root: &Path) {
    write_scenario_with_policy_ids(root, r#"["RUST-AGENT-CFG-001"]"#);
}

pub(super) fn write_scenario_with_policy_ids(root: &Path, policy_ids: &str) {
    fs::write(
        root.join("scenario.toml"),
        format!(
            r#"
id = "contract-test"
title = "Contract test"
policy_ids = {policy_ids}
agent_goal = "Keep the scenario understandable."
reference_repositories = ["rust-lang/rust"]
reference_patterns = ["Test fixtures still name the source of the contract pattern"]
inputs = "inputs"
expected = "expected"
"#
        ),
    )
    .expect("write scenario");
}

pub(super) fn write_benchmark(root: &Path, text: &str) {
    fs::write(root.join("benchmark.toml"), text).expect("write benchmark");
}

pub(super) fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = panic.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    "<non-string panic>".to_string()
}
use super::{Path, PathBuf, fs};
