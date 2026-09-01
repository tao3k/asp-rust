use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use asp_rust::{
    AspRustConfig, RustSearchOptions, RustSearchViewRequest,
    render_asp_rust_search_view_with_config,
};
use serde::Deserialize;
use tempfile::TempDir;

fn main() {
    microbench_query_owner_item_frontier_follows_git_owned_p95_budget();
    microbench_search_prime_seed_follows_git_owned_p95_budget();
    microbench_search_dependency_api_follows_git_owned_p95_budget();
}

fn microbench_query_owner_item_frontier_follows_git_owned_p95_budget() {
    let budget = microbench_budget("rust.query.owner-item-frontier-render");
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "bench-query-perf");
    fs::create_dir_all(root.join("src")).expect("src dir");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() -> usize { 1 }\n\npub fn beta() -> usize { alpha() + 1 }\n",
    )
    .expect("write source");
    let config = AspRustConfig::default();
    let options = RustSearchOptions {
        item_query: Some("alpha".to_string()),
        ..RustSearchOptions::default()
    };

    let stats = measure_microbench(&budget, || {
        let stdout = render_asp_rust_search_view_with_config(&RustSearchViewRequest {
            project_root: root,
            config: &config,
            view: "owner",
            query: Some("src/lib.rs"),
            options: &options,
        })
        .expect("render query owner item frontier");
        assert!(stdout.contains("[search-owner]"), "{stdout}");
        assert!(stdout.contains("itemQuery=alpha"), "{stdout}");
        assert!(stdout.contains("|hot alpha"), "{stdout}");
    });

    assert_microbench_within_budget("rust.query.owner-item-frontier-render", &stats, &budget);
}

fn microbench_search_prime_seed_follows_git_owned_p95_budget() {
    let budget = microbench_budget("rust.search.prime-seed-render");
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_manifest(root, "bench-search-perf");
    fs::create_dir_all(root.join("src")).expect("src dir");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn alpha() -> usize { 1 }\n\npub fn beta() -> usize { alpha() + 1 }\n",
    )
    .expect("write source");
    let config = AspRustConfig::default();
    let options = RustSearchOptions {
        output_view: Some("seeds".to_string()),
        ..RustSearchOptions::default()
    };

    let stats = measure_microbench(&budget, || {
        let stdout = render_asp_rust_search_view_with_config(&RustSearchViewRequest {
            project_root: root,
            config: &config,
            view: "prime",
            query: None,
            options: &options,
        })
        .expect("render search prime seeds");
        assert!(stdout.contains("[search-prime]"), "{stdout}");
        assert!(stdout.contains("|seed owner:src/lib.rs"), "{stdout}");
        assert!(stdout.contains("selected_owners=1"), "{stdout}");
    });

    assert_microbench_within_budget("rust.search.prime-seed-render", &stats, &budget);
}

fn microbench_search_dependency_api_follows_git_owned_p95_budget() {
    let budget = microbench_budget("rust.search.dependency-api-render");
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    write_dependency_manifest(root);
    write_dependency_sources(root, 64);
    let config = AspRustConfig::default();
    let options = RustSearchOptions::default();

    let stats = measure_microbench(&budget, || {
        let stdout = render_asp_rust_search_view_with_config(&RustSearchViewRequest {
            project_root: root,
            config: &config,
            view: "deps",
            query: Some("serde@1::Serialize"),
            options: &options,
        })
        .expect("render dependency API search");
        assert!(stdout.contains("[search-deps]"), "{stdout}");
        assert!(stdout.contains("versionScope=current"), "{stdout}");
        assert!(stdout.contains("hit_kind=dependency-api"), "{stdout}");
    });

    assert_microbench_within_budget("rust.search.dependency-api-render", &stats, &budget);
}

fn write_manifest(root: &Path, name: &str) {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("write manifest");
}

fn write_dependency_manifest(root: &Path) {
    fs::write(
        root.join("Cargo.toml"),
        "[package]\n\
         name = \"bench-search-deps\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [dependencies]\n\
         serde = { version = \"1\", features = [\"derive\"] }\n",
    )
    .expect("write dependency manifest");
}

fn write_dependency_sources(root: &Path, module_count: usize) {
    let src = root.join("src");
    fs::create_dir_all(&src).expect("src dir");
    let modules = (0..module_count)
        .map(|index| {
            fs::write(
                src.join(format!("module_{index}.rs")),
                format!(
                    "use serde::Serialize;\n\
                     #[derive(Serialize)]\n\
                     pub struct Thing{index} {{\n\
                         pub value: usize,\n\
                     }}\n\
                     pub fn encode_{index}(value: Thing{index}) -> String {{\n\
                         format!(\"{{:?}}\", value.value)\n\
                     }}\n"
                ),
            )
            .expect("write dependency source");
            format!("pub mod module_{index};")
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(src.join("lib.rs"), modules).expect("write lib modules");
}

#[derive(Debug, Deserialize)]
struct MicrobenchBudgetFile {
    cases: std::collections::BTreeMap<String, MicrobenchBudget>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MicrobenchBudget {
    warmup_iterations: usize,
    measure_iterations: usize,
    p95_max_ms: f64,
}

#[derive(Debug)]
struct MicrobenchStats {
    min: Duration,
    mean: Duration,
    median: Duration,
    p95: Duration,
    max: Duration,
    stddev_ms: f64,
}

fn microbench_budget(case_id: &str) -> MicrobenchBudget {
    let budget_file: MicrobenchBudgetFile =
        serde_json::from_str(include_str!("query_search.microbench.json"))
            .expect("parse microbench budget");
    budget_file
        .cases
        .get(case_id)
        .unwrap_or_else(|| panic!("missing microbench budget case {case_id}"))
        .clone()
}

fn measure_microbench(budget: &MicrobenchBudget, mut run: impl FnMut()) -> MicrobenchStats {
    for _ in 0..budget.warmup_iterations {
        run();
    }
    let mut samples = Vec::with_capacity(budget.measure_iterations);
    for _ in 0..budget.measure_iterations {
        let started_at = Instant::now();
        run();
        samples.push(started_at.elapsed());
    }
    samples.sort_unstable();
    let mean_seconds =
        samples.iter().map(Duration::as_secs_f64).sum::<f64>() / samples.len() as f64;
    let variance = samples
        .iter()
        .map(|sample| {
            let delta = sample.as_secs_f64() - mean_seconds;
            delta * delta
        })
        .sum::<f64>()
        / samples.len() as f64;
    MicrobenchStats {
        min: samples[0],
        mean: Duration::from_secs_f64(mean_seconds),
        median: percentile(&samples, 0.5),
        p95: percentile(&samples, 0.95),
        max: samples[samples.len() - 1],
        stddev_ms: variance.sqrt() * 1000.0,
    }
}

fn assert_microbench_within_budget(
    case_id: &str,
    stats: &MicrobenchStats,
    budget: &MicrobenchBudget,
) {
    if cfg!(debug_assertions) {
        return;
    }
    let p95_ms = stats.p95.as_secs_f64() * 1000.0;
    assert!(
        p95_ms <= budget.p95_max_ms,
        "{case_id} p95 {:.3}ms exceeded budget {:.3}ms; min={:.3} mean={:.3} median={:.3} max={:.3} stddev={:.3}",
        p95_ms,
        budget.p95_max_ms,
        stats.min.as_secs_f64() * 1000.0,
        stats.mean.as_secs_f64() * 1000.0,
        stats.median.as_secs_f64() * 1000.0,
        stats.max.as_secs_f64() * 1000.0,
        stats.stddev_ms
    );
}

fn percentile(sorted_samples: &[Duration], quantile: f64) -> Duration {
    let index = ((sorted_samples.len() as f64 * quantile).ceil() as usize)
        .saturating_sub(1)
        .min(sorted_samples.len() - 1);
    sorted_samples[index]
}
