// Identity Fusion CLI Tool
// Analyzes passenger names from flight logs and produces entity resolution mappings
//
// Usage: identity-fusion --csv <path> [--output <dir>] [--api-key <key>] [--auto-merge]

use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

use flight_tracker_pro_lib::extract::identity_fusion::{
    IdentityFusion, FusionConfig, MergeCandidate, MatchType,
};

#[derive(Parser, Debug)]
#[command(name = "identity-fusion")]
#[command(about = "Resolve passenger identities from flight log data")]
#[command(version)]
struct Args {
    /// Path to the CSV file with flight data
    #[arg(short, long)]
    csv: PathBuf,

    /// Output directory for results (default: ./identity_output)
    #[arg(short, long, default_value = "./identity_output")]
    output: PathBuf,

    /// Google Gemini API key for AI analysis (or set GEMINI_API_KEY env var)
    #[arg(short = 'k', long)]
    api_key: Option<String>,

    /// Automatically apply high-confidence merges
    #[arg(long)]
    auto_merge: bool,

    /// Fuzzy matching threshold (0.0-1.0, default: 0.85)
    #[arg(long, default_value = "0.85")]
    fuzzy_threshold: f64,

    /// Use AI to analyze ambiguous names
    #[arg(long)]
    use_ai: bool,

    /// Export format: json, sql, or both (default: both)
    #[arg(long, default_value = "both")]
    format: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Validate CSV exists
    if !args.csv.exists() {
        anyhow::bail!("CSV file not found: {}", args.csv.display());
    }

    // Create output directory
    fs::create_dir_all(&args.output).await?;

    println!("🔍 Identity Fusion - Passenger Entity Resolution");
    println!("================================================");
    println!("CSV: {}", args.csv.display());
    println!("Output: {}", args.output.display());
    println!("Fuzzy threshold: {:.2}", args.fuzzy_threshold);
    println!();

    // Step 1: Extract passenger names from CSV
    println!("📊 Extracting passenger names from CSV...");
    let name_counts = extract_passenger_names(&args.csv).await?;
    println!("   Found {} unique names", name_counts.len());

    // Show top names
    let mut sorted_names: Vec<_> = name_counts.iter().collect();
    sorted_names.sort_by(|a, b| b.1.cmp(a.1));

    println!();
    println!("📋 Top 20 Passengers by Flight Count:");
    for (i, (name, count)) in sorted_names.iter().take(20).enumerate() {
        println!("   {:2}. {} ({} flights)", i + 1, name, count);
    }

    // Step 2: Run identity fusion algorithm
    println!();
    println!("🔗 Running identity fusion analysis...");

    let config = FusionConfig {
        fuzzy_threshold: args.fuzzy_threshold,
        auto_merge_threshold: 0.95,
        ..Default::default()
    };

    let mut fusion = IdentityFusion::new(config);

    let name_vec: Vec<_> = name_counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
    let candidates = fusion.analyze_names(&name_vec);

    // Categorize candidates
    let auto_merge: Vec<_> = candidates.iter().filter(|c| c.auto_merge).collect();
    let manual_review: Vec<_> = candidates.iter().filter(|c| !c.auto_merge).collect();

    println!();
    println!("📈 Analysis Results:");
    println!("   Entities created: {}", fusion.get_entities().len());
    println!("   Auto-merge candidates: {}", auto_merge.len());
    println!("   Manual review needed: {}", manual_review.len());

    // Show auto-merge candidates
    if !auto_merge.is_empty() {
        println!();
        println!("✅ Auto-Merge Candidates (high confidence):");
        for candidate in &auto_merge {
            let match_type = match candidate.match_type {
                MatchType::ExactMatch => "exact",
                MatchType::Abbreviation => "abbrev",
                MatchType::Substring => "partial",
                MatchType::FuzzyMatch => "fuzzy",
                MatchType::AIInferred => "ai",
            };
            println!(
                "   {} → {} ({}: {:.2})",
                candidate.source_name, candidate.target_canonical_name, match_type, candidate.similarity_score
            );
        }
    }

    // Apply auto-merges if requested
    if args.auto_merge {
        println!();
        println!("🔄 Applying {} auto-merges...", auto_merge.len());
        for candidate in auto_merge {
            fusion.apply_merge(candidate);
        }
    }

    // Show manual review candidates
    if !manual_review.is_empty() {
        println!();
        println!("⚠️  Manual Review Required ({} candidates):", manual_review.len());
        for (i, candidate) in manual_review.iter().enumerate().take(30) {
            let match_type = match candidate.match_type {
                MatchType::ExactMatch => "exact",
                MatchType::Abbreviation => "abbrev",
                MatchType::Substring => "partial",
                MatchType::FuzzyMatch => "fuzzy",
                MatchType::AIInferred => "ai",
            };
            println!(
                "   {:2}. {} → {} ({}: {:.2})",
                i + 1, candidate.source_name, candidate.target_canonical_name, match_type, candidate.similarity_score
            );
        }
        if manual_review.len() > 30 {
            println!("   ... and {} more", manual_review.len() - 30);
        }
    }

    // Find unmapped names
    let mapped_names: std::collections::HashSet<_> = candidates.iter().map(|c| &c.source_name).collect();
    let entity_names: std::collections::HashSet<_> = fusion.get_entities().iter().map(|e| &e.canonical_name).collect();

    let unmapped: Vec<_> = sorted_names
        .iter()
        .filter(|(name, _)| !mapped_names.contains(*name) && !entity_names.contains(*name))
        .take(50)
        .collect();

    if !unmapped.is_empty() {
        println!();
        println!("❓ Unmapped Names (no match found):");
        for (name, count) in &unmapped {
            println!("   {} ({} flights)", name, count);
        }
    }

    // Step 3: Export results
    println!();
    println!("💾 Exporting results...");

    // Export aliases.json
    let aliases = fusion.export_aliases();
    let aliases_path = args.output.join("aliases.json");
    let aliases_json = serde_json::to_string_pretty(&aliases)?;
    fs::write(&aliases_path, &aliases_json).await?;
    println!("   Aliases JSON: {}", aliases_path.display());

    // Export entities.json
    let entities: Vec<_> = fusion.get_entities().iter().map(|e| (*e).clone()).collect();
    let entities_path = args.output.join("entities.json");
    let entities_json = serde_json::to_string_pretty(&entities)?;
    fs::write(&entities_path, &entities_json).await?;
    println!("   Entities JSON: {}", entities_path.display());

    // Export merge_candidates.json
    let candidates_path = args.output.join("merge_candidates.json");
    let candidates_json = serde_json::to_string_pretty(&candidates)?;
    fs::write(&candidates_path, &candidates_json).await?;
    println!("   Merge Candidates: {}", candidates_path.display());

    // Export SQL if requested
    if args.format == "sql" || args.format == "both" {
        let sql_path = args.output.join("apply_aliases.sql");
        let sql = generate_sql(&aliases);
        fs::write(&sql_path, &sql).await?;
        println!("   SQL Script: {}", sql_path.display());
    }

    // Export summary report
    let report_path = args.output.join("fusion_report.md");
    let report = generate_report(&fusion, &candidates, &unmapped, &name_counts);
    fs::write(&report_path, &report).await?;
    println!("   Summary Report: {}", report_path.display());

    println!();
    println!("✅ Identity fusion complete!");
    println!();
    println!("Next steps:");
    println!("  1. Review aliases.json and approve/edit mappings");
    println!("  2. Review merge_candidates.json for manual review items");
    if args.format == "sql" || args.format == "both" {
        println!("  3. Apply SQL script to your database: apply_aliases.sql");
    }

    Ok(())
}

/// Extract passenger names and their frequencies from CSV
async fn extract_passenger_names(csv_path: &PathBuf) -> Result<HashMap<String, usize>> {
    let content = fs::read_to_string(csv_path).await?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(content.as_bytes());

    let headers = reader.headers()?.clone();

    // Find passengers column
    let passengers_idx = headers
        .iter()
        .position(|h| h.to_lowercase().contains("passenger"))
        .unwrap_or(4); // Default to column 4

    let mut name_counts: HashMap<String, usize> = HashMap::new();

    for result in reader.records() {
        if let Ok(record) = result {
            if let Some(passengers) = record.get(passengers_idx) {
                // Split by common delimiters
                for name in passengers.split(|c| c == ';' || c == ',' || c == '|') {
                    let cleaned = name.trim().to_uppercase();

                    // Skip empty or very short names
                    if cleaned.len() >= 2 {
                        // Skip common non-name entries
                        if !should_skip_name(&cleaned) {
                            *name_counts.entry(cleaned).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(name_counts)
}

/// Check if a "name" should be skipped (not actually a person)
fn should_skip_name(name: &str) -> bool {
    let skip_patterns = [
        "ABOVE", "BELOW", "CREW", "PAX", "PASSENGER", "RELOCATE",
        "FERRY", "REPO", "MAINTENANCE", "TEST", "TRAINING", "DEMO",
        "SHOW PLANE", "INQUIRIES", "FEMALE", "MALE", "ADULT", "CHILD",
    ];

    for pattern in &skip_patterns {
        if name.contains(pattern) {
            return true;
        }
    }

    // Skip if mostly numbers
    let alpha_count = name.chars().filter(|c| c.is_alphabetic()).count();
    let total_count = name.chars().filter(|c| c.is_alphanumeric()).count();

    if total_count > 0 && (alpha_count as f64 / total_count as f64) < 0.5 {
        return true;
    }

    false
}

/// Generate SQL script to apply aliases
fn generate_sql(aliases: &HashMap<String, String>) -> String {
    let mut sql = String::new();

    sql.push_str("-- Identity Fusion: Apply Passenger Aliases\n");
    sql.push_str("-- Generated by identity-fusion tool\n");
    sql.push_str("-- Review before executing!\n\n");

    sql.push_str("BEGIN TRANSACTION;\n\n");

    sql.push_str("-- Insert alias mappings into passenger_mappings table\n");
    for (alias, canonical) in aliases {
        let escaped_alias = alias.replace("'", "''");
        let escaped_canonical = canonical.replace("'", "''");

        sql.push_str(&format!(
            "INSERT OR REPLACE INTO passenger_mappings (abbreviation, full_name, updated_at) \n\
             VALUES ('{}', '{}', datetime('now'));\n",
            escaped_alias, escaped_canonical
        ));
    }

    sql.push_str("\nCOMMIT;\n");

    sql
}

/// Generate markdown report
fn generate_report(
    fusion: &IdentityFusion,
    candidates: &[MergeCandidate],
    unmapped: &[&(&String, &usize)],
    name_counts: &HashMap<String, usize>,
) -> String {
    let mut report = String::new();

    report.push_str("# Identity Fusion Report\n\n");
    report.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

    // Summary
    report.push_str("## Summary\n\n");
    report.push_str(&format!("- **Total unique names:** {}\n", name_counts.len()));
    report.push_str(&format!("- **Entities created:** {}\n", fusion.get_entities().len()));
    report.push_str(&format!("- **Merge candidates:** {}\n", candidates.len()));
    report.push_str(&format!("- **Unmapped names:** {}\n\n", unmapped.len()));

    // Entities
    report.push_str("## Resolved Entities\n\n");
    let entities_ref = fusion.get_entities();
    let mut entities: Vec<_> = entities_ref.iter().collect();
    entities.sort_by(|a, b| b.flight_count.cmp(&a.flight_count));

    for entity in entities.iter().take(50) {
        report.push_str(&format!("### {} ({} flights)\n", entity.canonical_name, entity.flight_count));
        if entity.aliases.len() > 1 {
            report.push_str("**Aliases:** ");
            report.push_str(&entity.aliases.join(", "));
            report.push_str("\n");
        }
        report.push_str("\n");
    }

    // Merge candidates needing review
    let manual_review: Vec<_> = candidates.iter().filter(|c| !c.auto_merge).collect();
    if !manual_review.is_empty() {
        report.push_str("## Manual Review Required\n\n");
        report.push_str("| Source | Target | Match Type | Similarity |\n");
        report.push_str("|--------|--------|------------|------------|\n");

        for candidate in manual_review.iter().take(50) {
            let match_type = match candidate.match_type {
                MatchType::ExactMatch => "Exact",
                MatchType::Abbreviation => "Abbreviation",
                MatchType::Substring => "Substring",
                MatchType::FuzzyMatch => "Fuzzy",
                MatchType::AIInferred => "AI",
            };
            report.push_str(&format!(
                "| {} | {} | {} | {:.2} |\n",
                candidate.source_name, candidate.target_canonical_name, match_type, candidate.similarity_score
            ));
        }
        report.push_str("\n");
    }

    // Unmapped names
    if !unmapped.is_empty() {
        report.push_str("## Unmapped Names\n\n");
        report.push_str("These names could not be matched to any entity:\n\n");
        for (name, count) in unmapped.iter().take(100) {
            report.push_str(&format!("- {} ({} flights)\n", name, count));
        }
        report.push_str("\n");
    }

    report
}
