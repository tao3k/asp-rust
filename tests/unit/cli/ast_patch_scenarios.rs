use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::support::{run_cli, run_cli_with_stdin};

mod cases_1;

fn run_ast_patch_scenario(dir: &Path) {
    let name = scenario_name(dir);
    let temp = tempfile::tempdir()
        .unwrap_or_else(|error| panic!("{name}: failed to create tempdir: {error}"));
    copy_dir_recursive(&dir.join("input"), temp.path());

    let scenario = read_json(&dir.join("scenario.json"));
    let mode = scenario["mode"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: scenario mode must be a string"));
    let expected_status = scenario["expectedStatus"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: expectedStatus must be a string"));
    let packet = fs::read_to_string(dir.join("packet.json"))
        .unwrap_or_else(|error| panic!("{name}: failed to read packet: {error}"));

    let output = run_cli_with_stdin(
        [
            OsString::from("ast-patch"),
            OsString::from(mode),
            OsString::from("--packet"),
            OsString::from("-"),
            temp.path().as_os_str().to_os_string(),
        ],
        &packet,
    );
    let status = output.status;
    let stdout = String::from_utf8(output.stdout)
        .unwrap_or_else(|error| panic!("{name}: stdout was not utf-8: {error}"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        status.success(),
        "{name}: ast-patch command failed: status={status:?} stdout={stdout} stderr={stderr}"
    );

    if mode == "apply" && expected_status == "applied" {
        assert!(
            stdout.is_empty(),
            "{name}: successful ast-patch apply should not print a receipt: {stdout}"
        );
    } else {
        let receipt = serde_json::from_str::<Value>(&stdout)
            .unwrap_or_else(|error| panic!("{name}: receipt JSON: {error}: {stdout}"));
        assert_receipt_matches(&name, &scenario, &receipt);
    }

    let expected = snapshot_dir(&dir.join("expected"));
    let actual = snapshot_dir(temp.path());
    assert_eq!(
        actual, expected,
        "{name}: applied tree should match expected fixture"
    );

    for compact_check in json_array(&scenario, "compactChecks") {
        assert_compact_check(&name, dir, temp.path(), compact_check);
    }
}

fn ast_patch_scenarios_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ast_patch_scenarios")
}

fn assert_receipt_matches(name: &str, scenario: &Value, receipt: &Value) {
    let expected_status = scenario["expectedStatus"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: scenario.expectedStatus must be a string"));
    assert_eq!(
        receipt["status"].as_str(),
        Some(expected_status),
        "{name}: receipt.status"
    );
    assert_eq!(
        receipt["mode"].as_str(),
        scenario["mode"].as_str(),
        "{name}: receipt.mode"
    );

    if let Some(expected) = scenario.get("expectedCapability").and_then(Value::as_str) {
        assert_eq!(
            receipt["capability"].as_str(),
            Some(expected),
            "{name}: receipt.capability"
        );
    }
    if let Some(expected) = scenario
        .get("expectedMutationAvailable")
        .and_then(Value::as_bool)
    {
        assert_eq!(
            receipt["mutationAvailable"].as_bool(),
            Some(expected),
            "{name}: receipt.mutationAvailable"
        );
    }
    if let Some(expected) = scenario.get("expectedOperation").and_then(Value::as_str) {
        assert_eq!(
            receipt["operation"].as_str(),
            Some(expected),
            "{name}: receipt.operation"
        );
    }

    match scenario.get("expectedFailureKind") {
        Some(Value::String(expected)) => assert_eq!(
            receipt["failureKind"].as_str(),
            Some(expected.as_str()),
            "{name}: receipt.failureKind"
        ),
        Some(Value::Null) => assert!(
            receipt["failureKind"].is_null(),
            "{name}: receipt.failureKind should be null"
        ),
        Some(_) => panic!("{name}: scenario.expectedFailureKind must be string or null"),
        None => {}
    }

    for expected in json_string_array(scenario, "verificationContains") {
        assert_receipt_verification_contains(name, receipt, expected);
    }
    for forbidden in json_string_array(scenario, "verificationExcludes") {
        assert_receipt_verification_excludes(name, receipt, forbidden);
    }
}

fn assert_compact_check(name: &str, scenario_dir: &Path, root: &Path, check: &Value) {
    let path = check["path"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: compactChecks[].path must be a string"));
    let query = check["query"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: compactChecks[].query must be a string"));

    let search_packet = run_owner_item_search_json(root, path, query);
    let item = select_search_item(
        name,
        "compactChecks[]",
        &search_packet,
        check.get("matchKind").and_then(Value::as_str),
        check.get("matchName").and_then(Value::as_str),
    );
    let evidence = search_item_evidence(item);
    let compact_code = configured_compact_code(name, scenario_dir, check);
    assert_expected_compact_code(name, scenario_dir, check, &compact_code);
    for expected in json_string_array(check, "codeContains") {
        assert!(
            compact_code.contains(expected),
            "{name}: compact code missing {expected:?}:\n{compact_code}"
        );
    }
    for forbidden in json_string_array(check, "codeNotContains") {
        assert!(
            !compact_code.contains(forbidden),
            "{name}: compact code unexpectedly contained {forbidden:?}:\n{compact_code}"
        );
    }
    assert_eq!(
        search_packet["method"], "search/owner",
        "{name}: parser-owned discovery method"
    );
    assert!(
        item["fields"]["structuralSelector"]
            .as_str()
            .is_some_and(|selector| selector.starts_with("rust://")),
        "{name}: search item must expose a structural selector: {item}"
    );
    assert!(
        item["fields"]["read"].as_str().is_some(),
        "{name}: search item must expose an exact read locator: {item}"
    );

    let target = ast_patch_target_from_search_item(item);
    let exact_source = exact_read_from_target(root, &target);
    for expected in json_string_array(check, "exactContains") {
        assert!(
            exact_source.contains(expected),
            "{name}: exact formatted source missing {expected:?}:\n{exact_source}"
        );
    }
    if check
        .get("assertCompactShorter")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        assert!(
            compact_code.len() < exact_source.len(),
            "{name}: compact code was not shorter than exact source\ncompact={}\nexact={}",
            compact_code.len(),
            exact_source.len()
        );
    }
    if let Some(minimum) = check.get("minimumFunctionalComplexity") {
        assert_minimum_functional_complexity(
            name,
            minimum,
            &evidence,
            compact_code.len(),
            exact_source.len(),
        );
    }
    if check
        .get("rejectCompactAsPreimage")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        assert_compact_code_rejected_as_preimage(name, root, &target, &exact_source, &compact_code);
    }
    if let Some(save_apply_patch) = check.get("saveApplyPatch").and_then(Value::as_str) {
        assert_saved_compact_apply_patch_passes(name, scenario_dir, save_apply_patch);
    }
}

fn assert_minimum_functional_complexity(
    name: &str,
    minimum: &Value,
    evidence: &SearchItemEvidence<'_>,
    compact_len: usize,
    exact_len: usize,
) {
    assert_min_usize(
        name,
        "minimumFunctionalComplexity.minDistinctResponsibilities",
        evidence.responsibilities.len(),
        minimum
            .get("minDistinctResponsibilities")
            .and_then(Value::as_u64),
    );
    for required in json_string_array(minimum, "requiredResponsibilities") {
        assert!(
            evidence.responsibilities.contains(required),
            "{name}: missing semantic responsibility {required:?}; got {:?}",
            evidence.responsibilities
        );
    }
    for required in json_string_array(minimum, "requiredNativeParserResponsibilities") {
        assert!(
            evidence.responsibilities.contains(required),
            "{name}: search packet is missing parser-owned responsibility {required:?}: {:?}",
            evidence.responsibilities
        );
    }
    for required in json_string_array(minimum, "requiredProjectionNodeResponsibilities") {
        assert!(
            evidence.responsibilities.contains(required),
            "{name}: search packet is missing projected responsibility {required:?}: {:?}",
            evidence.responsibilities
        );
    }
    if let Some(max_ratio) = minimum
        .get("maxCompactToExactRatio")
        .and_then(Value::as_f64)
    {
        let ratio = compact_len as f64 / exact_len.max(1) as f64;
        assert!(
            ratio <= max_ratio,
            "{name}: compact/exact ratio {ratio:.3} exceeds {max_ratio:.3}"
        );
    }
}

fn assert_min_usize(name: &str, label: &str, actual: usize, expected: Option<u64>) {
    if let Some(expected) = expected {
        assert!(
            actual >= usize::try_from(expected).expect("minimum fits usize"),
            "{name}: {label} expected at least {expected}, got {actual}"
        );
    }
}

fn assert_expected_compact_code(name: &str, scenario_dir: &Path, check: &Value, actual: &str) {
    if let Some(fixture) = check.get("codeFixture").and_then(Value::as_str) {
        let expected = read_fixture_text(scenario_dir, fixture);
        assert_eq!(
            actual,
            expected.trim_end(),
            "{name}: compact code fixture drift"
        );
    }
    if let Some(expected) = check.get("codeEquals").and_then(Value::as_str) {
        assert_eq!(actual, expected, "{name}: compact code mismatch");
    }
    if check
        .get("assertRustfmtStyleCompact")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let lines = actual.lines().collect::<Vec<_>>();
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("    ") && !line.trim().is_empty()),
            "{name}: compact code lost rustfmt-style nested indentation: {actual}"
        );
        assert!(
            lines.iter().any(|line| line.trim_end().ends_with('{')),
            "{name}: compact code lost visible block opening brace: {actual}"
        );
        assert!(
            lines.iter().any(|line| line.trim() == "}"),
            "{name}: compact code lost visible block closing brace: {actual}"
        );
    }
}

#[derive(Debug)]
struct SearchItemEvidence<'a> {
    responsibilities: BTreeSet<&'a str>,
}

fn search_item_evidence(item: &Value) -> SearchItemEvidence<'_> {
    let responsibilities = item["fields"]["responsibilities"]
        .as_array()
        .unwrap_or_else(|| panic!("search item responsibilities must be an array: {item}"))
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    SearchItemEvidence { responsibilities }
}

fn configured_compact_code(name: &str, scenario_dir: &Path, check: &Value) -> String {
    let fixture = check
        .get("codeFixture")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{name}: compact check must pin codeFixture"));
    let configured = read_fixture_text(scenario_dir, fixture);
    assert!(
        !configured.trim().is_empty(),
        "{name}: configured compact negative preimage must not be empty"
    );
    configured.trim_end().to_string()
}

fn assert_compact_code_rejected_as_preimage(
    name: &str,
    root: &Path,
    target: &Value,
    exact_source: &str,
    compact_code: &str,
) {
    let packet = json!({
        "target": target,
        "operation": {
            "op": "replace_item",
            "snippet": exact_source,
            "expectedSnippet": compact_code,
            "maxEdits": 1
        }
    })
    .to_string();
    let output = run_cli_with_stdin(
        [
            OsString::from("ast-patch"),
            OsString::from("dry-run"),
            OsString::from("--packet"),
            OsString::from("-"),
            root.as_os_str().to_os_string(),
        ],
        &packet,
    );
    assert!(output.status.success(), "{name}: {output:?}");
    let receipt = serde_json::from_slice::<Value>(&output.stdout).expect("receipt JSON");
    assert_eq!(
        receipt["status"], "failed",
        "{name}: compact code should not verify as exact preimage: {receipt}"
    );
    assert_eq!(
        receipt["failureKind"], "target-preimage-mismatch",
        "{name}: compact code should fail at preimage match: {receipt}"
    );
}

fn assert_saved_compact_apply_patch_passes(name: &str, scenario_dir: &Path, fixture: &str) {
    let save_patch = read_json(&scenario_dir.join(fixture));
    let path = save_patch["path"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: saved compact patch path must be a string"));
    let query = save_patch["query"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: saved compact patch query must be a string"));
    let op = save_patch["op"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: saved compact patch op must be a string"));
    assert_eq!(op, "replace_item", "{name}: saved compact patch op");

    let input = tempfile::tempdir()
        .unwrap_or_else(|error| panic!("{name}: failed to create tempdir: {error}"));
    copy_dir_recursive(&scenario_dir.join("input"), input.path());

    let search_packet = run_owner_item_search_json(input.path(), path, query);
    let item = select_search_item(
        name,
        "saved compact input",
        &search_packet,
        save_patch.get("targetKind").and_then(Value::as_str),
        save_patch.get("targetName").and_then(Value::as_str),
    );
    let input_evidence = search_item_evidence(item);
    let input_fixture = save_patch["inputCompactFixture"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: inputCompactFixture must be a string"));
    let input_compact = read_fixture_text(scenario_dir, input_fixture);

    let target = ast_patch_target_from_search_item(item);
    let preimage = exact_read_from_target(input.path(), &target);
    if let Some(minimum) = save_patch.get("inputMinimumFunctionalComplexity") {
        assert_item_functional_complexity(
            name,
            "saved compact input",
            minimum,
            &input_evidence,
            input_compact.trim_end().len(),
            preimage.len(),
        );
    }

    let replacement_read = save_patch["replacementRead"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: replacementRead must be a string"));
    let replacement = exact_read_from_locator(&scenario_dir.join("expected"), replacement_read);
    let packet = json!({
        "target": target,
        "operation": {
            "op": "replace_item",
            "snippet": replacement,
            "expectedSnippet": preimage,
            "maxEdits": save_patch.get("maxEdits").and_then(Value::as_u64).unwrap_or(1)
        }
    })
    .to_string();
    let output = run_cli_with_stdin(
        [
            OsString::from("ast-patch"),
            OsString::from("apply"),
            OsString::from("--packet"),
            OsString::from("-"),
            input.path().as_os_str().to_os_string(),
        ],
        &packet,
    );
    assert!(
        output.status.success(),
        "{name}: saved apply patch failed: {output:?}"
    );
    assert!(
        output.stdout.is_empty(),
        "{name}: successful saved ast-patch apply should not print a receipt: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let expected = snapshot_dir(&scenario_dir.join("expected"));
    let actual = snapshot_dir(input.path());
    assert_eq!(
        actual, expected,
        "{name}: saved compact patch should match expected tree"
    );

    let expected_search_packet = run_owner_item_search_json(input.path(), path, query);
    let expected_item = select_search_item(
        name,
        "saved compact expected",
        &expected_search_packet,
        save_patch.get("targetKind").and_then(Value::as_str),
        save_patch.get("targetName").and_then(Value::as_str),
    );
    let expected_evidence = search_item_evidence(expected_item);
    let expected_fixture = save_patch["expectedCompactFixture"]
        .as_str()
        .unwrap_or_else(|| panic!("{name}: expectedCompactFixture must be a string"));
    let expected_compact = read_fixture_text(scenario_dir, expected_fixture);
    if let Some(minimum) = save_patch.get("expectedMinimumFunctionalComplexity") {
        let expected_target = ast_patch_target_from_search_item(expected_item);
        let expected_exact = exact_read_from_target(input.path(), &expected_target);
        assert_item_functional_complexity(
            name,
            "saved compact expected",
            minimum,
            &expected_evidence,
            expected_compact.trim_end().len(),
            expected_exact.len(),
        );
    }
}

fn assert_item_functional_complexity(
    name: &str,
    label: &str,
    minimum: &Value,
    evidence: &SearchItemEvidence<'_>,
    compact_len: usize,
    exact_len: usize,
) {
    let check_name = format!("{name}: {label}");
    assert_minimum_functional_complexity(&check_name, minimum, evidence, compact_len, exact_len);
}

fn select_search_item<'a>(
    name: &str,
    label: &str,
    search_packet: &'a Value,
    kind: Option<&str>,
    item_name: Option<&str>,
) -> &'a Value {
    let items = search_packet["items"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: {label} search packet items must be an array"));
    if kind.is_none() && item_name.is_none() {
        return items
            .first()
            .unwrap_or_else(|| panic!("{name}: {label} search packet had no items"));
    }
    items
        .iter()
        .find(|item| {
            kind.is_none_or(|kind| item["kind"].as_str() == Some(kind))
                && item_name
                    .is_none_or(|item_name| item["name"].as_str() == Some(item_name))
        })
        .unwrap_or_else(|| {
            panic!(
                "{name}: {label} search packet did not contain item kind={kind:?} name={item_name:?}: {search_packet}"
            )
        })
}

fn run_owner_item_search_json(root: &Path, path: &str, query: &str) -> Value {
    let output = run_cli([
        "search".as_ref(),
        "owner".as_ref(),
        path.as_ref(),
        "items".as_ref(),
        "--query".as_ref(),
        query.as_ref(),
        "--json".as_ref(),
        root.as_os_str(),
    ]);
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice::<Value>(&output.stdout).expect("owner item search packet JSON")
}

fn ast_patch_target_from_search_item(item: &Value) -> Value {
    let owner_path = item["ownerPath"]
        .as_str()
        .expect("search item ownerPath must be a string");
    let read = item["fields"]["read"]
        .as_str()
        .expect("search item fields.read must be a string");
    let locator = item["fields"]["structuralSelector"]
        .as_str()
        .expect("search item fields.structuralSelector must be a string");
    let item_name = item["name"]
        .as_str()
        .expect("search item name must be a string");
    let item_kind = item["kind"]
        .as_str()
        .expect("search item kind must be a string");
    json!({
        "ownerPath": owner_path,
        "locator": locator,
        "read": read,
        "itemName": item_name,
        "itemKind": item_kind,
    })
}

fn json_string_array<'a>(scenario: &'a Value, key: &str) -> Vec<&'a str> {
    scenario
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .unwrap_or_else(|| panic!("scenario.{key} entries must be strings"))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn json_array<'a>(scenario: &'a Value, key: &str) -> Vec<&'a Value> {
    scenario
        .get(key)
        .and_then(Value::as_array)
        .map(|values| values.iter().collect())
        .unwrap_or_default()
}

fn assert_receipt_verification_contains(name: &str, receipt: &Value, expected: &str) {
    let verification = receipt["verification"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: receipt.verification must be an array"));
    assert!(
        verification.iter().any(|value| value == expected),
        "{name}: missing verification {expected}: {verification:?}"
    );
}

fn assert_receipt_verification_excludes(name: &str, receipt: &Value, forbidden: &str) {
    let verification = receipt["verification"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: receipt.verification must be an array"));
    assert!(
        !verification.iter().any(|value| value == forbidden),
        "{name}: forbidden verification {forbidden}: {verification:?}"
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    Dir,
    File(Vec<u8>),
}

const SYNTHETIC_AST_PATCH_SCENARIO_MANIFEST: &str =
    "[package]\nname = \"ast-patch-scenario\"\nversion = \"0.0.0\"\nedition = \"2021\"\n";

fn snapshot_dir(root: &Path) -> BTreeMap<PathBuf, Entry> {
    let mut entries = BTreeMap::new();
    if root.is_dir() {
        snapshot_dir_recursive(root, root, &mut entries);
    }
    entries
}

fn snapshot_dir_recursive(base: &Path, dir: &Path, entries: &mut BTreeMap<PathBuf, Entry>) {
    let mut children = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read dir {}: {error}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .collect::<Vec<_>>();
    children.sort();
    for path in children {
        if is_generated_fixture_artifact(&path) || is_synthetic_ast_patch_scenario_manifest(&path) {
            continue;
        }
        let rel = path
            .strip_prefix(base)
            .unwrap_or_else(|error| panic!("strip prefix {}: {error}", path.display()))
            .to_path_buf();
        let metadata = fs::metadata(&path)
            .unwrap_or_else(|error| panic!("metadata {}: {error}", path.display()));
        if metadata.is_dir() {
            entries.insert(rel, Entry::Dir);
            snapshot_dir_recursive(base, &path, entries);
        } else if metadata.is_file() {
            let bytes = fs::read(&path)
                .unwrap_or_else(|error| panic!("read file {}: {error}", path.display()));
            entries.insert(rel, Entry::File(bytes));
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    let mut children = fs::read_dir(src)
        .unwrap_or_else(|error| panic!("read input dir {}: {error}", src.display()))
        .map(|entry| entry.expect("input dir entry").path())
        .collect::<Vec<_>>();
    children.sort();
    for path in children {
        if is_generated_fixture_artifact(&path) {
            continue;
        }
        let dest = dst.join(path.file_name().expect("input path file name"));
        let metadata = fs::metadata(&path)
            .unwrap_or_else(|error| panic!("metadata {}: {error}", path.display()));
        if metadata.is_dir() {
            fs::create_dir_all(&dest)
                .unwrap_or_else(|error| panic!("create dir {}: {error}", dest.display()));
            copy_dir_recursive(&path, &dest);
        } else if metadata.is_file() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .unwrap_or_else(|error| panic!("create dir {}: {error}", parent.display()));
            }
            fs::copy(&path, &dest).unwrap_or_else(|error| {
                panic!("copy {} to {}: {error}", path.display(), dest.display())
            });
        }
    }
    if dst.join("src").is_dir() && !dst.join("Cargo.toml").exists() {
        fs::write(
            dst.join("Cargo.toml"),
            SYNTHETIC_AST_PATCH_SCENARIO_MANIFEST,
        )
        .unwrap_or_else(|error| panic!("write fixture Cargo.toml in {}: {error}", dst.display()));
    }
}

fn is_generated_fixture_artifact(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "target" || name == "Cargo.lock")
}

fn is_synthetic_ast_patch_scenario_manifest(path: &Path) -> bool {
    if path.file_name().and_then(|name| name.to_str()) != Some("Cargo.toml") {
        return false;
    }
    fs::read_to_string(path).is_ok_and(|content| content == SYNTHETIC_AST_PATCH_SCENARIO_MANIFEST)
}

fn read_json(path: &Path) -> Value {
    let content =
        fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&content)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn read_fixture_text(scenario_dir: &Path, relative_path: &str) -> String {
    let path = scenario_dir.join(relative_path);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn exact_read_from_target(root: &Path, target: &Value) -> String {
    let read = target["read"]
        .as_str()
        .expect("patchSafety.target.read string");
    exact_read_from_locator(root, read)
}

fn exact_read_from_locator(root: &Path, read: &str) -> String {
    let (path, start, end) = parse_read_locator(read);
    let source_path = root.join(path);
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", source_path.display()));
    source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line_number = index + 1;
            (line_number >= start && line_number <= end).then_some(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn parse_read_locator(read: &str) -> (&str, usize, usize) {
    let mut parts = read.rsplitn(3, ':');
    let end = parts
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .expect("read end line");
    let start = parts
        .next()
        .and_then(|value| value.parse::<usize>().ok())
        .expect("read start line");
    let path = parts.next().expect("read path");
    (path, start, end)
}

fn env_string(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name).map(PathBuf::from)
}

fn env_flag(name: &str) -> bool {
    matches!(
        env::var(name).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

fn copy_target_file_to_temp(source_root: &Path, temp_root: &Path, relative_path: &str) {
    let source_path = source_root.join(relative_path);
    let dest_path = temp_root.join(relative_path);
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("create dir {}: {error}", parent.display()));
    }
    fs::copy(&source_path, &dest_path).unwrap_or_else(|error| {
        panic!(
            "copy real checkout source {} to {}: {error}",
            source_path.display(),
            dest_path.display()
        )
    });
}

fn scenario_name(dir: &Path) -> String {
    dir.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<scenario>")
        .to_string()
}
