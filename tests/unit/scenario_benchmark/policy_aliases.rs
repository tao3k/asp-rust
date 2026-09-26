#[test]
fn agent_policy_schema_ids_do_not_expose_legacy_aliases() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();
    for relative_root in ["src", "tests", "docs"] {
        collect_legacy_agent_policy_aliases(&crate_root.join(relative_root), &mut offenders);
    }
    offenders.sort();

    assert!(
        offenders.is_empty(),
        "legacy agent policy aliases must not be public: {offenders:?}"
    );
}

fn collect_legacy_agent_policy_aliases(path: &std::path::Path, offenders: &mut Vec<String>) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    if metadata.is_dir() {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return;
        };
        if matches!(name, "target" | ".git") {
            return;
        }
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            collect_legacy_agent_policy_aliases(&entry.path(), offenders);
        }
        return;
    }

    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return;
    };
    if !matches!(extension, "rs" | "md" | "toml") {
        return;
    }
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    if contains_legacy_agent_policy_alias(&content) {
        offenders.push(path.display().to_string());
    }
}

fn contains_legacy_agent_policy_alias(content: &str) -> bool {
    let hyphen_alias = ["AGENT", "-R"].concat();
    let underscore_alias = ["AGENT", "_R"].concat();
    content
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '-' || character == '_')
        })
        .any(|token| {
            [&hyphen_alias, &underscore_alias].iter().any(|prefix| {
                token.strip_prefix(prefix.as_str()).is_some_and(|suffix| {
                    suffix
                        .chars()
                        .take(3)
                        .all(|character| character.is_ascii_digit())
                })
            })
        })
}
