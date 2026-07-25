use super::package::package_header_parts;

pub(super) struct SearchBlock {
    header: String,
    details: Vec<String>,
}

enum BlockEntry {
    Raw(SearchBlock),
    PackageGroup(PackageBlockGroup),
}

struct PackageBlockGroup {
    prefix: String,
    suffix: String,
    packages: Vec<String>,
    details: Vec<String>,
}

pub(super) fn parse_blocks(rendered: &str) -> Vec<SearchBlock> {
    let mut blocks = Vec::<SearchBlock>::new();
    let mut current = None::<SearchBlock>;
    for line in rendered.lines() {
        if !line.starts_with('|') {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
            current = Some(SearchBlock {
                header: line.to_string(),
                details: Vec::new(),
            });
        } else if let Some(block) = &mut current {
            block.details.push(line.to_string());
        } else {
            current = Some(SearchBlock {
                header: String::new(),
                details: vec![line.to_string()],
            });
        }
    }
    if let Some(block) = current {
        blocks.push(block);
    }
    blocks
}

pub(super) fn render_terse_package_blocks(blocks: Vec<SearchBlock>) -> Vec<SearchBlock> {
    let mut entries = Vec::<BlockEntry>::new();
    for block in blocks {
        let Some(parts) = package_header_parts(&block.header) else {
            entries.push(BlockEntry::Raw(block));
            continue;
        };
        if let Some(BlockEntry::PackageGroup(group)) = entries.iter_mut().rev().find(|entry| {
            matches!(
                entry,
                BlockEntry::PackageGroup(group)
                    if group.prefix == parts.prefix && group.suffix == parts.suffix
            )
        }) {
            group.packages.push(parts.package);
            group.details.extend(block.details);
            continue;
        }
        entries.push(BlockEntry::PackageGroup(PackageBlockGroup {
            prefix: parts.prefix,
            suffix: parts.suffix,
            packages: vec![parts.package],
            details: block.details,
        }));
    }
    entries
        .into_iter()
        .map(|entry| match entry {
            BlockEntry::Raw(block) => block,
            BlockEntry::PackageGroup(group) => group.render(),
        })
        .collect()
}

impl PackageBlockGroup {
    fn render(self) -> SearchBlock {
        SearchBlock {
            header: format!(
                "{}pkg={}{}",
                self.prefix,
                self.packages.join(","),
                self.suffix
            ),
            details: self.details,
        }
    }
}

pub(super) fn render_blocks(blocks: Vec<SearchBlock>) -> String {
    let mut rendered = String::new();
    for block in blocks {
        if !block.header.is_empty() {
            rendered.push_str(&block.header);
            rendered.push('\n');
        }
        for detail in block.details {
            rendered.push_str(&detail);
            rendered.push('\n');
        }
    }
    rendered
}
