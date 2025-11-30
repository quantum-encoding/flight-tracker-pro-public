// Flight Log Ingestion CLI Tool
// Orchestrates the PDF split -> Vision extraction -> Aggregation pipeline
//
// Usage: ingest_logs --pdf <path> [--concurrency 50] [--output <dir>] [--api-key <key>]

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::fs;

// Import from the library
use flight_tracker_pro_lib::extract::{
    splitter::{split_pdf, get_page_count, SplitConfig, ImageFormat},
    vision_agent::{VisionAgent, VisionAgentConfig, PageExtractionResult},
    aggregator::{aggregate_results, save_master_log, save_csv_export},
};

#[derive(Parser, Debug)]
#[command(name = "ingest_logs")]
#[command(about = "Extract flight data from scanned PDF flight logs using AI vision")]
#[command(version)]
struct Args {
    /// Path to the PDF file to process
    #[arg(short, long)]
    pdf: PathBuf,

    /// Number of concurrent API requests (default: 10)
    #[arg(short, long, default_value = "10")]
    concurrency: usize,

    /// Output directory for results (default: ./output)
    #[arg(short, long, default_value = "./output")]
    output: PathBuf,

    /// Google Gemini API key (or set GEMINI_API_KEY env var)
    #[arg(short = 'k', long)]
    api_key: Option<String>,

    /// DPI resolution for page images (default: 200)
    #[arg(long, default_value = "200")]
    dpi: u32,

    /// Start page (1-indexed, default: 1)
    #[arg(long)]
    start_page: Option<u32>,

    /// End page (1-indexed, default: last page)
    #[arg(long)]
    end_page: Option<u32>,

    /// Keep temporary page images after processing
    #[arg(long)]
    keep_images: bool,

    /// Save individual page JSON results
    #[arg(long)]
    save_page_results: bool,

    /// Skip PDF splitting (use existing images in temp dir)
    #[arg(long)]
    skip_split: bool,

    /// Dry run - only split PDF, don't call API
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Get API key
    let api_key = args.api_key
        .or_else(|| std::env::var("GEMINI_API_KEY").ok())
        .context("No API key provided. Use --api-key or set GEMINI_API_KEY environment variable")?;

    // Validate PDF exists
    if !args.pdf.exists() {
        anyhow::bail!("PDF file not found: {}", args.pdf.display());
    }

    // Create output directories
    let temp_dir = args.output.join("temp");
    let pages_dir = temp_dir.join("pages");
    let results_dir = args.output.join("results");

    fs::create_dir_all(&pages_dir).await?;
    fs::create_dir_all(&results_dir).await?;

    println!("🔍 Flight Log Ingestion Pipeline");
    println!("================================");
    println!("PDF: {}", args.pdf.display());
    println!("Output: {}", args.output.display());
    println!("Concurrency: {}", args.concurrency);
    println!();

    // Step 1: Get page count and determine range
    let total_pages = get_page_count(&args.pdf).await?;
    let start_page = args.start_page.unwrap_or(1);
    let end_page = args.end_page.unwrap_or(total_pages as u32);

    println!("📄 PDF has {} pages total", total_pages);
    println!("📑 Processing pages {} to {}", start_page, end_page);
    println!();

    // Step 2: Split PDF into page images
    let page_paths = if args.skip_split {
        println!("⏭️  Skipping PDF split, using existing images...");
        collect_existing_images(&pages_dir).await?
    } else {
        println!("✂️  Splitting PDF into page images...");
        let config = SplitConfig {
            dpi: args.dpi,
            format: ImageFormat::Png,
            page_range: Some((start_page, end_page)),
        };

        let split_result = split_pdf(&args.pdf, &pages_dir, &config).await?;
        println!("   Created {} page images", split_result.page_count);
        split_result.page_paths
    };

    if args.dry_run {
        println!();
        println!("🏃 Dry run complete. Page images saved to: {}", pages_dir.display());
        println!("   Run without --dry-run to process with Gemini API");
        return Ok(());
    }

    // Step 3: Process pages with Gemini Vision
    println!();
    println!("🤖 Processing pages with Gemini Vision API...");
    println!("   Model: gemini-2.5-flash-lite (cheapest)");
    println!();

    let agent_config = VisionAgentConfig {
        api_key,
        model: "gemini-2.5-flash-lite".to_string(),
        max_tokens: 8192,
        temperature: 0.1,
    };

    let results = process_pages_with_progress(
        &agent_config,
        page_paths,
        start_page,
        args.concurrency,
    ).await;

    // Save individual page results if requested
    if args.save_page_results {
        println!();
        println!("💾 Saving individual page results...");
        for result in &results {
            let path = results_dir.join(format!("page_{:03}.json", result.page_number));
            let json = serde_json::to_string_pretty(result)?;
            fs::write(&path, json).await?;
        }
    }

    // Step 4: Aggregate results
    println!();
    println!("📊 Aggregating results...");

    let master_log = aggregate_results(results);

    println!("   Total entries extracted: {}", master_log.total_entries);
    println!("   Pages with errors: {}", master_log.pages_with_errors);
    println!("   Unique aircraft: {}", master_log.unique_aircraft.len());
    println!("   Unique airports: {}", master_log.unique_airports.len());

    if let Some((start, end)) = &master_log.date_range {
        println!("   Date range: {} to {}", start, end);
    }

    // Step 5: Save outputs
    println!();
    println!("💾 Saving outputs...");

    let master_json_path = args.output.join("master_log.json");
    save_master_log(&master_log, &master_json_path).await?;
    println!("   Master JSON: {}", master_json_path.display());

    let csv_path = args.output.join("flight_log.csv");
    save_csv_export(&master_log, &csv_path).await?;
    println!("   CSV export: {}", csv_path.display());

    // Cleanup temp files if not keeping
    if !args.keep_images {
        println!();
        println!("🧹 Cleaning up temporary files...");
        if let Err(e) = fs::remove_dir_all(&temp_dir).await {
            eprintln!("   Warning: Could not remove temp dir: {}", e);
        }
    }

    // Print errors if any
    if !master_log.processing_errors.is_empty() {
        println!();
        println!("⚠️  Processing Errors:");
        for err in &master_log.processing_errors {
            println!("   Page {}: {}", err.page_number, err.error);
        }
    }

    println!();
    println!("✅ Done! Extracted {} flight entries from {} pages.",
             master_log.total_entries, master_log.pages_processed);
    println!();
    println!("Next steps:");
    println!("  1. Review the CSV at: {}", csv_path.display());
    println!("  2. Import into Flight Tracker Pro using the 'Import Flights' feature");

    Ok(())
}

/// Collect existing PNG images from a directory
async fn collect_existing_images(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let mut entries = fs::read_dir(dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().map(|e| e == "png").unwrap_or(false) {
            paths.push(path);
        }
    }

    paths.sort();
    Ok(paths)
}

/// Process pages with progress reporting
async fn process_pages_with_progress(
    config: &VisionAgentConfig,
    page_paths: Vec<PathBuf>,
    start_page: u32,
    concurrency: usize,
) -> Vec<PageExtractionResult> {
    let total = page_paths.len();
    let processed = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let mut handles = Vec::new();

    for (i, path) in page_paths.into_iter().enumerate() {
        let page_num = start_page + i as u32;
        let config = config.clone();
        let processed = processed.clone();
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        let handle = tokio::spawn(async move {
            let agent = VisionAgent::new(config);
            let result = agent.extract_from_image(&path, page_num).await;
            drop(permit);

            let count = processed.fetch_add(1, Ordering::SeqCst) + 1;
            let entries_count = result.as_ref().map(|r| r.entries.len()).unwrap_or(0);

            // Progress output
            print!("\r   Processing: {}/{} pages ({} entries from page {})",
                   count, total, entries_count, page_num);
            use std::io::Write;
            std::io::stdout().flush().ok();

            match result {
                Ok(r) => r,
                Err(e) => PageExtractionResult {
                    page_number: page_num,
                    image_path: path.to_string_lossy().to_string(),
                    entries: vec![],
                    raw_response: None,
                    error: Some(e.to_string()),
                },
            }
        });

        handles.push(handle);
    }

    // Collect all results
    let mut results = Vec::new();
    for handle in handles {
        if let Ok(result) = handle.await {
            results.push(result);
        }
    }

    println!(); // New line after progress
    results.sort_by_key(|r| r.page_number);
    results
}
