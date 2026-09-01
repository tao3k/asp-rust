use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use crate::{AspRustConfig, AspRustDownstreamPolicyReceipt, AspRustReport, RustVerificationPlan};

use super::{
    ASP_RUST_BUILD_GATE_CACHE_SCHEMA_ID, ASP_RUST_BUILD_GATE_CACHE_SCHEMA_VERSION,
    AspRustBuildGateCacheRecord, AspRustBuildGateSnapshot, BuildGateCacheContract,
    TEMP_FILE_SEQUENCE, build_gate_cache_payload_digest, build_gate_cache_root, cache_path,
    content_digest, load_build_gate_cache, reset_snapshot_file_read_count,
    snapshot_build_gate_inputs, snapshot_build_gate_inputs_with_cache, snapshot_file_read_count,
    store_build_gate_cache,
};

fn temp_root(name: &str) -> PathBuf {
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "asp-rust-build-gate-cache-{name}-{}-{sequence}",
        std::process::id()
    ))
}

fn empty_record(
    cache_key: String,
    snapshot: AspRustBuildGateSnapshot,
) -> AspRustBuildGateCacheRecord {
    let receipt = AspRustDownstreamPolicyReceipt {
        schema_id: "test.receipt".to_string(),
        schema_version: "1".to_string(),
        gate_label: "test".to_string(),
        dependency_baseline_packages: Vec::new(),
        active_verification_task_count: 0,
        performance_task_count: 0,
        stability_task_count: 0,
        performance_report_obligation: false,
        stability_report_obligation: false,
        report_obligations: Vec::new(),
    };
    AspRustBuildGateCacheRecord {
        schema_id: ASP_RUST_BUILD_GATE_CACHE_SCHEMA_ID.to_string(),
        schema_version: ASP_RUST_BUILD_GATE_CACHE_SCHEMA_VERSION.to_string(),
        cache_key,
        snapshot,
        payload_digest: build_gate_cache_payload_digest(
            &AspRustReport {
                modules: Vec::new(),
                findings: Vec::new(),
                invariant_candidates: Vec::new(),
                root_paths: Vec::new(),
                blocking_severities: BTreeSet::new(),
                project_resolution: None,
                workspace_member_scopes: Vec::new(),
            },
            &RustVerificationPlan::default(),
            &receipt,
            &[],
        )
        .expect("digest empty cache payload"),
        report: AspRustReport {
            modules: Vec::new(),
            findings: Vec::new(),
            invariant_candidates: Vec::new(),
            root_paths: Vec::new(),
            blocking_severities: BTreeSet::new(),
            project_resolution: None,
            workspace_member_scopes: Vec::new(),
        },
        verification_plan: RustVerificationPlan::default(),
        downstream_policy_receipt: receipt,
        dependency_baseline_receipts: Vec::new(),
    }
}

#[test]
fn snapshot_hashes_complete_content_before_parse() {
    let root = temp_root("snapshot");
    fs::create_dir_all(root.join("src")).expect("create source root");
    fs::write(root.join("Cargo.toml"), "[package]\nname='fixture'\n").expect("write manifest");
    fs::write(root.join("src/lib.rs"), "pub fn old() {}\n").expect("write source");
    let config = AspRustConfig::default();
    let first = snapshot_build_gate_inputs(&root, &config).expect("first snapshot");
    fs::write(root.join("src/lib.rs"), "pub fn new() {}\n").expect("change source");
    let second = snapshot_build_gate_inputs(&root, &config).expect("second snapshot");
    assert_ne!(first.digest, second.digest);
    assert_eq!(first.file_count, 2);
    assert_eq!(second.file_count, 2);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn package_snapshot_excludes_nested_workspace_member_inputs() {
    let root = temp_root("package-atomic-snapshot");
    let nested = root.join("nested-member");
    fs::create_dir_all(nested.join("src")).expect("create nested member");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='root-package'\nversion='0.1.0'\n\
         [lib]\npath='lib.rs'\n\
         [workspace]\nmembers=['nested-member']\n",
    )
    .expect("write root manifest");
    fs::write(root.join("lib.rs"), "pub fn root() {}\n").expect("write root source");
    fs::write(
        nested.join("Cargo.toml"),
        "[package]\nname='nested-member'\nversion='0.1.0'\n",
    )
    .expect("write nested manifest");
    fs::write(nested.join("src/lib.rs"), "pub fn before() {}\n").expect("write nested source");
    let config = AspRustConfig::default();

    let before = snapshot_build_gate_inputs(&root, &config).expect("snapshot before");
    fs::write(nested.join("src/lib.rs"), "pub fn after() {}\n").expect("change nested source");
    let after = snapshot_build_gate_inputs(&root, &config).expect("snapshot after");

    assert_eq!(before, after, "nested member invalidated package cache");
    assert_eq!(
        before
            .files
            .iter()
            .map(|file| file.path.as_path())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([Path::new("Cargo.toml"), Path::new("lib.rs")])
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn generic_build_gate_cache_requires_explicit_state_home() {
    let root = temp_root("state-home");
    let state_home = root.join("state");
    let project = root.join("project");
    fs::create_dir_all(&project).expect("create project root");

    assert_eq!(build_gate_cache_root(&project, None), None);
    let cache_root = build_gate_cache_root(&project, Some(state_home.clone().into_os_string()))
        .expect("resolve State Home build-gate cache");
    assert!(cache_root.starts_with(state_home.join("runtime/build-gates/rph/bg/v1")));
    assert!(!cache_root.starts_with(root.join(".agent-semantic-protocols/cache")));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn cached_snapshot_reads_only_changed_files() {
    let root = temp_root("incremental-snapshot");
    let cache = root.join("cache");
    fs::create_dir_all(root.join("src")).expect("create source root");
    fs::write(root.join("Cargo.toml"), "[package]\nname='fixture'\n").expect("write manifest");
    fs::write(root.join("src/lib.rs"), "mod value;\n").expect("write facade");
    fs::write(root.join("src/value.rs"), "pub fn value() -> usize { 1 }\n")
        .expect("write implementation");
    let config = AspRustConfig::default();

    reset_snapshot_file_read_count();
    let cold =
        snapshot_build_gate_inputs_with_cache(&root, &config, Some(&cache)).expect("cold snapshot");
    assert_eq!(snapshot_file_read_count(), 3);

    reset_snapshot_file_read_count();
    let warm =
        snapshot_build_gate_inputs_with_cache(&root, &config, Some(&cache)).expect("warm snapshot");
    assert_eq!(warm, cold);
    assert_eq!(
        snapshot_file_read_count(),
        0,
        "warm snapshot must reuse every unchanged file digest"
    );

    fs::write(
        root.join("src/value.rs"),
        "pub fn value() -> usize { 22 }\n",
    )
    .expect("change one implementation");
    reset_snapshot_file_read_count();
    let changed = snapshot_build_gate_inputs_with_cache(&root, &config, Some(&cache))
        .expect("changed snapshot");
    assert_ne!(changed.digest, cold.digest);
    assert_eq!(
        snapshot_file_read_count(),
        1,
        "one changed file must not re-read the complete package"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn corrupt_cache_is_a_cold_miss_and_atomic_record_round_trips() {
    let root = temp_root("round-trip");
    fs::create_dir_all(&root).expect("create cache root");
    let snapshot = AspRustBuildGateSnapshot {
        digest: content_digest(b"[]"),
        file_count: 0,
        byte_count: 0,
        files: Vec::new(),
    };
    let key = "stable-key".to_string();
    let record = empty_record(key.clone(), snapshot);
    store_build_gate_cache(&root, &record).expect("store cache record");
    assert_eq!(load_build_gate_cache(&root, &key), Some(record.clone()));
    for rejected in [
        {
            let mut record = record.clone();
            record.schema_id.push_str(".wrong");
            record
        },
        {
            let mut record = record.clone();
            record.schema_version = "wrong".to_string();
            record
        },
        {
            let mut record = record.clone();
            record.cache_key = "wrong".to_string();
            record
        },
        {
            let mut record = record.clone();
            record.snapshot.file_count += 1;
            record
        },
        {
            let mut record = record.clone();
            record.snapshot.byte_count += 1;
            record
        },
    ] {
        fs::write(
            cache_path(&root, &key),
            serde_json::to_vec(&rejected).expect("serialize rejected cache record"),
        )
        .expect("write rejected cache record");
        assert_eq!(load_build_gate_cache(&root, &key), None);
    }
    let mut tampered = record;
    tampered.report.root_paths.push(PathBuf::from("tampered"));
    fs::write(
        cache_path(&root, &key),
        serde_json::to_vec(&tampered).expect("serialize tampered cache record"),
    )
    .expect("write parseable tampered cache record");
    assert_eq!(load_build_gate_cache(&root, &key), None);
    fs::write(cache_path(&root, &key), b"{not-json").expect("corrupt cache record");
    assert_eq!(load_build_gate_cache(&root, &key), None);
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn symlinked_file_is_not_a_snapshot_input() {
    let base = temp_root("symlink-file");
    let root = base.join("project");
    fs::create_dir_all(root.join("src")).expect("create source root");
    fs::write(root.join("src/lib.rs"), "pub fn local() {}\n").expect("write local source");
    let target = base.join("external.rs");
    fs::write(&target, "pub fn first() {}\n").expect("write external source");
    std::os::unix::fs::symlink(&target, root.join("src/external.rs"))
        .expect("create source symlink");
    let config = AspRustConfig::default();
    let first = snapshot_build_gate_inputs(&root, &config).expect("first snapshot");
    fs::write(&target, "pub fn second() {}\n").expect("change external source");
    let second = snapshot_build_gate_inputs(&root, &config).expect("second snapshot");
    assert_eq!(first, second);
    assert!(
        first
            .files
            .iter()
            .all(|file| file.path != Path::new("src/external.rs"))
    );
    let _ = fs::remove_dir_all(base);
}

#[cfg(unix)]
#[test]
fn symlinked_directory_is_not_a_snapshot_input() {
    let base = temp_root("symlink-directory");
    let root = base.join("project");
    fs::create_dir_all(&root).expect("create project root");
    let target = base.join("external");
    fs::create_dir_all(&target).expect("create external directory");
    fs::write(target.join("generated.rs"), "pub fn first() {}\n").expect("write external source");
    std::os::unix::fs::symlink(&target, root.join("linked")).expect("create directory symlink");
    let config = AspRustConfig::default();
    let first = snapshot_build_gate_inputs(&root, &config).expect("first snapshot");
    fs::write(target.join("generated.rs"), "pub fn second() {}\n").expect("change external source");
    let second = snapshot_build_gate_inputs(&root, &config).expect("second snapshot");
    assert_eq!(first, second);
    assert!(first.files.is_empty());
    let _ = fs::remove_dir_all(base);
}

#[test]
fn cache_key_invalidates_config_policy_scope_contract_and_baseline() {
    let root = temp_root("key-invalidation");
    fs::create_dir_all(root.join("src")).expect("create source root");
    fs::write(root.join("Cargo.toml"), "[package]\nname='fixture'\n").expect("write manifest");
    fs::write(root.join("src/lib.rs"), "pub fn value() -> usize { 1 }\n").expect("write source");
    let config = AspRustConfig::default();
    let snapshot = snapshot_build_gate_inputs(&root, &config).expect("snapshot");
    let baseline = vec![AspRustDependencyBaselinePackageReceipt {
        name: "dependency".to_string(),
        version: "1.0.0".to_string(),
        source_contains: "rev=one".to_string(),
    }];
    let key = build_gate_cache_key(
        &config,
        AspRustRunScope::ProjectWorkspace,
        &baseline,
        &snapshot,
    )
    .expect("baseline key");
    let harness_provider_digest =
        super::harness_provider_digest().expect("harness provider digest");

    let mut nested_policy = config.clone();
    nested_policy
        .verification_policy
        .disabled_task_kinds
        .insert(crate::RustVerificationTaskKind::Performance);
    let nested_key = build_gate_cache_key(
        &nested_policy,
        AspRustRunScope::ProjectWorkspace,
        &baseline,
        &snapshot,
    )
    .expect("nested policy key");
    let mut changed_baseline = baseline.clone();
    changed_baseline[0].source_contains = "rev=two".to_string();
    let baseline_key = build_gate_cache_key(
        &config,
        AspRustRunScope::ProjectWorkspace,
        &changed_baseline,
        &snapshot,
    )
    .expect("changed baseline key");
    let scope_key = build_gate_cache_key(&config, AspRustRunScope::Package, &baseline, &snapshot)
        .expect("changed scope key");
    let schema_key = build_gate_cache_key_with_contract(
        &config,
        AspRustRunScope::ProjectWorkspace,
        &baseline,
        &snapshot,
        BuildGateCacheContract {
            schema_id: "changed.schema",
            schema_version: ASP_RUST_BUILD_GATE_CACHE_SCHEMA_VERSION,
            harness_version: env!("CARGO_PKG_VERSION"),
            harness_provider_digest: &harness_provider_digest,
            policy_authority_digest: "blake3-256:test-policy-authority",
        },
    )
    .expect("changed schema key");
    let schema_version_key = build_gate_cache_key_with_contract(
        &config,
        AspRustRunScope::ProjectWorkspace,
        &baseline,
        &snapshot,
        BuildGateCacheContract {
            schema_id: ASP_RUST_BUILD_GATE_CACHE_SCHEMA_ID,
            schema_version: "changed-schema-version",
            harness_version: env!("CARGO_PKG_VERSION"),
            harness_provider_digest: &harness_provider_digest,
            policy_authority_digest: "blake3-256:test-policy-authority",
        },
    )
    .expect("changed schema version key");
    let harness_key = build_gate_cache_key_with_contract(
        &config,
        AspRustRunScope::ProjectWorkspace,
        &baseline,
        &snapshot,
        BuildGateCacheContract {
            schema_id: ASP_RUST_BUILD_GATE_CACHE_SCHEMA_ID,
            schema_version: ASP_RUST_BUILD_GATE_CACHE_SCHEMA_VERSION,
            harness_version: "changed-harness",
            harness_provider_digest: &harness_provider_digest,
            policy_authority_digest: "blake3-256:test-policy-authority",
        },
    )
    .expect("changed harness key");
    let provider_key = build_gate_cache_key_with_contract(
        &config,
        AspRustRunScope::ProjectWorkspace,
        &baseline,
        &snapshot,
        BuildGateCacheContract {
            schema_id: ASP_RUST_BUILD_GATE_CACHE_SCHEMA_ID,
            schema_version: ASP_RUST_BUILD_GATE_CACHE_SCHEMA_VERSION,
            harness_version: env!("CARGO_PKG_VERSION"),
            harness_provider_digest: "changed-provider-digest",
            policy_authority_digest: "blake3-256:test-policy-authority",
        },
    )
    .expect("changed provider key");
    let policy_authority_key = build_gate_cache_key_with_contract(
        &config,
        AspRustRunScope::ProjectWorkspace,
        &baseline,
        &snapshot,
        BuildGateCacheContract {
            schema_id: ASP_RUST_BUILD_GATE_CACHE_SCHEMA_ID,
            schema_version: ASP_RUST_BUILD_GATE_CACHE_SCHEMA_VERSION,
            harness_version: env!("CARGO_PKG_VERSION"),
            harness_provider_digest: &harness_provider_digest,
            policy_authority_digest: "blake3-256:changed-policy-authority",
        },
    )
    .expect("changed policy authority key");

    for changed in [
        nested_key,
        baseline_key,
        scope_key,
        schema_key,
        schema_version_key,
        harness_key,
        provider_key,
        policy_authority_key,
    ] {
        assert_ne!(changed, key);
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn downstream_cold_publish_then_warm_hit_parses_once() {
    let base = temp_root("downstream-hit");
    let project = base.join("project");
    let cache = base.join("cache");
    fs::create_dir_all(project.join("src")).expect("create source root");
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname='cache-fixture'\nversion='0.1.0'\nedition='2024'\n",
    )
    .expect("write manifest");
    fs::write(
        project.join("Cargo.lock"),
        "# This file is automatically @generated by Cargo.\nversion = 4\n\n[[package]]\nname = \"cache-fixture\"\nversion = \"0.1.0\"\n",
    )
    .expect("write stable lockfile");
    fs::write(
        project.join("src/lib.rs"),
        "//! Cache fixture facade.\n\nmod obsolete;\nmod value;\n\npub use obsolete::obsolete_value;\npub use value::cached_value;\n",
    )
    .expect("write source");
    fs::write(
        project.join("src/obsolete.rs"),
        "//! Module removed later to verify deletion invalidation.\n\n/// Returns a value from the module that will be deleted.\npub fn obsolete_value() -> usize { 0 }\n",
    )
    .expect("write removable implementation");
    fs::write(
        project.join("src/value.rs"),
        "//! Cached value implementation.\n\n/// Returns the cached fixture value.\npub fn cached_value() -> usize { 1 }\n",
    )
    .expect("write implementation");
    crate::runner::reset_analyze_rust_project_call_count();
    let policy = crate::AspRustDownstreamPolicy::new("cache-fixture", AspRustConfig::default());
    let initial_snapshot =
        crate::build_gate::cache::snapshot_build_gate_inputs(&project, policy.config())
            .expect("snapshot initial cache inputs");
    let initial_key = crate::build_gate::cache::build_gate_cache_key(
        policy.config(),
        AspRustRunScope::Package,
        &[],
        &initial_snapshot,
    )
    .expect("build initial cache key");
    assert_eq!(
        crate::runner::analyze_rust_project_call_count(),
        0,
        "snapshot and cache-key construction must not run the analyzer"
    );

    let cold = crate::build_gate::assert_asp_rust_downstream_policy_with_state_home(
        &project, &policy, &cache,
    );
    assert_eq!(crate::runner::analyze_rust_project_call_count(), 1);
    let resolved_cache_root = crate::build_gate::cache::build_gate_cache_root(
        &project,
        Some(cache.clone().into_os_string()),
    )
    .expect("resolve test cache root");
    let initial_cache_path = cache_path(&resolved_cache_root, &initial_key);
    assert!(
        crate::build_gate::cache::load_build_gate_cache(&resolved_cache_root, &initial_key)
            .is_some(),
        "cold build-gate run must publish a loadable cache record at {:?} (display-len={})",
        initial_cache_path,
        initial_cache_path.display().to_string().len()
    );
    let warm_snapshot =
        crate::build_gate::cache::snapshot_build_gate_inputs(&project, policy.config())
            .expect("snapshot warm cache inputs");
    let warm_key = crate::build_gate::cache::build_gate_cache_key(
        policy.config(),
        AspRustRunScope::Package,
        &[],
        &warm_snapshot,
    )
    .expect("build warm cache key");
    assert_eq!(
        warm_key, initial_key,
        "warm cache key drifted after cold run"
    );
    let warm = crate::build_gate::assert_asp_rust_downstream_policy_with_state_home(
        &project, &policy, &cache,
    );
    assert_eq!(crate::runner::analyze_rust_project_call_count(), 1);
    assert_eq!(warm, cold);

    fs::write(
        project.join("src/value.rs"),
        "//! Changed cached value implementation.\n\n/// Returns the changed cached fixture value.\npub fn cached_value() -> usize { 2 }\n",
    )
    .expect("change downstream source");
    let changed_snapshot =
        crate::build_gate::cache::snapshot_build_gate_inputs(&project, policy.config())
            .expect("snapshot changed cache inputs");
    let changed_key = crate::build_gate::cache::build_gate_cache_key(
        policy.config(),
        AspRustRunScope::Package,
        &[],
        &changed_snapshot,
    )
    .expect("build changed cache key");
    assert_ne!(changed_key, initial_key);
    let changed = crate::build_gate::assert_asp_rust_downstream_policy_with_state_home(
        &project, &policy, &cache,
    );
    assert_eq!(crate::runner::analyze_rust_project_call_count(), 2);
    assert_eq!(changed, cold);
    assert!(
        crate::build_gate::cache::load_build_gate_cache(&resolved_cache_root, &changed_key)
            .is_some(),
        "changed source must publish a new cache record"
    );

    let changed_warm = crate::build_gate::assert_asp_rust_downstream_policy_with_state_home(
        &project, &policy, &cache,
    );
    assert_eq!(crate::runner::analyze_rust_project_call_count(), 2);
    assert_eq!(changed_warm, changed);

    fs::rename(
        project.join("src/value.rs"),
        project.join("src/renamed_value.rs"),
    )
    .expect("rename downstream source");
    fs::write(
        project.join("src/lib.rs"),
        "//! Cache fixture facade after rename.\n\nmod obsolete;\nmod renamed_value;\n\npub use obsolete::obsolete_value;\npub use renamed_value::cached_value;\n",
    )
    .expect("point facade at renamed source");
    let renamed_snapshot =
        crate::build_gate::cache::snapshot_build_gate_inputs(&project, policy.config())
            .expect("snapshot renamed cache inputs");
    let renamed_key = crate::build_gate::cache::build_gate_cache_key(
        policy.config(),
        AspRustRunScope::Package,
        &[],
        &renamed_snapshot,
    )
    .expect("build renamed cache key");
    assert_ne!(renamed_key, changed_key);
    let renamed = crate::build_gate::assert_asp_rust_downstream_policy_with_state_home(
        &project, &policy, &cache,
    );
    assert_eq!(crate::runner::analyze_rust_project_call_count(), 3);
    assert!(
        crate::build_gate::cache::load_build_gate_cache(&resolved_cache_root, &renamed_key)
            .is_some(),
        "renamed source must publish a new cache record"
    );
    let renamed_warm = crate::build_gate::assert_asp_rust_downstream_policy_with_state_home(
        &project, &policy, &cache,
    );
    assert_eq!(crate::runner::analyze_rust_project_call_count(), 3);
    assert_eq!(renamed_warm, renamed);

    fs::remove_file(project.join("src/obsolete.rs")).expect("delete obsolete source");
    fs::write(
        project.join("src/lib.rs"),
        "//! Cache fixture facade after delete.\n\nmod renamed_value;\n\npub use renamed_value::cached_value;\n",
    )
    .expect("remove deleted module from facade");
    let deleted_snapshot =
        crate::build_gate::cache::snapshot_build_gate_inputs(&project, policy.config())
            .expect("snapshot deleted cache inputs");
    let deleted_key = crate::build_gate::cache::build_gate_cache_key(
        policy.config(),
        AspRustRunScope::Package,
        &[],
        &deleted_snapshot,
    )
    .expect("build deleted cache key");
    assert_ne!(deleted_key, renamed_key);
    let deleted = crate::build_gate::assert_asp_rust_downstream_policy_with_state_home(
        &project, &policy, &cache,
    );
    assert_eq!(crate::runner::analyze_rust_project_call_count(), 4);
    assert!(
        crate::build_gate::cache::load_build_gate_cache(&resolved_cache_root, &deleted_key)
            .is_some(),
        "deleted source must publish a new cache record"
    );
    let deleted_warm = crate::build_gate::assert_asp_rust_downstream_policy_with_state_home(
        &project, &policy, &cache,
    );
    assert_eq!(crate::runner::analyze_rust_project_call_count(), 4);
    assert_eq!(deleted_warm, deleted);

    let _ = fs::remove_dir_all(base);
}

#[test]
fn same_key_concurrent_publish_is_valid_and_leaves_no_temporary_files() {
    let root = std::sync::Arc::new(temp_root("concurrent-publish"));
    fs::create_dir_all(root.as_ref()).expect("create cache root");
    let snapshot = AspRustBuildGateSnapshot {
        digest: content_digest(b"[]"),
        file_count: 0,
        byte_count: 0,
        files: Vec::new(),
    };
    let record = std::sync::Arc::new(empty_record("concurrent-key".to_string(), snapshot));
    let publishers = (0..8)
        .map(|_| {
            let root = std::sync::Arc::clone(&root);
            let record = std::sync::Arc::clone(&record);
            std::thread::spawn(move || {
                store_build_gate_cache(root.as_ref(), record.as_ref())
                    .expect("publish concurrent cache record");
            })
        })
        .collect::<Vec<_>>();
    for publisher in publishers {
        publisher.join().expect("join cache publisher");
    }

    assert_eq!(
        load_build_gate_cache(root.as_ref(), &record.cache_key),
        Some(record.as_ref().clone())
    );
    assert!(
        fs::read_dir(root.as_ref())
            .expect("read cache root")
            .all(|entry| !entry
                .expect("cache entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp"))
    );
    let _ = fs::remove_dir_all(root.as_ref());
}
use crate::build_gate::cache::{build_gate_cache_key, build_gate_cache_key_with_contract};
use crate::{AspRustDependencyBaselinePackageReceipt, AspRustRunScope};
