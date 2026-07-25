use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::process::{Command, Output};

pub(crate) fn run_cli<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut command = Command::new(env!("CARGO_BIN_EXE_rs-harness"));
    configure_shared_asp_renderer(&mut command);
    let args = normalize_fixture_workspace_args(args);
    command.args(args).output().expect("run cli")
}

pub(crate) fn run_cli_with_env<I, S, K, V>(
    args: I,
    envs: impl IntoIterator<Item = (K, V)>,
) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
    K: AsRef<std::ffi::OsStr>,
    V: AsRef<std::ffi::OsStr>,
{
    let mut command = Command::new(env!("CARGO_BIN_EXE_rs-harness"));
    configure_shared_asp_renderer(&mut command);
    for (key, value) in envs {
        command.env(key, value);
    }
    let args = normalize_fixture_workspace_args(args);
    command.args(args).output().expect("run cli")
}

pub(crate) fn run_cli_with_stdin<I, S>(args: I, stdin: &str) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut command = Command::new(env!("CARGO_BIN_EXE_rs-harness"));
    configure_shared_asp_renderer(&mut command);
    let args = normalize_fixture_workspace_args(args);
    let mut child = command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn cli");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("run cli")
}

fn normalize_fixture_workspace_args<I, S>(args: I) -> Vec<std::ffi::OsString>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut args = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect::<Vec<_>>();
    let Some(first) = args.first().and_then(|arg| arg.to_str()) else {
        return args;
    };
    if first != "query" && first != "search" {
        return args;
    }
    if args
        .iter()
        .any(|arg| arg.to_str().is_some_and(|value| value == "--workspace"))
    {
        return args;
    }
    if first == "query" {
        if let Some(index) = args.iter().enumerate().skip(1).find_map(|(index, arg)| {
            let path = Path::new(arg);
            (path.is_absolute() && path.is_dir()).then_some(index)
        }) {
            args.insert(index, "--workspace".into());
        }
        return args;
    }
    let Some(last) = args.last() else {
        return args;
    };
    let root = Path::new(last);
    if !root.join("Cargo.toml").is_file() {
        return args;
    }
    let root = args.pop().expect("last arg");
    args.push("--workspace".into());
    args.push(root);
    args
}

#[cfg(feature = "search")]
pub(crate) fn skip_if_protocol_graph_renderer_unavailable() -> bool {
    let binary = std::env::var_os("SEMANTIC_AGENT_PROTOCOL_BIN")
        .or_else(|| shared_asp_renderer_binary().map(Into::into))
        .unwrap_or_else(|| "asp".into());
    let available = Command::new(binary)
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok();

    if available {
        false
    } else {
        eprintln!(
            "skipping test: shared asp graph renderer unavailable; set SEMANTIC_AGENT_PROTOCOL_BIN to enable"
        );
        true
    }
}

pub(crate) fn configure_shared_asp_renderer(command: &mut Command) {
    if let Some(binary) = std::env::var_os("SEMANTIC_AGENT_PROTOCOL_BIN") {
        let binary = Path::new(&binary);
        if binary.is_relative()
            && binary.is_file()
            && let Ok(canonical) = binary.canonicalize()
        {
            command.env("SEMANTIC_AGENT_PROTOCOL_BIN", canonical);
        }
        return;
    }
    if let Some(binary) = shared_asp_renderer_binary() {
        command.env("SEMANTIC_AGENT_PROTOCOL_BIN", binary);
    }
}

pub(crate) fn write_source_snapshot_envelope(
    root: &Path,
    provider_id: &str,
    sources: &[(&str, &str)],
) -> std::path::PathBuf {
    let cas_root = root.join(".asp-test-cas");
    let owners = sources
        .iter()
        .enumerate()
        .map(|(index, (owner, source))| {
            let blob_digest = format!("{:064x}", index + 1);
            let cas_path = format!("{}/{}", &blob_digest[..2], &blob_digest[2..]);
            let blob_path = cas_root.join(&cas_path);
            fs::create_dir_all(blob_path.parent().expect("CAS blob parent"))
                .expect("create CAS shard");
            fs::write(blob_path, source).expect("write pinned source blob");
            serde_json::json!({
                "path": owner,
                "snapshotLeafDigest": format!("{:064x}", index + 1000),
                "blobDigest": blob_digest,
                "casPath": cas_path,
            })
        })
        .collect::<Vec<_>>();
    let envelope_path = root.join("source-snapshot-envelope.v1.json");
    fs::write(
        &envelope_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaId": "asp.exact-source-snapshot-envelope.v1",
            "schemaVersion": "1",
            "providerId": provider_id,
            "sourceSnapshot": {
                "schemaId": "asp.source-snapshot.v1",
                "schemaVersion": "1",
                "algorithm": "blake3-merkle-v1",
                "rootDigest": format!("{:064x}", sources.len() + 10_000),
                "sourceKind": "filesystem",
                "leafCount": sources.len(),
                "providerDigest": format!("{:064x}", 20_000),
            },
            "casRoot": cas_root,
            "owners": owners,
        }))
        .expect("encode source snapshot envelope"),
    )
    .expect("write source snapshot envelope");
    envelope_path
}

fn shared_asp_renderer_binary() -> Option<std::path::PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    current_dir
        .ancestors()
        .map(|ancestor| {
            ancestor
                .join("target")
                .join("debug")
                .join(format!("asp{}", std::env::consts::EXE_SUFFIX))
        })
        .find(|candidate| candidate.is_file())
        .or_else(|| {
            current_dir
                .ancestors()
                .map(|ancestor| {
                    ancestor
                        .join("tests")
                        .join("fixtures")
                        .join("bin")
                        .join(if cfg!(windows) { "asp.cmd" } else { "asp" })
                })
                .find(|candidate| candidate.is_file())
        })
}

#[cfg(feature = "search")]
pub(crate) fn run_search(root: &Path, args: &[&str]) -> String {
    let mut command_args = Vec::<std::ffi::OsString>::new();
    command_args.push("search".into());
    command_args.extend(args.iter().map(std::ffi::OsString::from));
    command_args.push("--workspace".into());
    command_args.push(root.as_os_str().to_os_string());
    let output = run_cli(command_args);
    assert!(output.status.success(), "{output:?}");
    normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    )
}

#[cfg(feature = "search")]
pub(crate) fn run_search_with_stdin(root: &Path, args: &[&str], stdin: &str) -> String {
    let mut command_args = Vec::<std::ffi::OsString>::new();
    command_args.push("search".into());
    command_args.extend(args.iter().map(std::ffi::OsString::from));
    command_args.push("--workspace".into());
    command_args.push(root.as_os_str().to_os_string());
    let output = run_cli_with_stdin(command_args, stdin);
    assert!(output.status.success(), "{output:?}");
    normalize_temp_root(
        &String::from_utf8(output.stdout).expect("utf8 stdout"),
        root,
    )
}

pub(crate) fn write_manifest(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
}

#[cfg(feature = "search")]
pub(crate) fn write_search_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"cli-search-views\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [lib]\n\
         path = \"src/lib.rs\"\n\n\
         [[test]]\n\
         name = \"domain\"\n\
         path = \"tests/domain.rs\"\n\n\
         [features]\n\
         default = [\"serde\"]\n\
         json = [\"dep:serde\", \"serde/derive\"]\n\n\
         [dependencies]\n\
         anyhow = \"1\"\n\
         serde = { version = \"1\", optional = true, features = [\"derive\"] }\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src/domain")).expect("create domain");
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(
        root.join("src/lib.rs"),
        "//! Test crate.\n\
         mod domain;\n\
         use serde::Serialize;\n\
         pub use domain::Thing;\n\n\
         pub fn load() -> Thing { domain::make_thing() }\n\n\
         pub fn clone_value(input: String) -> String { input.clone() }\n\n\
         /// Fallible API.\n\
         pub async unsafe fn fallible(input: String) -> anyhow::Result<Thing> { let _ = input; todo!() }\n\n\
         pub struct PublicWire { pub serializer: serde::Serialize }\n\n\
         impl PublicWire { pub fn as_thing(&mut self) -> Thing { domain::make_thing() } }\n\n\
         pub trait WireApi { fn wire(&self) -> anyhow::Result<Thing>; }\n\
         impl WireApi for PublicWire { fn wire(&self) -> anyhow::Result<Thing> { todo!() } }\n",
    )
    .expect("write lib");
    fs::write(
        root.join("src/domain/mod.rs"),
        "//! Domain branch.\n\
         use serde::Serialize;\n\n\
         #[derive(Serialize)]\n\
         pub struct Thing { pub id: String }\n\n\
         pub fn make_thing() -> Thing { Thing { id: \"id\".to_string() } }\n\n\
         #[cfg(feature = \"json\")]\n\
         pub fn json_enabled() -> bool { true }\n",
    )
    .expect("write domain");
    fs::write(
        root.join("tests/domain.rs"),
        "use cli_search_views::load;\n\n\
         #[test]\n\
         fn loads() { let _ = load(); }\n",
    )
    .expect("write test");
}

#[cfg(feature = "search")]
pub(crate) fn write_complex_dependency_fixture(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"cli-complex-flow\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [lib]\n\
         path = \"src/lib.rs\"\n\n\
         [[test]]\n\
         name = \"flow\"\n\
         path = \"tests/flow.rs\"\n\n\
         [features]\n\
         default = [\"runtime\", \"walk\"]\n\
         runtime = [\"dep:tokio\", \"tokio/rt-multi-thread\", \"tokio/sync\", \"dep:bytes\"]\n\
         walk = [\"dep:ignore\"]\n\
         wire = [\"dep:bytes\"]\n\n\
         [dependencies]\n\
         bytes = { version = \"1\", optional = true }\n\
         ignore = { version = \"0.4\", optional = true }\n\
         tokio = { version = \"1\", optional = true, features = [\"rt-multi-thread\", \"sync\"] }\n\n\
         [lints.rust]\n\
         unexpected_cfgs = { level = \"warn\", check-cfg = ['cfg(tokio_unstable)'] }\n",
    )
    .expect("write manifest");
    fs::create_dir_all(root.join("src/http")).expect("create http");
    fs::create_dir_all(root.join("src/io")).expect("create io");
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(
        root.join("src/lib.rs"),
        "//! Complex dependency search fixture.\n\
         pub mod http;\n\
         pub mod io;\n\n\
         pub use http::client::{send_bytes, RuntimeApi, RuntimeClient};\n\
         pub use io::walk::{build_walk, WalkPlan};\n\n\
         #[cfg(feature = \"runtime\")]\n\
         pub fn runtime_enabled() -> bool { true }\n",
    )
    .expect("write lib");
    fs::write(root.join("src/http/mod.rs"), "pub mod client;\n").expect("write http mod");
    fs::write(
        root.join("src/http/client.rs"),
        "use bytes::{Buf, Bytes};\n\
         use tokio::sync::mpsc::Sender;\n\n\
         pub struct RuntimeClient { pub sender: Sender<Bytes>, pub buffer: Bytes }\n\n\
         pub async fn send_bytes(sender: Sender<Bytes>, mut payload: Bytes) -> Result<(), tokio::sync::mpsc::error::SendError<Bytes>> {\n\
         \tlet _ = payload.remaining();\n\
         \tsender.send(payload).await\n\
         }\n\n\
         pub trait RuntimeApi { fn sender(&self) -> Sender<Bytes>; }\n\n\
         impl RuntimeApi for RuntimeClient { fn sender(&self) -> Sender<Bytes> { self.sender.clone() } }\n\n\
         #[cfg(tokio_unstable)]\n\
         pub fn runtime_metrics() -> usize { 1 }\n",
    )
    .expect("write client");
    fs::write(root.join("src/io/mod.rs"), "pub mod walk;\n").expect("write io mod");
    fs::write(
        root.join("src/io/walk.rs"),
        "use bytes::Bytes;\n\
         use ignore::WalkBuilder;\n\n\
         pub struct WalkPlan { pub builder: WalkBuilder, pub seed: Bytes }\n\n\
         pub fn build_walk(root: &str) -> WalkBuilder { WalkBuilder::new(root) }\n\n\
         pub fn consume_chunk(chunk: Bytes) -> usize { chunk.len() }\n",
    )
    .expect("write walk");
    fs::write(
        root.join("tests/flow.rs"),
        "use cli_complex_flow::{build_walk, send_bytes, RuntimeClient, WalkPlan};\n\n\
         #[test]\n\
         fn routes_runtime_walk_and_bytes() { let _ = \"runtime tokio bytes ignore WalkBuilder RuntimeClient send_bytes WalkPlan build_walk\"; }\n",
    )
    .expect("write flow test");
}

pub(crate) fn write_clean_source(root: &Path) {
    fs::create_dir(root.join("src")).expect("create src");
    fs::write(root.join("src/lib.rs"), "//! Test crate.\n").expect("write lib");
}

pub(crate) fn normalize_temp_root(rendered: &str, root: &Path) -> String {
    let mut spellings = vec![root.display().to_string()];
    if let Ok(canonical_root) = std::fs::canonicalize(root) {
        spellings.push(canonical_root.display().to_string());
    }
    let slash_normalized = spellings
        .iter()
        .map(|spelling| spelling.replace('\\', "/"))
        .collect::<Vec<_>>();
    spellings.extend(slash_normalized);
    spellings.sort();
    spellings.dedup();
    spellings.sort_by_key(|right| std::cmp::Reverse(right.len()));
    spellings
        .into_iter()
        .fold(rendered.to_string(), |output, spelling| {
            output.replace(&spelling, "$TEMP")
        })
}
