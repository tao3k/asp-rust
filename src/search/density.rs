//! Generic compaction for search line-protocol packets.

use std::collections::BTreeMap;

#[must_use]
pub(crate) fn render_terse_search_packet(rendered: &str) -> String {
    let lines = render_terse_edge_blocks(render_terse_cfg_rows(rendered));
    if lines.is_empty() {
        String::new()
    } else {
        let mut terse = lines.join("\n");
        terse.push('\n');
        terse
    }
}

fn render_terse_cfg_rows(rendered: &str) -> Vec<String> {
    rendered
        .lines()
        .map(ToOwned::to_owned)
        .fold(TerseCfgProjection::default(), TerseCfgProjection::push_line)
        .finish()
}

#[derive(Default)]
struct TerseCfgProjection {
    rendered: Vec<String>,
    cfg_features: Vec<String>,
}

impl TerseCfgProjection {
    fn push_line(mut self, line: String) -> Self {
        if let Some(feature) = cfg_feature_token(&line) {
            if !self.cfg_features.contains(&feature) {
                self.cfg_features.push(feature);
            }
            return self;
        }
        self.flush_cfg_group();
        self.rendered.push(line);
        self
    }

    fn finish(mut self) -> Vec<String> {
        self.flush_cfg_group();
        self.rendered
    }

    fn flush_cfg_group(&mut self) {
        if let Some(line) = render_terse_cfg_feature_line(&self.cfg_features) {
            self.rendered.push(line);
        } else {
            self.rendered.extend(
                self.cfg_features
                    .iter()
                    .map(|feature| format!("|cfg feature:{feature} next=cfg:feature:{feature}")),
            );
        }
        self.cfg_features.clear();
    }
}

fn render_terse_edge_blocks(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .fold(
            EdgeBlockAccumulator::default(),
            EdgeBlockAccumulator::push_line,
        )
        .finish()
}

#[derive(Default)]
struct EdgeBlockAccumulator {
    rendered: Vec<String>,
    block: Vec<String>,
}

impl EdgeBlockAccumulator {
    fn push_line(mut self, line: String) -> Self {
        if line.starts_with('|') {
            self.block.push(line);
        } else {
            self.flush_block();
            self.rendered.push(line);
        }
        self
    }

    fn finish(mut self) -> Vec<String> {
        self.flush_block();
        self.rendered
    }

    fn flush_block(&mut self) {
        self.rendered
            .extend(render_terse_edge_block(std::mem::take(&mut self.block)));
    }
}

fn render_terse_edge_block(lines: Vec<String>) -> Vec<String> {
    if lines.len() < 3 {
        return lines;
    }
    let groups = edge_block_groups(&lines);
    if groups.is_empty() {
        return lines;
    }
    let mut replacements = vec![None::<String>; lines.len()];
    let mut removed = vec![false; lines.len()];
    for group in groups {
        let Some(terse_right) = render_terse_path_collection(&group.rights) else {
            continue;
        };
        let terse = format!("|edge {} {} {terse_right}", group.left, group.relation);
        let expanded_len = group
            .rights
            .iter()
            .map(|right| format!("|edge {} {} {right}", group.left, group.relation).len())
            .sum::<usize>()
            + group.rights.len().saturating_sub(1);
        if terse.len() >= expanded_len {
            continue;
        }
        let Some(index) = group.indices.first().copied() else {
            continue;
        };
        replacements[index] = Some(terse);
        for index in group.indices.into_iter().skip(1) {
            removed[index] = true;
        }
    }
    lines
        .into_iter()
        .enumerate()
        .filter_map(|(index, line)| {
            (!removed[index]).then(|| replacements[index].take().unwrap_or(line))
        })
        .collect()
}

fn edge_block_groups(lines: &[String]) -> Vec<EdgeBlockGroup> {
    let mut groups = Vec::<EdgeBlockGroup>::new();
    let mut group_indices = BTreeMap::<(String, String), usize>::new();
    for (index, line) in lines.iter().enumerate() {
        let Some((left, relation, right)) = edge_parts(line) else {
            continue;
        };
        let group_key = (left.clone(), relation.clone());
        if let Some(group_index) = group_indices.get(&group_key) {
            let group = &mut groups[*group_index];
            group.rights.push(right);
            group.indices.push(index);
        } else {
            group_indices.insert(group_key, groups.len());
            groups.push(EdgeBlockGroup {
                left,
                relation,
                rights: vec![right],
                indices: vec![index],
            });
        }
    }
    groups
}

struct EdgeBlockGroup {
    left: String,
    relation: String,
    rights: Vec<String>,
    indices: Vec<usize>,
}

fn cfg_feature_token(line: &str) -> Option<String> {
    let rest = line.strip_prefix("|cfg feature:")?;
    let (feature, next) = rest.split_once(' ')?;
    (cfg_feature_token_is_terse_safe(feature) && next == format!("next=cfg:feature:{feature}"))
        .then(|| feature.to_string())
}

fn cfg_feature_token_is_terse_safe(feature: &str) -> bool {
    !feature.is_empty()
        && feature
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn render_terse_cfg_feature_line(features: &[String]) -> Option<String> {
    if features.len() < 2 {
        return None;
    }
    let joined = features.join(",");
    let terse = format!("|cfg feature:{{{joined}}} next=cfg:feature:{{{joined}}}");
    let expanded_len = features
        .iter()
        .map(|feature| format!("|cfg feature:{feature} next=cfg:feature:{feature}").len())
        .sum::<usize>()
        + features.len().saturating_sub(1);
    (terse.len() < expanded_len).then_some(terse)
}

fn edge_parts(line: &str) -> Option<(String, String, String)> {
    let rest = line.strip_prefix("|edge ")?;
    let (left, rest) = rest.split_once(' ')?;
    let (relation, right) = rest.split_once(' ')?;
    if !relation.starts_with('-') || !relation.ends_with("->") {
        return None;
    }
    Some((left.to_string(), relation.to_string(), right.to_string()))
}

fn render_terse_path_collection(values: &[String]) -> Option<String> {
    if values.len() < 3 {
        return None;
    }
    let labeled_paths = values
        .iter()
        .map(|value| split_labeled_path(value))
        .collect::<Option<Vec<_>>>()?;
    let label = labeled_paths.first()?.0.clone();
    if labeled_paths
        .iter()
        .any(|(value_label, _)| value_label != &label)
    {
        return None;
    }
    let paths = labeled_paths
        .into_iter()
        .map(|(_, path)| path)
        .collect::<Vec<_>>();
    let prefix = common_path_dir(&paths)?;
    let suffixes = paths
        .iter()
        .map(|path| path.strip_prefix(&prefix).unwrap_or(path).to_string())
        .collect::<Vec<_>>();
    if suffixes.iter().any(|suffix| suffix.is_empty()) {
        return None;
    }
    Some(format!("{label}{prefix}{{{}}}", suffixes.join(",")))
}

fn split_labeled_path(value: &str) -> Option<(String, String)> {
    let (label, path) = value.split_once(':')?;
    if label.is_empty()
        || path.is_empty()
        || !label
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return None;
    }
    Some((format!("{label}:"), path.to_string()))
}

fn common_path_dir(paths: &[String]) -> Option<String> {
    let (prefix, _) = paths.first()?.rsplit_once('/')?;
    let mut prefix = format!("{prefix}/");
    while !paths.iter().all(|path| path.starts_with(&prefix)) {
        let trimmed = prefix.trim_end_matches('/');
        let (parent, _) = trimmed.rsplit_once('/')?;
        prefix = format!("{parent}/");
    }
    Some(prefix)
}
