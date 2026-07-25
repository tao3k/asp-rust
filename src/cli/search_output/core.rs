// Compact output controls shared by CLI search views.
use super::blocks;
use super::package::PackageHeaderParts;

use std::collections::{BTreeMap, BTreeSet};

pub(in crate::cli) struct SearchOutputControls<'a> {
    pub(in crate::cli) depth: Option<usize>,
    pub(in crate::cli) output_view: Option<&'a str>,
    pub(in crate::cli) packet_kind: Option<&'a str>,
    pub(in crate::cli) seeds: Option<usize>,
}

pub(crate) fn apply_search_output_controls(
    controls: SearchOutputControls<'_>,
    rendered: &str,
) -> String {
    let rendered = apply_packet_kind(rendered, controls.packet_kind);
    if controls.depth == Some(0) {
        return render_header_only(&rendered);
    }
    if controls.output_view == Some("seeds") {
        return render_search_seed_view(&rendered, controls.seeds);
    }
    if controls.output_view == Some("both") {
        return render_search_both_view(&rendered);
    }
    rendered
}

fn apply_packet_kind(rendered: &str, packet_kind: Option<&str>) -> String {
    let Some(packet_kind) = packet_kind else {
        return rendered.to_string();
    };
    let Some(rest) = rendered.strip_prefix("[search-owner]") else {
        return rendered.to_string();
    };
    format!("[{packet_kind}]{rest}")
}

fn render_header_only(rendered: &str) -> String {
    let mut header_only = rendered
        .lines()
        .filter(|line| !line.starts_with('|'))
        .collect::<Vec<_>>()
        .join("\n");
    if !header_only.is_empty() {
        header_only.push('\n');
    }
    header_only
}

fn render_search_both_view(rendered: &str) -> String {
    blocks::render_blocks(blocks::render_terse_package_blocks(blocks::parse_blocks(
        rendered,
    )))
}

fn render_search_seed_view(rendered: &str, seed_limit: Option<usize>) -> String {
    const DEFAULT_SEED_LIMIT: usize = 8;

    let SearchSeeds {
        headers,
        facts,
        mut seeds,
        synthesis,
        notes,
    } = collect_search_seeds(rendered);
    for line in &synthesis {
        seeds.extend(synthesis_seed_lines(line));
    }
    seeds.sort_by(|left, right| {
        seed_priority(left)
            .cmp(&seed_priority(right))
            .then_with(|| seed_secondary_rank(left).cmp(&seed_secondary_rank(right)))
            .then_with(|| left.len().cmp(&right.len()))
            .then_with(|| left.cmp(right))
    });
    let seed_limit = seed_limit.unwrap_or(DEFAULT_SEED_LIMIT);
    let has_positive_header = headers
        .iter()
        .any(|header| header_has_positive_count(header));
    let mut terse = String::new();
    for header in render_terse_package_headers(headers.clone()) {
        terse.push_str(&header);
        terse.push('\n');
    }
    for fact in facts {
        terse.push_str(&fact);
        terse.push('\n');
    }
    for line in render_terse_seed_lines(bounded_seeds(seeds, seed_limit)) {
        terse.push_str(&line);
        terse.push('\n');
    }
    let notes = if has_positive_header {
        notes
            .into_iter()
            .filter(|note| !note.contains("kind=not-found") && !note.contains("seeds_truncated="))
            .collect::<Vec<_>>()
    } else {
        notes
    };
    for note in notes.into_iter().take(1) {
        terse.push_str(&note);
        terse.push('\n');
    }
    terse
}

fn seed_priority(seed: &str) -> usize {
    if seed.starts_with("dependency:") || seed.starts_with("deps:") || seed.starts_with("import:") {
        0
    } else if seed.starts_with("feature:") || seed.starts_with("features:") {
        1
    } else if seed.starts_with("cfg:") {
        2
    } else if seed.starts_with("package:") || seed.starts_with("prime:") {
        3
    } else if seed.starts_with("owner:") {
        4
    } else if seed == "tests" || seed.starts_with("tests:") {
        5
    } else if seed.starts_with("docs:") || seed.starts_with("docs-use:") {
        6
    } else if seed.starts_with("text:") {
        7
    } else if seed.starts_with("symbol:") {
        8
    } else {
        9
    }
}

fn seed_secondary_rank(seed: &str) -> usize {
    let target = seed.rsplit(':').next().unwrap_or(seed);
    if seed.starts_with("cfg:feature:") {
        0
    } else if seed == "cfg:test" || matches!(target, "default" | "full" | "all") {
        3
    } else if seed.starts_with("owner:tests/task_") {
        0
    } else if seed.starts_with("owner:tests/") {
        1
    } else if seed.starts_with("owner:")
        && (target.ends_with("/lib.rs") || target.ends_with("/main.rs"))
    {
        2
    } else if target.chars().next().is_some_and(char::is_uppercase) || target.contains('-') {
        0
    } else if target.contains('_') {
        2
    } else {
        1
    }
}

fn header_has_positive_count(header: &str) -> bool {
    header
        .split_whitespace()
        .filter_map(|field| field.split_once('='))
        .any(|(key, value)| {
            matches!(
                key,
                "api" | "calls" | "cfg" | "dep" | "defs" | "hit" | "item" | "own" | "tests"
            ) && value.parse::<usize>().is_ok_and(|count| count > 0)
        })
}

fn bounded_seeds(seeds: Vec<String>, seed_limit: usize) -> Vec<String> {
    let first_pass = first_pass_seed_indices(&seeds, seed_limit);
    let first_pass_set = first_pass.iter().copied().collect::<BTreeSet<_>>();
    let remaining = seed_limit.saturating_sub(first_pass.len());
    first_pass
        .into_iter()
        .chain(
            (0..seeds.len())
                .filter(|index| !first_pass_set.contains(index))
                .take(remaining),
        )
        .map(|index| seeds[index].clone())
        .collect()
}

fn first_pass_seed_indices(seeds: &[String], seed_limit: usize) -> Vec<usize> {
    seeds
        .iter()
        .enumerate()
        .scan(
            BTreeMap::<usize, usize>::new(),
            |priority_counts, (index, seed)| {
                let priority = seed_priority(seed);
                let count = priority_counts.entry(priority).or_default();
                if *count >= first_pass_limit_for_priority(priority) {
                    return Some(None);
                }
                *count += 1;
                Some(Some(index))
            },
        )
        .flatten()
        .take(seed_limit)
        .collect()
}

fn first_pass_limit_for_priority(priority: usize) -> usize {
    if priority == 0 { 2 } else { 1 }
}

enum HeaderEntry {
    Raw(String),
    PackageGroup(PackageHeaderGroup),
}

struct PackageHeaderGroup {
    pub(in crate::cli::search_output) prefix: String,
    pub(in crate::cli::search_output) suffix: String,
    packages: Vec<String>,
}

fn render_terse_package_headers(headers: Vec<String>) -> Vec<String> {
    let mut entries = Vec::<HeaderEntry>::new();
    let mut package_group_indices = BTreeMap::<(String, String), usize>::new();
    for header in headers {
        let Some(parts) = package_header_parts(&header) else {
            entries.push(HeaderEntry::Raw(header));
            continue;
        };
        let group_key = (parts.prefix.clone(), parts.suffix.clone());
        if let Some(entry_index) = package_group_indices.get(&group_key)
            && let HeaderEntry::PackageGroup(group) = &mut entries[*entry_index]
        {
            group.packages.push(parts.package);
            continue;
        }
        package_group_indices.insert(group_key, entries.len());
        entries.push(HeaderEntry::PackageGroup(PackageHeaderGroup {
            prefix: parts.prefix,
            suffix: parts.suffix,
            packages: vec![parts.package],
        }));
    }
    entries
        .into_iter()
        .map(|entry| match entry {
            HeaderEntry::Raw(header) => header,
            HeaderEntry::PackageGroup(group) => group.render(),
        })
        .collect()
}

impl PackageHeaderGroup {
    fn render(self) -> String {
        format!(
            "{}pkg={}{}",
            self.prefix,
            self.packages.join(","),
            self.suffix
        )
    }
}

pub(in crate::cli::search_output) fn package_header_parts(
    line: &str,
) -> Option<PackageHeaderParts> {
    let mut cursor = 0;
    for field in line.split_whitespace() {
        let field_start = cursor + line[cursor..].find(field)?;
        let field_end = field_start + field.len();
        cursor = field_end;
        if let Some(package) = field.strip_prefix("pkg=") {
            return Some(PackageHeaderParts {
                prefix: line[..field_start].to_string(),
                package: package.to_string(),
                suffix: line[field_end..].to_string(),
            });
        }
    }
    None
}

enum SeedLine {
    Raw(String),
    Group { kind: String, targets: Vec<String> },
}

fn render_terse_seed_lines(seeds: Vec<String>) -> Vec<String> {
    let mut entries = Vec::<SeedLine>::new();
    let mut seed_group_indices = BTreeMap::<String, usize>::new();
    for seed in seeds {
        let Some((kind, target)) = groupable_seed_parts(&seed) else {
            entries.push(SeedLine::Raw(seed));
            continue;
        };
        if let Some(entry_index) = seed_group_indices.get(kind)
            && let SeedLine::Group { targets, .. } = &mut entries[*entry_index]
        {
            targets.push(target.to_string());
            continue;
        }
        seed_group_indices.insert(kind.to_string(), entries.len());
        entries.push(SeedLine::Group {
            kind: kind.to_string(),
            targets: vec![target.to_string()],
        });
    }
    entries
        .into_iter()
        .map(|entry| match entry {
            SeedLine::Raw(seed) => format!("|seed {seed}"),
            SeedLine::Group { kind, targets } => format!("|seed {kind}:{}", targets.join(",")),
        })
        .collect()
}

fn graph_seed_definition(line: &str) -> Option<(&str, String)> {
    let (id, payload) = line.split_once('=')?;
    if id.is_empty() || !id.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return None;
    }
    let (typed_target, action) = payload.rsplit_once('!')?;
    let (kind, target) = typed_target.split_once(':')?;
    let seed_kind = match (kind, action) {
        ("owner", "owner") => "owner",
        ("package", "prime") => "package",
        ("test" | "tests", "tests") => "tests",
        _ => return None,
    };
    Some((id, format!("{seed_kind}:{target}")))
}

fn synthesis_seed_lines(line: &str) -> Vec<String> {
    let Some(raw_seeds) = line_protocol_field(line, "seeds") else {
        return Vec::new();
    };
    raw_seeds
        .split(',')
        .filter_map(|seed| {
            let seed = seed.trim().trim_matches('"');
            let (kind, target) = seed.split_once(':')?;
            if target.is_empty() {
                return None;
            }
            Some(format!("{kind}:{target}"))
        })
        .collect()
}

fn line_protocol_field<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    let needle = format!("{field}=");
    let start = line.find(&needle)? + needle.len();
    line[start..].split_whitespace().next()
}

fn groupable_seed_parts(seed: &str) -> Option<(&str, &str)> {
    let (kind, target) = seed.split_once(':')?;
    if !groupable_seed_kind(kind) || target.is_empty() || target.contains(',') {
        return None;
    }
    Some((kind, target))
}

fn groupable_seed_kind(kind: &str) -> bool {
    matches!(
        kind,
        "cfg"
            | "dependency"
            | "deps"
            | "docs"
            | "docs-use"
            | "feature"
            | "features"
            | "import"
            | "owner"
            | "package"
            | "prime"
            | "symbol"
            | "text"
            | "tests"
    )
}

struct SearchSeeds {
    headers: Vec<String>,
    facts: Vec<String>,
    seeds: Vec<String>,
    synthesis: Vec<String>,
    notes: Vec<String>,
}

fn collect_search_seeds(rendered: &str) -> SearchSeeds {
    rendered
        .lines()
        .fold(SearchSeedAccumulator::default(), |mut accumulator, line| {
            accumulator.record_line(line);
            accumulator
        })
        .finish()
}

#[derive(Default)]
struct SearchSeedAccumulator {
    headers: Vec<String>,
    facts: Vec<String>,
    seeds: Vec<String>,
    synthesis: Vec<String>,
    notes: Vec<String>,
    seen: BTreeSet<String>,
    graph_nodes: BTreeMap<String, String>,
    current_package: Option<String>,
}

impl SearchSeedAccumulator {
    fn record_line(&mut self, line: &str) {
        if !line.starts_with('|') {
            if self.record_graph_line(line) {
                return;
            }
            self.current_package = package_from_header(line);
            self.headers.push(line.to_string());
            return;
        }
        if line.starts_with("|note ") && !line.contains("seeds_truncated=") {
            self.notes.push(line.to_string());
        }
        if line.starts_with("|query ") || line.starts_with("|fact ") {
            if self.seen.insert(format!("evidence:{line}")) {
                self.facts.push(line.to_string());
            }
            return;
        }
        if line.starts_with("|synthesis ") {
            self.synthesis.push(line.to_string());
            return;
        }
        if let Some(seed) = line.strip_prefix("|seed ") {
            record_seed(seed, &mut self.seen, &mut self.seeds);
            return;
        }
        collect_next_seeds(
            line,
            self.current_package.as_deref(),
            &mut self.seen,
            &mut self.seeds,
        );
    }

    fn record_graph_line(&mut self, line: &str) -> bool {
        if line.starts_with("[search-graph]") {
            self.record_graph_fact(line);
            return true;
        }
        if graph_frontier_field(line).is_some() {
            self.record_graph_fact(line);
            return true;
        }
        if line.starts_with("rank=") {
            self.record_graph_fact(line);
            return true;
        }
        if let Some((id, seed)) = graph_seed_definition(line) {
            self.graph_nodes.insert(id.to_string(), seed);
            self.record_graph_fact(line);
            return true;
        }
        if is_compact_graph_fact_line(line) {
            self.record_graph_fact(line);
            return true;
        }
        false
    }

    fn record_graph_fact(&mut self, line: &str) {
        if self.seen.insert(format!("graph:{line}")) {
            self.facts.push(line.to_string());
        }
    }

    fn finish(self) -> SearchSeeds {
        SearchSeeds {
            headers: self.headers,
            facts: self.facts,
            seeds: self.seeds,
            synthesis: self.synthesis,
            notes: self.notes,
        }
    }
}

fn is_compact_graph_fact_line(line: &str) -> bool {
    line.starts_with("legend: ")
        || line.starts_with("aliases: graph:")
        || line.starts_with("entries=")
        || line.starts_with("nextCommand=")
        || line.starts_with("noOutput ")
        || line.starts_with("avoid=")
        || line.contains(">{")
}

fn graph_frontier_field(line: &str) -> Option<&str> {
    line.strip_prefix("frontier=").or_else(|| {
        let start = line.find("frontier=")? + "frontier=".len();
        line[start..].split_whitespace().next()
    })
}

fn record_seed(seed: &str, seen: &mut BTreeSet<String>, seeds: &mut Vec<String>) {
    let seed = seed.trim();
    if seed.is_empty() {
        return;
    }
    if seen.insert(seed.to_string()) {
        seeds.push(seed.to_string());
    }
}

fn collect_next_seeds(
    line: &str,
    package: Option<&str>,
    seen: &mut BTreeSet<String>,
    seeds: &mut Vec<String>,
) {
    let rest = if let Some(next_actions) = line.strip_prefix("|next ") {
        next_actions
    } else {
        let Some((_, rest)) = line.split_once("next=") else {
            return;
        };
        rest
    };
    let next = rest.split_whitespace().next().unwrap_or(rest);
    for seed in split_next_actions(next)
        .into_iter()
        .map(|seed| qualify_package_seed(&seed, package))
    {
        if seen.insert(seed.clone()) {
            seeds.push(seed);
        }
    }
}

fn split_next_actions(next: &str) -> Vec<String> {
    next.chars()
        .fold(NextActionSplit::default(), NextActionSplit::push)
        .finish()
}

#[derive(Default)]
struct NextActionSplit {
    actions: Vec<String>,
    current: String,
    brace_depth: usize,
}

impl NextActionSplit {
    fn push(mut self, character: char) -> Self {
        match character {
            ',' if self.brace_depth == 0 => self.flush_current(),
            '{' => {
                self.brace_depth += 1;
                self.current.push(character);
                self
            }
            '}' => {
                self.brace_depth = self.brace_depth.saturating_sub(1);
                self.current.push(character);
                self
            }
            _ => {
                self.current.push(character);
                self
            }
        }
    }

    fn finish(mut self) -> Vec<String> {
        self = self.flush_current();
        self.actions
    }

    fn flush_current(mut self) -> Self {
        let action = self.current.trim();
        if !action.is_empty() {
            self.actions.push(action.to_string());
        }
        self.current.clear();
        self
    }
}

fn package_from_header(line: &str) -> Option<String> {
    line.split_whitespace()
        .find_map(|field| {
            field
                .strip_prefix("pkg=")
                .or_else(|| field.strip_prefix("package="))
        })
        .filter(|package| !matches!(*package, "." | "-"))
        .map(ToOwned::to_owned)
}

fn qualify_package_seed(seed: &str, package: Option<&str>) -> String {
    let Some(package) = package else {
        return seed.to_string();
    };
    for prefix in ["owner:", "tests:", "ingest:", "text:"] {
        if let Some(target) = seed.strip_prefix(prefix)
            && package_seed_target_needs_prefix(target, package)
        {
            return format!("{prefix}{package}/{target}");
        }
    }
    seed.to_string()
}

fn package_seed_target_needs_prefix(target: &str, package: &str) -> bool {
    !target.starts_with('/')
        && !target.starts_with('<')
        && seed_target_looks_like_path(target)
        && target != package
        && !target
            .strip_prefix(package)
            .is_some_and(|rest| rest.starts_with('/'))
        && !target.contains("://")
}

fn seed_target_looks_like_path(target: &str) -> bool {
    target.contains('/') || target.ends_with(".rs")
}
