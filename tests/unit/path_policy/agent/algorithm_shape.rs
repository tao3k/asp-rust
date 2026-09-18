use std::fs;

use asp_rust::run_asp_rust_for_scope;
use tempfile::TempDir;

use crate::path_policy::support::{findings_for_rule, write_manifest};

#[test]
fn public_nested_algorithm_shape_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "nested-algorithm-shape");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Classifies rows.\n\
         pub fn classify(rows: &[usize], enabled: bool) -> usize {\n\
         \tlet mut total = 0;\n\
         \tfor row in rows {\n\
         \t\tif enabled {\n\
         \t\t\tif *row > 10 {\n\
         \t\t\t\tif *row < 20 {\n\
         \t\t\t\t\ttotal += *row;\n\
         \t\t\t\t}\n\
         \t\t\t}\n\
         \t\t}\n\
         \t}\n\
         \ttotal\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-CFG-PUBLIC-015");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("control-flow.decision-stack"));
    assert_eq!(
        findings[0]
            .labels
            .get("softwareCriteria")
            .map(String::as_str),
        Some("control-flow.decision-stack")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_broad_linear_algorithm_surface_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "broad-linear-algorithm");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(root.join("src/api.rs"), broad_linear_source()).expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-CFG-PUBLIC-016");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("control-flow.broad-linear-phase")
    );
    assert_eq!(
        findings[0]
            .labels
            .get("softwareCriteria")
            .map(String::as_str),
        Some("control-flow.broad-linear-phase")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_match_dispatch_is_not_nested_algorithm_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "match-dispatch-algorithm");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Classifies a route.\n\
         pub fn classify(kind: &str) -> usize {\n\
         \tmatch kind {\n\
         \t\t\"alpha\" => 1,\n\
         \t\t\"beta\" => 2,\n\
         \t\t\"gamma\" => 3,\n\
         \t\t\"delta\" => 4,\n\
         \t\t\"epsilon\" => 5,\n\
         \t\t\"zeta\" => 6,\n\
         \t\t\"eta\" => 7,\n\
         \t\t_ => 0,\n\
         \t}\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-CFG-PUBLIC-015").is_empty());
    assert!(findings_for_rule(&report, "RUST-AGENT-CFG-PUBLIC-016").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_literal_dispatch_chain_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "literal-dispatch-chain");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(root.join("src/api.rs"), literal_dispatch_source()).expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-CFG-PUBLIC-015");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("control-flow.literal-dispatch-chain")
    );
    assert_eq!(
        findings[0]
            .labels
            .get("softwareCriteria")
            .map(String::as_str),
        Some("control-flow.literal-dispatch-chain")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn public_manual_iterator_boilerplate_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "manual-iterator-boilerplate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(root.join("src/api.rs"), manual_iterator_source()).expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-ITER-PUBLIC-017");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("native-idiom.manual-transform-loop")
    );
    assert_eq!(
        findings[0]
            .labels
            .get("softwareCriteria")
            .map(String::as_str),
        Some("native-idiom.manual-transform-loop")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn loop_local_linear_membership_scan_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "linear-membership-scan");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(root.join("src/api.rs"), linear_membership_scan_source()).expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-DATA-MEMBERSHIP-029");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("data-structure.linear-membership-scan")
    );
    assert_eq!(
        findings[0]
            .labels
            .get("softwareCriteria")
            .map(String::as_str),
        Some("data-structure.linear-membership-scan")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn indexed_membership_lookup_clears_linear_scan_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "indexed-membership-lookup");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(root.join("src/api.rs"), indexed_membership_lookup_source()).expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-DATA-MEMBERSHIP-029").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn async_blocking_call_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "async-blocking-call");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async API owner.\n\
         /// Refreshes cached work.\n\
         pub async fn refresh() -> usize {\n\
         \tstd::thread::sleep(std::time::Duration::from_millis(1));\n\
         \t1\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-ASYNC-BLOCKING-030");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("async.blocking-boundary"));
    assert_eq!(
        findings[0]
            .labels
            .get("softwareCriteria")
            .map(String::as_str),
        Some("async.blocking-boundary")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn spawn_blocking_boundary_clears_async_blocking_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "async-blocking-boundary");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async API owner.\n\
         /// Refreshes cached work.\n\
         pub async fn refresh() -> usize {\n\
         \ttokio::task::spawn_blocking(|| {\n\
         \t\tstd::thread::sleep(std::time::Duration::from_millis(1));\n\
         \t\t1\n\
         \t});\n\
         \t1\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-ASYNC-BLOCKING-030").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn sync_lock_guard_across_await_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "sync-lock-across-await");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async API owner.\n\
         use std::sync::Mutex;\n\
         /// Refreshes shared state.\n\
         pub async fn refresh(state: &Mutex<usize>) -> usize {\n\
         \tlet mut guard = state.lock().unwrap();\n\
         \t*guard += 1;\n\
         \ttokio::task::yield_now().await;\n\
         \t*guard\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-ASYNC-SYNC-LOCK-031");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("async.sync-lock-across-await"));
    assert_eq!(
        findings[0]
            .labels
            .get("softwareCriteria")
            .map(String::as_str),
        Some("async.sync-lock-across-await")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn std_rwlock_read_guard_across_await_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "std-rwlock-read-guard-across-await");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async API owner.\n\
         use std::sync::RwLock;\n\
         /// Reads shared state.\n\
         pub async fn refresh(state: &RwLock<usize>) -> usize {\n\
         \tlet guard = state.read().unwrap();\n\
         \ttokio::task::yield_now().await;\n\
         \t*guard\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-ASYNC-SYNC-LOCK-031");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(findings[0].summary.contains("async.sync-lock-across-await"));
    assert_eq!(
        findings[0]
            .labels
            .get("softwareCriteria")
            .map(String::as_str),
        Some("async.sync-lock-across-await")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn tokio_rwlock_read_guard_clears_sync_lock_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "tokio-rwlock-read-guard");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async API owner.\n\
         use tokio::sync::RwLock;\n\
         /// Reads shared state with an async-aware lock.\n\
         pub async fn refresh(state: &RwLock<usize>) -> usize {\n\
         \tlet guard = state.read().await;\n\
         \ttokio::task::yield_now().await;\n\
         \t*guard\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-ASYNC-SYNC-LOCK-031").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn dropped_sync_lock_guard_clears_across_await_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "dropped-sync-lock-before-await");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async API owner.\n\
         use std::sync::Mutex;\n\
         /// Refreshes shared state.\n\
         pub async fn refresh(state: &Mutex<usize>) -> usize {\n\
         \tlet value = {\n\
         \t\tlet mut guard = state.lock().unwrap();\n\
         \t\t*guard += 1;\n\
         \t\t*guard\n\
         \t};\n\
         \ttokio::task::yield_now().await;\n\
         \tvalue\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-ASYNC-SYNC-LOCK-031").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn unbounded_async_queue_without_backpressure_is_agent_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "unbounded-async-queue");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async queue API owner.\n\
         use tokio::sync::mpsc;\n\
         /// Fans out work with no capacity boundary.\n\
         pub async fn fanout(items: Vec<String>) -> usize {\n\
         \tlet (tx, mut rx) = mpsc::unbounded_channel();\n\
         \tfor item in items {\n\
         \t\tlet _ = tx.send(item);\n\
         \t}\n\
         \tdrop(tx);\n\
         \tlet mut count = 0;\n\
         \twhile let Some(_item) = rx.recv().await {\n\
         \t\tcount += 1;\n\
         \t}\n\
         \tcount\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-ASYNC-BACKPRESSURE-032");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("async.unbounded-queue-backpressure")
    );
    assert_eq!(
        findings[0]
            .labels
            .get("softwareCriteria")
            .map(String::as_str),
        Some("async.unbounded-queue-backpressure")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn bounded_async_queue_clears_backpressure_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "bounded-async-queue");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async queue API owner.\n\
         use tokio::sync::mpsc;\n\
         /// Fans out work with a capacity boundary.\n\
         pub async fn fanout(items: Vec<String>) -> usize {\n\
         \tlet (tx, mut rx) = mpsc::channel(64);\n\
         \tfor item in items {\n\
         \t\tlet _ = tx.send(item).await;\n\
         \t}\n\
         \tdrop(tx);\n\
         \tlet mut count = 0;\n\
         \twhile let Some(_item) = rx.recv().await {\n\
         \t\tcount += 1;\n\
         \t}\n\
         \tcount\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-ASYNC-BACKPRESSURE-032").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn try_send_boundary_clears_unbounded_queue_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "unbounded-async-queue-wrapper");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async queue API owner.\n\
         use std::task::{Context, Poll};\n\
         use tokio::sync::mpsc;\n\
         pub struct Sender {\n\
         \tinner: mpsc::UnboundedSender<String>,\n\
         }\n\
         /// Builds a queue hidden behind an explicit readiness API.\n\
         pub fn channel() -> Sender {\n\
         \tlet (tx, _rx) = mpsc::unbounded_channel();\n\
         \tSender { inner: tx }\n\
         }\n\
         impl Sender {\n\
         \tpub fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), ()>> {\n\
         \t\tPoll::Ready(Ok(()))\n\
         \t}\n\
         \tpub fn try_send(&mut self, message: String) -> Result<(), String> {\n\
         \t\tself.inner.send(message).map_err(|error| error.0)\n\
         \t}\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-ASYNC-BACKPRESSURE-032").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn tokio_select_read_exact_is_cancellation_safety_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "tokio-select-read-exact");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async select API owner.\n\
         use tokio::io::{AsyncRead, AsyncReadExt};\n\
         /// Reads a frame or returns when shutdown fires.\n\
         pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R, mut shutdown: tokio::sync::oneshot::Receiver<()>) -> std::io::Result<[u8; 8]> {\n\
         \tlet mut buf = [0; 8];\n\
         \ttokio::select! {\n\
         \t\tresult = reader.read_exact(&mut buf) => { result.map(|_| buf) }\n\
         \t\t_ = &mut shutdown => { Ok(buf) }\n\
         \t}\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-ASYNC-CANCEL-SAFETY-033");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("async.select-cancellation-safety")
    );
    assert_eq!(
        findings[0]
            .labels
            .get("softwareCriteria")
            .map(String::as_str),
        Some("async.select-cancellation-safety")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn tokio_select_read_clears_cancellation_safety_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "tokio-select-read");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async select API owner.\n\
         use tokio::io::{AsyncRead, AsyncReadExt};\n\
         /// Reads any available bytes or returns when shutdown fires.\n\
         pub async fn read_some<R: AsyncRead + Unpin>(reader: &mut R, mut shutdown: tokio::sync::oneshot::Receiver<()>) -> std::io::Result<usize> {\n\
         \tlet mut buf = [0; 8];\n\
         \ttokio::select! {\n\
         \t\tresult = reader.read(&mut buf) => { result }\n\
         \t\t_ = &mut shutdown => { Ok(0) }\n\
         \t}\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-ASYNC-CANCEL-SAFETY-033").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn read_exact_outside_select_clears_cancellation_safety_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "read-exact-outside-select");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async select API owner.\n\
         use tokio::io::{AsyncRead, AsyncReadExt};\n\
         /// Reads an exact frame outside cancellation competition.\n\
         pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<[u8; 8]> {\n\
         \tlet mut buf = [0; 8];\n\
         \treader.read_exact(&mut buf).await?;\n\
         \tOk(buf)\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-ASYNC-CANCEL-SAFETY-033").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn tokio_timeout_read_exact_is_cancellation_safety_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "tokio-timeout-read-exact");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async timeout API owner.\n\
         use std::time::Duration;\n\
         use tokio::io::{AsyncRead, AsyncReadExt};\n\
         /// Reads a frame under a timeout.\n\
         pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<[u8; 8]> {\n\
         \tlet mut buf = [0; 8];\n\
         \ttokio::time::timeout(Duration::from_secs(1), reader.read_exact(&mut buf)).await??;\n\
         \tOk(buf)\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    let findings = findings_for_rule(&report, "RUST-AGENT-ASYNC-CANCEL-SAFETY-034");
    assert_eq!(findings.len(), 1, "{:?}", report.findings);
    assert!(
        findings[0]
            .summary
            .contains("async.timeout-cancellation-safety")
    );
    assert_eq!(
        findings[0]
            .labels
            .get("softwareCriteria")
            .map(String::as_str),
        Some("async.timeout-cancellation-safety")
    );
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn tokio_timeout_read_clears_cancellation_safety_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "tokio-timeout-read");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async timeout API owner.\n\
         use std::time::Duration;\n\
         use tokio::io::{AsyncRead, AsyncReadExt};\n\
         /// Reads available bytes under a timeout.\n\
         pub async fn read_some<R: AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<usize> {\n\
         \tlet mut buf = [0; 8];\n\
         \tlet bytes = tokio::time::timeout(Duration::from_secs(1), reader.read(&mut buf)).await??;\n\
         \tOk(bytes)\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-ASYNC-CANCEL-SAFETY-034").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn read_exact_outside_timeout_clears_cancellation_safety_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "read-exact-outside-timeout");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Async timeout API owner.\n\
         use std::time::Duration;\n\
         use tokio::io::{AsyncRead, AsyncReadExt};\n\
         /// Keeps exact read progress outside the timed future.\n\
         pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<[u8; 8]> {\n\
         \ttokio::time::timeout(Duration::from_secs(1), async { Ok::<(), std::io::Error>(()) }).await??;\n\
         \tlet mut buf = [0; 8];\n\
         \treader.read_exact(&mut buf).await?;\n\
         \tOk(buf)\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert!(findings_for_rule(&report, "RUST-AGENT-ASYNC-CANCEL-SAFETY-034").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

#[test]
fn deeply_nested_algorithm_does_not_duplicate_native_iterator_advice() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "nested-algorithm-no-native-duplicate");
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\nmod api;\n").expect("write lib");
    fs::write(
        root.join("src/api.rs"),
        "//! Public API owner.\n\
         /// Classifies rows.\n\
         pub fn classify(rows: &[usize], enabled: bool) -> usize {\n\
         \tlet mut total = 0;\n\
         \tfor row in rows {\n\
         \t\tif enabled {\n\
         \t\t\tif *row > 10 {\n\
         \t\t\t\tif *row < 20 {\n\
         \t\t\t\t\ttotal += 1;\n\
         \t\t\t\t}\n\
         \t\t\t}\n\
         \t\t}\n\
         \t}\n\
         \ttotal\n\
         }\n",
    )
    .expect("write api");

    let report = run_asp_rust_for_scope(root, asp_rust::AspRustRunScope::Package)
        .expect("run project harness");

    assert_eq!(
        findings_for_rule(&report, "RUST-AGENT-CFG-PUBLIC-015").len(),
        1
    );
    assert!(findings_for_rule(&report, "RUST-AGENT-ITER-PUBLIC-017").is_empty());
    assert!(report.is_clean(), "{:?}", report.findings);
}

fn broad_linear_source() -> String {
    let mut source = String::from(
        "//! Public API owner.\n\
         /// Summarizes values.\n\
         pub fn summarize(value: usize) -> usize {\n",
    );
    for index in 0..15 {
        source.push_str(&format!("    let step_{index} = value + {index};\n"));
    }
    source.push_str("    step_0\n}\n");
    source
}

fn manual_iterator_source() -> String {
    "//! Public API owner.\n\
     /// Summarizes values.\n\
     pub fn summarize(values: &[usize]) -> bool {\n\
     \tlet mut doubled = Vec::new();\n\
     \tfor value in values {\n\
     \t\tif *value > 0 {\n\
     \t\t\tdoubled.push(*value * 2);\n\
     \t\t}\n\
     \t}\n\
     \tfor value in values {\n\
     \t\tif *value > 100 {\n\
     \t\t\treturn true;\n\
     \t\t}\n\
     \t}\n\
     \tlet mut count = 0;\n\
     \tfor value in values {\n\
     \t\tif *value > 10 {\n\
     \t\t\tcount += 1;\n\
     \t\t}\n\
     \t}\n\
     \tlet mut total = 0;\n\
     \tfor value in values {\n\
     \t\ttotal += *value;\n\
     \t}\n\
     \tlet _ = (doubled, count, total);\n\
     \tfalse\n\
     }\n"
    .to_string()
}

fn linear_membership_scan_source() -> String {
    "//! Public API owner.\n\
     /// Selects rows whose IDs are allowed.\n\
     pub fn select_allowed(rows: &[Row], allowed: &[String]) -> Vec<String> {\n\
     \tlet mut selected = Vec::new();\n\
     \tfor row in rows {\n\
     \t\tif allowed.iter().any(|candidate| candidate == &row.id) {\n\
     \t\t\tselected.push(row.id.clone());\n\
     \t\t}\n\
     \t}\n\
     \tselected\n\
     }\n\
     pub struct Row { pub id: String }\n"
        .to_string()
}

fn indexed_membership_lookup_source() -> String {
    "//! Public API owner.\n\
     use std::collections::BTreeSet;\n\
     /// Selects rows whose IDs are allowed.\n\
     pub fn select_allowed(rows: &[Row], allowed: &BTreeSet<String>) -> Vec<String> {\n\
     \trows.iter()\n\
     \t\t.filter(|row| allowed.contains(&row.id))\n\
     \t\t.map(|row| row.id.clone())\n\
     \t\t.collect()\n\
     }\n\
     pub struct Row { pub id: String }\n"
        .to_string()
}

fn literal_dispatch_source() -> String {
    "//! Public API owner.\n\
     /// Routes a kind.\n\
     pub fn route(kind: &str) -> usize {\n\
     \tif kind == \"alpha\" {\n\
     \t\t1\n\
     \t} else if kind == \"beta\" {\n\
     \t\t2\n\
     \t} else if kind == \"gamma\" {\n\
     \t\t3\n\
     \t} else {\n\
     \t\t0\n\
     \t}\n\
     }\n"
    .to_string()
}
