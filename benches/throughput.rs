//! Throughput benchmark for mdlint.
//!
//! Generates a synthetic vault of markdown files and measures
//! parse + lint throughput in MB/s.
//!
//! Run with: cargo bench --bench throughput
//! Adjust sizes via env vars: BENCH_FILES=1000 BENCH_FILE_KB=4

use std::fs;
use std::path::Path;
use std::time::Instant;
use tempfile::TempDir;

fn generate_vault(dir: &Path, num_files: usize, file_kb: usize) {
    let target_bytes = file_kb * 1024;

    // Generate a template file content with varied link types
    let file_names: Vec<String> = (0..num_files).map(|i| format!("page-{i:04}")).collect();

    for (i, name) in file_names.iter().enumerate() {
        let mut content = String::with_capacity(target_bytes + 512);

        // Frontmatter
        content.push_str(&format!("---\ntitle: {name}\ntags:\n  - bench\n---\n\n"));

        // H1
        content.push_str(&format!("# {name}\n\n"));

        // Generate content until we hit the target size
        let mut section = 0;
        while content.len() < target_bytes {
            section += 1;

            // Heading (h2)
            content.push_str(&format!("## Section {section}\n\n"));

            // Paragraph with prose
            content.push_str("Lorem ipsum dolor sit amet, consectetur adipiscing elit. ");
            content.push_str("Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ");
            content.push_str("Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.\n\n");

            // Standard link to another file in the vault
            let target_idx = (i + section) % num_files;
            content.push_str(&format!(
                "- [See also page {}](./{}.md)\n",
                target_idx, file_names[target_idx]
            ));

            // Wikilink to another file
            let wiki_idx = (i + section * 2) % num_files;
            content.push_str(&format!("- [[{}]]\n", file_names[wiki_idx]));

            // Anchor link
            content.push_str(&format!(
                "- [Section ref](./{}.md#section-{})\n",
                file_names[target_idx],
                (section % 3) + 1
            ));

            // Self-anchor
            if section > 1 {
                content.push_str(&format!("- [Back to section {}](#section-{})\n", section - 1, section - 1));
            }

            // Broken link (every 10th section to simulate real vaults)
            if section % 10 == 0 {
                content.push_str("- [Broken](./does-not-exist.md)\n");
            }

            // Image link
            content.push_str("- ![img](./assets/image.png)\n");

            content.push('\n');

            // Code block (should be skipped by linter)
            content.push_str("```markdown\n[not a link](./fake.md)\n[[not-a-wikilink]]\n```\n\n");

            // Subheading (h3)
            content.push_str(&format!("### Detail {section}.1\n\n"));
            content.push_str("More detailed content goes here. This is a paragraph that adds bulk.\n\n");
        }

        let file_path = dir.join(format!("{name}.md"));
        fs::write(&file_path, &content).unwrap();
    }
}

fn run_benchmark(label: &str, num_files: usize, file_kb: usize) {
    let dir = TempDir::new().unwrap();
    generate_vault(dir.path(), num_files, file_kb);

    // Measure total bytes on disk
    let total_bytes: u64 = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .map(|e| e.metadata().unwrap().len())
        .sum();

    let total_mb = total_bytes as f64 / (1024.0 * 1024.0);

    // Warmup
    let cfg = mdlint::config::Config::default();
    let _ = mdlint::workspace::Workspace::from_directory(dir.path(), &cfg);

    // Bench: full workspace build (parse + index)
    let iterations = 5;
    let mut parse_times = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let ws = mdlint::workspace::Workspace::from_directory(dir.path(), &cfg).unwrap();
        let _diagnostics = mdlint::rules::run_all(&ws, &cfg);
        parse_times.push(start.elapsed());
    }

    parse_times.sort();
    let median = parse_times[iterations / 2];
    let secs = median.as_secs_f64();
    let mb_per_sec = total_mb / secs;

    println!(
        "{label:>30}  {num_files:>6} files  {total_mb:>7.2} MB  {secs:>8.4}s  {mb_per_sec:>8.1} MB/s"
    );
}

fn main() {
    let num_files: usize = std::env::var("BENCH_FILES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let file_kb: usize = std::env::var("BENCH_FILE_KB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4);

    println!("{:>30}  {:>6}       {:>7}     {:>8}   {:>8}", "benchmark", "files", "size", "median", "throughput");
    println!("{}", "-".repeat(85));

    // Small vault: quick smoke test
    run_benchmark("small vault", 10, 2);

    // Medium vault: typical docs site
    run_benchmark("medium vault", num_files, file_kb);

    // Large vault: stress test
    run_benchmark("large vault", num_files * 10, file_kb);

    // Large files: fewer files but bigger
    run_benchmark("large files", num_files / 2, file_kb * 8);
}
