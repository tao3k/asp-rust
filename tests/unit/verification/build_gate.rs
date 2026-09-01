use asp_rust::{
    ASP_RUST_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_ID,
    ASP_RUST_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_VERSION, ASP_RUST_WORKSPACE_BUILD_DAG_SCHEMA_ID,
    ASP_RUST_WORKSPACE_EVIDENCE_GRAPH_RECEIPT_SCHEMA_ID, AspRustDependencyBaseline,
    AspRustDownstreamPolicy, AspRustWorkspaceEvidenceGraphEdgeKind,
    AspRustWorkspaceEvidenceGraphMemberInput, AspRustWorkspaceEvidenceGraphNodeKind,
    AspRustWorkspacePolicy, AspRustWorkspaceTrustLoopStepStatus,
    asp_rust_downstream_policy_receipt, asp_rust_workspace_build_dag,
    asp_rust_workspace_evidence_graph_receipt, assert_asp_rust_dependency_baseline,
    assert_asp_rust_downstream_policy, assert_asp_rust_verification_with_config,
    assert_asp_rust_workspace_policy, assert_asp_rust_workspace_policy_with,
    default_asp_rust_config, render_asp_rust_downstream_policy_receipt_json,
    render_asp_rust_workspace_evidence_graph_receipt_json,
    rust_downstream_verification_gate_guide_markdown,
};
use std::fs;
use tempfile::TempDir;

use crate::verification::support::write_api_project;

const THIN_BUILD_SCRIPT_ADVICE_ALLOW: &str = "scope=downstream policy thin-build-script test; owner=verification::build_gate test; finding_category=advisory public API doc findings; why_safe_now=the test verifies policy object wiring while advisory findings remain visible; cleanup_trigger=remove when the API fixture documents its public item";
const WORKSPACE_POLICY_ADVICE_ALLOW: &str = "scope=workspace common policy test; owner=verification::build_gate test; finding_category=advisory public API doc findings; why_safe_now=the test verifies common workspace policy reuse while advisory findings remain visible; cleanup_trigger=remove when the API fixture documents its public item";
const MEMBER_POLICY_ADVICE_ALLOW: &str = "scope=workspace member override test; owner=verification::build_gate test; finding_category=advisory public API doc findings; why_safe_now=the test verifies member-specific policy override behavior; cleanup_trigger=remove when the member fixture no longer needs an advice override";
const CRITERION_POLICY_ADVICE_ALLOW: &str = "scope=criterion downstream policy test; owner=verification::build_gate test; finding_category=advisory public API doc findings; why_safe_now=the test verifies criterion policy wiring while advisory findings remain visible; cleanup_trigger=remove when the API fixture documents its public item";

#[test]
fn build_gate_verification_requires_reports_for_configured_task_kinds() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);

    let performance_only_config = default_asp_rust_config()
        .with_latency_sensitive_performance_owner(
            "src/api.rs",
            "API request path owns latency-sensitive dispatch",
        );
    assert_asp_rust_verification_with_config(root, &performance_only_config, "api crate");

    let stability_only_config = default_asp_rust_config().with_availability_stability_owner(
        "src/api.rs",
        "API request path must degrade and recover predictably",
    );
    assert_asp_rust_verification_with_config(root, &stability_only_config, "api crate");

    let complete_config = default_asp_rust_config()
        .with_latency_sensitive_performance_owner(
            "src/api.rs",
            "API request path owns latency-sensitive dispatch",
        )
        .with_availability_stability_owner(
            "src/api.rs",
            "API request path must degrade and recover predictably",
        );
    assert_asp_rust_verification_with_config(root, &complete_config, "api crate");
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        return (*message).to_string();
    }
    "<non-string panic>".to_string()
}

#[test]
fn downstream_policy_receipt_projects_verification_and_dependency_contract() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);

    let policy = AspRustDownstreamPolicy::new(
        "example-workspace::api",
        default_asp_rust_config()
            .with_latency_sensitive_performance_owner(
                "src/api.rs",
                "API request path owns latency-sensitive dispatch",
            )
            .with_availability_stability_owner(
                "src/api.rs",
                "API request path must degrade and recover predictably",
            ),
    )
    .with_dependency_baseline(AspRustDependencyBaseline::new().require_git_package(
        "asp-rust",
        "0.1.2",
        "rev=abc123",
    ));

    let receipt = asp_rust_downstream_policy_receipt(root, &policy).expect("policy receipt");

    assert_eq!(
        receipt.schema_id,
        ASP_RUST_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_ID
    );
    assert_eq!(
        receipt.schema_version,
        ASP_RUST_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_VERSION
    );
    assert_eq!(receipt.gate_label, "example-workspace::api");
    assert_eq!(receipt.dependency_baseline_packages.len(), 1);
    assert_eq!(receipt.dependency_baseline_packages[0].name, "asp-rust");
    assert_eq!(
        receipt.dependency_baseline_packages[0].source_contains,
        "rev=abc123"
    );
    assert!(receipt.active_verification_task_count >= 2, "{receipt:?}");
    assert!(receipt.performance_task_count > 0, "{receipt:?}");
    assert!(receipt.stability_task_count > 0, "{receipt:?}");
    assert!(receipt.performance_report_obligation, "{receipt:?}");
    assert!(receipt.stability_report_obligation, "{receipt:?}");
    assert!(receipt.report_obligations.iter().any(|obligation| {
        obligation.key == "performance_index_json"
            && obligation
                .task_kinds
                .iter()
                .any(|kind| kind == "performance")
    }));
    assert!(receipt.report_obligations.iter().any(|obligation| {
        obligation.key == "stability_index_json"
            && obligation.task_kinds.iter().any(|kind| kind == "stability")
    }));

    let json = render_asp_rust_downstream_policy_receipt_json(&receipt).expect("receipt json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid receipt json");
    assert_eq!(
        value["schema_id"],
        ASP_RUST_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_ID
    );
    assert_eq!(value["gate_label"], "example-workspace::api");
    assert_eq!(
        value["dependency_baseline_packages"][0]["source_contains"],
        "rev=abc123"
    );
}

#[test]
fn workspace_evidence_graph_receipt_connects_multi_crate_trust_loop() {
    let temp = TempDir::new().expect("temp dir");
    let workspace_root = temp.path();
    let api_root = workspace_root.join("api");
    let worker_root = workspace_root.join("worker");
    fs::create_dir_all(&api_root).expect("api dir");
    fs::create_dir_all(&worker_root).expect("worker dir");
    write_api_project(&api_root);
    write_api_project(&worker_root);

    let workspace_policy = AspRustWorkspacePolicy::new(
        "example-workspace",
        default_asp_rust_config()
            .with_latency_sensitive_performance_owner(
                "src/api.rs",
                "API request path owns latency-sensitive dispatch",
            )
            .with_availability_stability_owner(
                "src/api.rs",
                "API request path must degrade and recover predictably",
            ),
    )
    .with_dependency_baseline(AspRustDependencyBaseline::new().require_git_package(
        "asp-rust",
        "0.1.2",
        "rev=abc123",
    ));

    let receipt = asp_rust_workspace_evidence_graph_receipt(
        workspace_root,
        workspace_policy.workspace_label(),
        vec![
            AspRustWorkspaceEvidenceGraphMemberInput::new(
                "api",
                &api_root,
                workspace_policy.member_crate("api"),
            ),
            AspRustWorkspaceEvidenceGraphMemberInput::new(
                "worker",
                &worker_root,
                workspace_policy.member_crate("worker"),
            ),
        ],
    )
    .expect("workspace evidence graph receipt");

    assert_eq!(
        receipt.schema_id,
        ASP_RUST_WORKSPACE_EVIDENCE_GRAPH_RECEIPT_SCHEMA_ID
    );
    assert_eq!(receipt.workspace_label, "example-workspace");
    assert_eq!(receipt.summary.member_crate_count, 2);
    assert_eq!(receipt.summary.dependency_baseline_package_count, 2);
    assert!(receipt.summary.active_verification_task_count >= 4);
    assert!(receipt.summary.performance_task_count >= 2);
    assert!(receipt.summary.stability_task_count >= 2);
    assert!(receipt.summary.report_obligation_count >= 4);
    assert_eq!(receipt.summary.security_task_count, 0);
    assert_eq!(receipt.members.len(), 2);
    assert!(
        receipt
            .nodes
            .iter()
            .any(|node| node.kind == AspRustWorkspaceEvidenceGraphNodeKind::Workspace)
    );
    assert!(
        receipt
            .nodes
            .iter()
            .any(|node| { node.kind == AspRustWorkspaceEvidenceGraphNodeKind::MemberCrate })
    );
    assert!(
        receipt
            .nodes
            .iter()
            .any(|node| node.kind
                == AspRustWorkspaceEvidenceGraphNodeKind::DependencyBaselinePackage)
    );
    assert!(
        receipt
            .nodes
            .iter()
            .any(|node| node.kind == AspRustWorkspaceEvidenceGraphNodeKind::ReportObligation)
    );
    assert!(
        receipt
            .edges
            .iter()
            .any(|edge| edge.kind
                == AspRustWorkspaceEvidenceGraphEdgeKind::RequiresDependencyBaseline)
    );
    assert!(
        receipt
            .edges
            .iter()
            .any(|edge| edge.kind == AspRustWorkspaceEvidenceGraphEdgeKind::RequiresReport)
    );
    assert!(receipt.trust_loop_steps.iter().any(|step| {
        step.key == "performance_stability_reports"
            && step.status == AspRustWorkspaceTrustLoopStepStatus::Required
    }));
    assert!(
        receipt
            .trust_loop_steps
            .iter()
            .any(|step| step.key == "security_review"
                && step.status == AspRustWorkspaceTrustLoopStepStatus::NotConfigured)
    );
    assert!(
        receipt
            .trust_loop_steps
            .iter()
            .any(|step| step.key == "build_gate"
                && step.status == AspRustWorkspaceTrustLoopStepStatus::Enforced)
    );

    let json = render_asp_rust_workspace_evidence_graph_receipt_json(&receipt).expect("json");
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");
    assert_eq!(
        value["schema_id"],
        ASP_RUST_WORKSPACE_EVIDENCE_GRAPH_RECEIPT_SCHEMA_ID
    );
    assert_eq!(value["summary"]["member_crate_count"], 2);
    assert_eq!(
        value["trust_loop_steps"]
            .as_array()
            .expect("trust loop array")
            .len(),
        6
    );
}

#[test]
fn downstream_verification_gate_guide_classifies_api_and_cli_surfaces() {
    let guide = rust_downstream_verification_gate_guide_markdown();

    assert!(guide.contains("## Crate Layout"), "{guide}");
    assert!(guide.contains("## Classification"), "{guide}");
    assert!(guide.contains("workspace semantic contract"), "{guide}");
    assert!(guide.contains("shared Build Support gate"), "{guide}");
    assert!(
        guide.contains("assert_asp_rust_workspace_policy_from_env"),
        "{guide}"
    );
    assert!(
        guide.contains("CLI quick check and observation surface"),
        "{guide}"
    );
    assert!(guide.contains("harness/mod.rs"), "{guide}");
    assert!(guide.contains("owners.rs"), "{guide}");
    assert!(guide.contains("verification.rs"), "{guide}");
    assert!(guide.contains("AspRustDownstreamPolicy"), "{guide}");
    assert!(
        guide.contains("assert_asp_rust_downstream_policy_from_env"),
        "{guide}"
    );
    assert!(
        guide.contains("assert_asp_rust_verification_from_env_with_config"),
        "{guide}"
    );
    assert!(guide.contains("AspRustDependencyBaseline"), "{guide}");
    assert!(
        guide.contains("assert_asp_rust_dependency_baseline"),
        "{guide}"
    );
    assert!(guide.contains("dependencies.rs"), "{guide}");
    assert!(guide.contains("[asp-rust-dependency-guidance]"), "{guide}");
    assert!(
        guide.contains("with_availability_stability_owner"),
        "{guide}"
    );
    assert!(
        guide.contains("Do not expose full verification as a standalone downstream CLI command"),
        "{guide}"
    );
    assert!(guide.contains("## Workspace Layout"), "{guide}");
    assert!(guide.contains("AspRustWorkspacePolicy"), "{guide}");
    assert!(guide.contains("member_crate_with_config"), "{guide}");
    assert!(
        guide.contains("A workspace should own common policy once"),
        "{guide}"
    );
    assert!(
        guide.contains("`cargo test` automatically triggers that shared Cargo unit"),
        "{guide}"
    );
    assert!(guide.contains("[asp-rust-agent-guidance]"), "{guide}");
}

#[test]
fn downstream_policy_object_keeps_build_script_thin() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);

    let policy = AspRustDownstreamPolicy::new(
        "api crate",
        default_asp_rust_config()
            .with_cargo_check_advice_allow_explanation(THIN_BUILD_SCRIPT_ADVICE_ALLOW)
            .with_latency_sensitive_performance_owner(
                "src/api.rs",
                "API request path owns latency-sensitive dispatch",
            )
            .with_availability_stability_owner(
                "src/api.rs",
                "API request path must degrade and recover predictably",
            ),
    );

    let report = assert_asp_rust_downstream_policy(root, &policy);

    assert_eq!(policy.gate_label(), "api crate");
    assert!(report.is_clean(), "{report:?}");
}

#[test]
fn downstream_policy_requires_criterion_bench_manifest_when_performance_adapter_is_enabled() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);

    let policy = criterion_downstream_policy("api crate");

    let missing_bench = std::panic::catch_unwind(|| {
        assert_asp_rust_downstream_policy(root, &policy);
    })
    .expect_err("criterion performance gate should require a bench manifest");
    let message = panic_message(missing_bench);

    assert!(message.contains("RUST-AGENT-PROJECT-010"), "{message}");
    assert!(
        message.contains("Performance verification skill lacks Cargo bench target"),
        "{message}"
    );
    assert!(
        message.contains("add a Criterion, Divan, or iai-callgrind [[bench]] target"),
        "{message}"
    );
    assert!(
        message.contains("benchmark framework dev-dependency"),
        "{message}"
    );
    assert!(
        message.contains("record benchmark runs through performance receipts"),
        "{message}"
    );
}

#[test]
fn downstream_policy_accepts_criterion_bench_manifest_contract() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    write_criterion_bench_contract(root);

    let policy = criterion_downstream_policy("api crate");
    let report = assert_asp_rust_downstream_policy(root, &policy);

    assert!(report.is_clean(), "{report:?}");
}

#[test]
fn downstream_member_policy_does_not_apply_performance_contract_to_siblings() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path();
    let benchmarked_member = workspace.join("benchmarked-member");
    let plain_member = workspace.join("plain-member");
    fs::create_dir_all(&benchmarked_member).expect("create benchmarked member");
    fs::create_dir_all(&plain_member).expect("create plain member");
    fs::write(
        workspace.join("Cargo.toml"),
        "[workspace]\nmembers = [\"benchmarked-member\", \"plain-member\"]\nresolver = \"3\"\n",
    )
    .expect("write workspace manifest");
    write_api_project(&benchmarked_member);
    write_criterion_bench_contract(&benchmarked_member);
    write_api_project(&plain_member);

    let policy = criterion_downstream_policy("benchmarked member");
    let report = assert_asp_rust_downstream_policy(&benchmarked_member, &policy);

    assert!(report.is_clean(), "{report:?}");
    assert!(
        report.workspace_member_scopes.is_empty(),
        "member policy must not project its verification contract onto siblings: {report:?}"
    );
}

#[test]
fn downstream_policy_asserts_dependency_baseline_from_cargo_lock() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    write_cargo_lock(
        root,
        r#"
version = 4

[[package]]
name = "asp-rust"
version = "0.1.2"
source = "git+https://github.com/tao3k/agent-semantic-protocols?rev=abc123#abc123"
"#,
    );

    let policy = AspRustDownstreamPolicy::new(
        "api crate",
        default_asp_rust_config()
            .with_cargo_check_advice_allow_explanation(THIN_BUILD_SCRIPT_ADVICE_ALLOW)
            .with_latency_sensitive_performance_owner(
                "src/api.rs",
                "API request path owns latency-sensitive dispatch",
            )
            .with_availability_stability_owner(
                "src/api.rs",
                "API request path must degrade and recover predictably",
            ),
    )
    .with_dependency_baseline(AspRustDependencyBaseline::new().require_git_package(
        "asp-rust",
        "0.1.2",
        "rev=abc123",
    ));

    let report = assert_asp_rust_downstream_policy(root, &policy);

    assert!(report.is_clean(), "{report:?}");
    assert_eq!(
        policy
            .dependency_baseline()
            .expect("dependency baseline")
            .packages()
            .len(),
        1
    );
}

#[test]
fn dependency_baseline_reports_duplicate_or_stale_entries_with_agent_guidance() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);
    write_cargo_lock(
        root,
        r#"
version = 4

[[package]]
name = "asp-rust"
version = "0.1.1"
source = "git+https://github.com/tao3k/agent-semantic-protocols?rev=old#old"

[[package]]
name = "asp-rust"
version = "0.1.2"
source = "git+https://github.com/tao3k/agent-semantic-protocols?rev=new#new"
"#,
    );
    let baseline =
        AspRustDependencyBaseline::new().require_git_package("asp-rust", "0.1.2", "rev=new");

    let duplicate = std::panic::catch_unwind(|| {
        assert_asp_rust_dependency_baseline(root, &baseline, "api crate");
    })
    .expect_err("duplicate lockfile package should fail baseline");
    let message = panic_message(duplicate);
    assert!(
        message.contains("requires exactly one Cargo.lock entry"),
        "{message}"
    );
    assert!(
        message.contains("[asp-rust-dependency-guidance]"),
        "{message}"
    );
    assert!(message.contains("cargo tree -i <package>"), "{message}");
    assert!(message.contains("rev=old"), "{message}");
    assert!(message.contains("rev=new"), "{message}");
}

#[test]
fn workspace_policy_reuses_common_config_for_member_crates() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_api_project(root);

    let workspace_policy = AspRustWorkspacePolicy::new(
        "example-workspace",
        default_asp_rust_config()
            .with_cargo_check_advice_allow_explanation(WORKSPACE_POLICY_ADVICE_ALLOW)
            .with_latency_sensitive_performance_owner(
                "src/api.rs",
                "workspace public API dispatch is latency-sensitive",
            )
            .with_availability_stability_owner(
                "src/api.rs",
                "workspace public API dispatch must degrade and recover predictably",
            ),
    );

    let api_policy = workspace_policy.member_crate("api");
    let report = assert_asp_rust_downstream_policy(root, &api_policy);

    assert_eq!(workspace_policy.workspace_label(), "example-workspace");
    assert_eq!(api_policy.gate_label(), "example-workspace::api");
    assert!(report.is_clean(), "{report:?}");

    let specialized_policy = workspace_policy
        .member_crate_with_config("api-specialized", |config| {
            config.with_cargo_check_advice_allow_explanation(MEMBER_POLICY_ADVICE_ALLOW)
        });

    assert_eq!(
        specialized_policy.gate_label(),
        "example-workspace::api-specialized"
    );
    assert_eq!(
        workspace_policy
            .config()
            .cargo_check_advice_allow_explanation
            .as_deref(),
        Some(WORKSPACE_POLICY_ADVICE_ALLOW)
    );
    assert_eq!(
        specialized_policy
            .config()
            .cargo_check_advice_allow_explanation
            .as_deref(),
        Some(MEMBER_POLICY_ADVICE_ALLOW)
    );
}

#[test]
fn build_dag_deduplicates_diamond_and_orders_dependencies_first() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path();
    write_dependency_graph_workspace(workspace, false);
    let config = default_asp_rust_config()
        .with_cargo_check_advice_allow_explanation(WORKSPACE_POLICY_ADVICE_ALLOW);
    let build_dag = asp_rust_workspace_build_dag(workspace, &config).expect("workspace Build DAG");

    assert_eq!(build_dag.schema_id, ASP_RUST_WORKSPACE_BUILD_DAG_SCHEMA_ID);
    assert_eq!(
        build_dag
            .packages
            .iter()
            .map(|package| package.package_name.as_str())
            .collect::<Vec<_>>(),
        ["shared", "left", "right", "app"]
    );
    assert_eq!(
        build_dag
            .packages
            .iter()
            .filter(|package| package.package_name == "shared")
            .count(),
        1,
        "diamond dependency must appear once"
    );

    let workspace_policy = AspRustWorkspacePolicy::new("fixture", config);
    let report = assert_asp_rust_workspace_policy(workspace, &workspace_policy);
    assert_eq!(report.members.len(), 4);
    assert_eq!(
        report
            .members
            .iter()
            .filter(|member| member.crate_label == "shared")
            .count(),
        1,
        "diamond dependency policy must execute once"
    );

    let mut configured = Vec::new();
    let report = assert_asp_rust_workspace_policy_with(
        workspace,
        &workspace_policy,
        |package_name, config| {
            configured.push(package_name.to_string());
            config
        },
    );
    assert_eq!(configured, ["shared", "left", "right", "app"]);
    assert_eq!(report.members.len(), 4);
}

#[test]
fn build_dag_uses_workspace_members_and_excludes_non_members() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path();
    write_dependency_graph_workspace(workspace, false);
    let config = default_asp_rust_config()
        .with_cargo_check_advice_allow_explanation(WORKSPACE_POLICY_ADVICE_ALLOW);
    let build_dag = asp_rust_workspace_build_dag(workspace, &config).expect("workspace Build DAG");
    assert_eq!(
        build_dag
            .packages
            .iter()
            .map(|package| package.package_name.as_str())
            .collect::<Vec<_>>(),
        ["shared", "left", "right", "app"]
    );
    assert!(
        build_dag
            .packages
            .iter()
            .all(|package| package.package_name != "excluded"),
        "excluded nested package must not enter the ASP instance Build DAG"
    );
}

#[test]
fn build_dag_rejects_local_cycle_before_policy_execution() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path();
    write_dependency_graph_workspace(workspace, true);
    let error = asp_rust_workspace_build_dag(workspace, &default_asp_rust_config())
        .expect_err("local dependency cycle must fail closed");
    assert!(error.contains("contains a cycle"), "{error}");
}

fn write_dependency_graph_workspace(workspace: &std::path::Path, cycle: bool) {
    fs::write(
        workspace.join("Cargo.toml"),
        "[workspace]\nmembers=['app', 'left', 'right', 'shared']\nexclude=['excluded']\nresolver='3'\n",
    )
    .expect("write workspace manifest");
    write_dependency_graph_package(workspace, "shared", cycle.then_some("app"));
    write_dependency_graph_package(workspace, "left", Some("shared"));
    write_dependency_graph_package(workspace, "right", Some("shared"));
    write_dependency_graph_package(workspace, "excluded", None);
    let app = workspace.join("app");
    fs::create_dir_all(app.join("src")).expect("create app source");
    fs::write(
        app.join("Cargo.toml"),
        "[package]\nname='app'\nversion='0.1.0'\nedition='2024'\n\n[dependencies]\nleft={path='../left'}\nright={path='../right'}\nexcluded={path='../excluded'}\n",
    )
    .expect("write app manifest");
    fs::write(app.join("src/lib.rs"), "//! App package.\n").expect("write app source");
}

fn write_dependency_graph_package(
    workspace: &std::path::Path,
    name: &str,
    dependency: Option<&str>,
) {
    let root = workspace.join(name);
    fs::create_dir_all(root.join("src")).expect("create package source");
    let dependency = dependency.map_or_else(String::new, |dependency| {
        format!("\n[dependencies]\n{dependency}={{path='../{dependency}'}}\n")
    });
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname='{name}'\nversion='0.1.0'\nedition='2024'\n{dependency}"),
    )
    .expect("write package manifest");
    fs::write(root.join("src/lib.rs"), format!("//! {name} package.\n"))
        .expect("write package source");
}

#[test]
fn workspace_policy_shares_dependency_baseline_with_member_crates() {
    let workspace_policy =
        AspRustWorkspacePolicy::new("example-workspace", default_asp_rust_config())
            .with_dependency_baseline(AspRustDependencyBaseline::new().require_git_package(
                "asp-rust",
                "0.1.2",
                "rev=abc123",
            ));

    let api_policy = workspace_policy.member_crate("api");

    assert_eq!(
        workspace_policy
            .dependency_baseline()
            .expect("workspace baseline")
            .packages()[0]
            .source_contains(),
        "rev=abc123"
    );
    assert_eq!(
        api_policy
            .dependency_baseline()
            .expect("member baseline")
            .packages()[0]
            .version(),
        "0.1.2"
    );
}

fn write_cargo_lock(root: &std::path::Path, contents: &str) {
    fs::write(root.join("Cargo.lock"), contents.trim_start()).expect("write Cargo.lock");
}

fn criterion_downstream_policy(gate_label: &str) -> AspRustDownstreamPolicy {
    AspRustDownstreamPolicy::new(
        gate_label,
        default_asp_rust_config()
            .with_cargo_check_advice_allow_explanation(CRITERION_POLICY_ADVICE_ALLOW)
            .with_criterion_performance_verification()
            .with_latency_sensitive_performance_owner(
                "src/api.rs",
                "API request path owns latency-sensitive dispatch",
            )
            .with_availability_stability_owner(
                "src/api.rs",
                "API request path must degrade and recover predictably",
            ),
    )
}

fn write_criterion_bench_contract(root: &std::path::Path) {
    let manifest_path = root.join("Cargo.toml");
    let mut manifest = fs::read_to_string(&manifest_path).expect("read Cargo.toml");
    manifest.push_str(
        r#"

[dev-dependencies]
criterion = "0.8"

[[bench]]
name = "api_hot_path"
path = "benches/api_hot_path.rs"
harness = false
"#,
    );
    fs::write(manifest_path, manifest).expect("write Cargo.toml");
    fs::create_dir(root.join("benches")).expect("create benches");
    fs::write(
        root.join("benches/api_hot_path.rs"),
        r#"//! Criterion bench target.

use criterion::{criterion_group, criterion_main, Criterion};

fn api_hot_path(c: &mut Criterion) {
    c.bench_function("api_hot_path", |b| b.iter(|| 1));
}

criterion_group!(benches, api_hot_path);
criterion_main!(benches);
"#,
    )
    .expect("write bench");
}
