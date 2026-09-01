use std::fmt::Write;
use std::path::Path;

use crate::AspRustConfig;

use super::RustSearchOptions;
use super::context::exact_owner_path_matches;
use super::format::{display_project_path, package_label, package_roots_for_request};

pub(super) fn render_exact_path_owner_seed_view(
    project_root: &Path,
    config: &AspRustConfig,
    query: &str,
    options: &RustSearchOptions,
) -> Result<Option<String>, String> {
    let package_roots =
        package_roots_for_request(project_root, config, options.package.as_deref())?;
    let matches = exact_owner_path_matches(project_root, &package_roots, query);
    if matches.is_empty() {
        return Ok(None);
    }
    let mut rendered = String::new();
    for (package_root, path) in matches {
        let owner_path = display_project_path(&package_root, &path);
        let tests = fast_related_test_paths(&package_root, config, &owner_path);
        let mut block = format!(
            "[search-owner] q={} pkg={} own=1 item=0\n",
            query,
            package_label(project_root, &package_root),
        );
        let _ = writeln!(block, "|seed owner:{owner_path}");
        if !tests.is_empty() {
            let _ = writeln!(block, "|seed tests:{}", tests.join(","));
        }
        let mut seeds = vec![format!("owner:{owner_path}")];
        seeds.extend(tests.iter().map(|test| format!("tests:{test}")));
        let _ = writeln!(
            block,
            "|synthesis algorithm=fast-exact-owner-frontier scope=owner summary=exact-owner-frontier selected_owners=1 edit_frontier={} test_frontier={} window_set={} seeds={}",
            owner_path,
            if tests.is_empty() {
                "-".to_string()
            } else {
                tests.join(",")
            },
            seeds.join(","),
            seeds.join(","),
        );
        rendered.push_str(&block);
    }
    Ok(Some(rendered))
}

fn fast_related_test_paths(
    package_root: &Path,
    config: &AspRustConfig,
    owner_path: &str,
) -> Vec<String> {
    let owner_tokens = fast_owner_path_tokens(owner_path);
    let mut tests = Vec::new();
    for test_dir in &config.test_dir_names {
        collect_bounded_test_paths(
            &package_root.join(test_dir),
            package_root,
            0,
            &mut tests,
            64,
        );
        if tests.len() >= 64 {
            break;
        }
    }
    tests.sort_by_key(|test_path| {
        std::cmp::Reverse(
            owner_tokens
                .iter()
                .filter(|token| test_path.to_ascii_lowercase().contains(token.as_str()))
                .count(),
        )
    });
    tests.truncate(8);
    tests
}

fn collect_bounded_test_paths(
    dir: &Path,
    package_root: &Path,
    depth: usize,
    tests: &mut Vec<String>,
    limit: usize,
) {
    if tests.len() >= limit || depth > 1 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries = entries
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if tests.len() >= limit {
            break;
        }
        if path.is_dir() {
            collect_bounded_test_paths(&path, package_root, depth + 1, tests, limit);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            tests.push(display_project_path(package_root, &path));
        }
    }
}

fn fast_owner_path_tokens(owner_path: &str) -> Vec<String> {
    owner_path
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .map(str::to_ascii_lowercase)
        .filter(|token| token.len() >= 3 && token != "src" && token != "lib")
        .collect()
}
