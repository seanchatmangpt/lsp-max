use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;
use std::path::PathBuf;
use test_analyzer::*;

#[derive(Parser)]
#[command(name = "test-analyzer")]
#[command(about = "Test pattern analyzer and falsification detector for Rust projects", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a directory for test patterns
    Scan {
        /// Path to scan
        path: PathBuf,

        /// Output format: json or pretty
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Filter by pattern
        #[arg(short, long)]
        pattern: Option<String>,
    },

    /// Generate a detailed report
    Report {
        /// Path to scan
        path: PathBuf,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Only show files with hollow tests
        #[arg(long)]
        hollow_only: bool,
    },

    /// Show only hollow tests
    Hollow {
        /// Path to scan
        path: PathBuf,

        /// Minimum falsification ratio to report (0.0 - 1.0)
        #[arg(short, long, default_value = "0.0")]
        min_ratio: f64,
    },

    /// Analyze test patterns
    Pattern {
        /// Path to scan
        path: PathBuf,

        /// Pattern type to filter
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// Compare two scans
    Compare {
        /// First directory to scan
        path1: PathBuf,

        /// Second directory to scan
        path2: PathBuf,

        /// Show only differences
        #[arg(short, long)]
        diff_only: bool,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            path,
            format,
            pattern,
        } => cmd_scan(&path, &format, pattern.as_deref())?,

        Commands::Report {
            path,
            output,
            hollow_only,
        } => cmd_report(&path, output.as_deref(), hollow_only)?,

        Commands::Hollow { path, min_ratio } => cmd_hollow(&path, min_ratio)?,

        Commands::Pattern { path, filter } => cmd_pattern(&path, filter.as_deref())?,

        Commands::Compare {
            path1,
            path2,
            diff_only,
        } => cmd_compare(&path1, &path2, diff_only)?,
    }

    Ok(())
}

fn cmd_scan(path: &std::path::Path, format: &str, pattern: Option<&str>) -> Result<()> {
    let test_files = TestAnalyzer::analyze_directory(path)?;
    let report = TestAnalyzer::generate_report(test_files);

    let output = if let Some(p) = pattern {
        filter_report_by_pattern(&report, p)
    } else {
        serde_json::to_value(&report)?
    };

    match format {
        "pretty" => println!("{}", serde_json::to_string_pretty(&output)?),
        "json" => println!("{}", serde_json::to_string(&output)?),
        _ => println!("{}", serde_json::to_string_pretty(&output)?),
    }

    Ok(())
}

fn cmd_report(path: &std::path::Path, output: Option<&std::path::Path>, hollow_only: bool) -> Result<()> {
    let test_files = TestAnalyzer::analyze_directory(path)?;
    let mut report = TestAnalyzer::generate_report(test_files);

    if hollow_only {
        report.by_file.retain(|f| f.hollow_count > 0);
    }

    let output_str = serde_json::to_string_pretty(&report)?;

    if let Some(out_path) = output {
        std::fs::write(out_path, output_str)?;
        println!("Report written to: {}", out_path.display());
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

fn cmd_hollow(path: &std::path::Path, min_ratio: f64) -> Result<()> {
    let test_files = TestAnalyzer::analyze_directory(path)?;
    let report = TestAnalyzer::generate_report(test_files);

    let filtered: Vec<_> = report
        .hollow_tests
        .iter()
        .filter(|t| {
            let file_ratio = report
                .by_file
                .iter()
                .find(|f| f.path == t.file)
                .map(|f| f.falsification_ratio)
                .unwrap_or(0.0);
            file_ratio >= min_ratio
        })
        .collect();

    println!("Hollow Tests ({})", filtered.len());
    println!("=====================================");

    for test in filtered {
        println!("\n{}: {}", test.file, test.test_name);
        println!("  Assertions: {}", test.assertion_count);
        println!(
            "  Types: {}",
            test.assertion_types
                .iter()
                .map(|t| format!("{:?}", t))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    Ok(())
}

fn cmd_pattern(path: &std::path::Path, filter: Option<&str>) -> Result<()> {
    let test_files = TestAnalyzer::analyze_directory(path)?;
    let report = TestAnalyzer::generate_report(test_files);

    println!("Test Patterns");
    println!("=====================================");
    println!("Service-Based:                {}", report.patterns.service_based);
    println!("LSP Backend Harness:          {}", report.patterns.lsp_harness);
    println!("Diagnostic Dogfood:           {}", report.patterns.dogfood);
    println!("Deterministic Property Sweep: {}", report.patterns.property_sweep);
    println!("Merger/Dedup:                 {}", report.patterns.merger);
    println!("Other:                        {}", report.patterns.other);

    if let Some(f) = filter {
        let filtered = report
            .by_file
            .iter()
            .flat_map(|file| {
                file.tests.iter().filter(move |t| {
                    t.patterns.iter().any(|p| format!("{:?}", p).contains(f))
                })
            })
            .collect::<Vec<_>>();

        println!("\nFiltered by '{}': {} tests", f, filtered.len());
        for test in filtered {
            println!("  - {}", test.name);
        }
    }

    Ok(())
}

fn cmd_compare(path1: &std::path::Path, path2: &std::path::Path, diff_only: bool) -> Result<()> {
    let files1 = TestAnalyzer::analyze_directory(path1)?;
    let report1 = TestAnalyzer::generate_report(files1);

    let files2 = TestAnalyzer::analyze_directory(path2)?;
    let report2 = TestAnalyzer::generate_report(files2);

    println!("Comparison: {} vs {}", path1.display(), path2.display());
    println!("=====================================");
    println!("\nSummary:");
    println!("  Path 1 - Total: {}, Hollow: {} ({:.1}%)",
        report1.summary.total_tests,
        report1.summary.hollow_tests,
        report1.summary.hollow_ratio * 100.0
    );
    println!("  Path 2 - Total: {}, Hollow: {} ({:.1}%)",
        report2.summary.total_tests,
        report2.summary.hollow_tests,
        report2.summary.hollow_ratio * 100.0
    );

    let test_delta = (report2.summary.total_tests as i64) - (report1.summary.total_tests as i64);
    let hollow_delta = (report2.summary.hollow_tests as i64) - (report1.summary.hollow_tests as i64);

    println!("\nDeltas:");
    println!("  Total tests: {}", if test_delta >= 0 { format!("+{}", test_delta) } else { test_delta.to_string() });
    println!("  Hollow tests: {}", if hollow_delta >= 0 { format!("+{}", hollow_delta) } else { hollow_delta.to_string() });

    println!("\nPattern Comparison:");
    println!("  Service-Based:     {} → {}", report1.patterns.service_based, report2.patterns.service_based);
    println!("  LSP Harness:       {} → {}", report1.patterns.lsp_harness, report2.patterns.lsp_harness);
    println!("  Dogfood:           {} → {}", report1.patterns.dogfood, report2.patterns.dogfood);
    println!("  Property Sweep:    {} → {}", report1.patterns.property_sweep, report2.patterns.property_sweep);
    println!("  Merger:            {} → {}", report1.patterns.merger, report2.patterns.merger);

    if diff_only {
        println!("\n\nFile-level differences:");
        let mut all_files = std::collections::HashMap::new();

        for file in &report1.by_file {
            all_files.insert(file.path.clone(), (Some(file), None));
        }

        for file in &report2.by_file {
            all_files.insert(
                file.path.clone(),
                match all_files.get(&file.path) {
                    Some((Some(f1), None)) => (Some(f1), Some(file)),
                    _ => (None, Some(file)),
                },
            );
        }

        for (path, (f1, f2)) in all_files.iter() {
            match (f1, f2) {
                (Some(file1), Some(file2)) => {
                    if file1.test_count != file2.test_count
                        || file1.hollow_count != file2.hollow_count
                    {
                        println!(
                            "  {} : {} → {} tests (hollow: {} → {})",
                            path, file1.test_count, file2.test_count, file1.hollow_count, file2.hollow_count
                        );
                    }
                }
                (Some(file1), None) => {
                    println!("  {} : {} tests (removed)", path, file1.test_count);
                }
                (None, Some(file2)) => {
                    println!("  {} : {} tests (added)", path, file2.test_count);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn filter_report_by_pattern(report: &AnalysisReport, pattern: &str) -> serde_json::Value {
    let filtered_files: Vec<_> = report
        .by_file
        .iter()
        .map(|file| {
            let filtered_tests: Vec<_> = file
                .tests
                .iter()
                .filter(|test| test.patterns.iter().any(|p| format!("{:?}", p).contains(pattern)))
                .cloned()
                .collect();

            let mut file_copy = file.clone();
            file_copy.tests = filtered_tests;
            file_copy
        })
        .filter(|f| !f.tests.is_empty())
        .collect();

    json!({
        "summary": report.summary,
        "by_file": filtered_files,
        "pattern_filter": pattern
    })
}
