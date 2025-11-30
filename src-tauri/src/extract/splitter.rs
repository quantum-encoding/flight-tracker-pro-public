// PDF Splitter Module
// Converts multi-page PDFs into individual PNG images for OCR processing

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;

/// Result of splitting a PDF into pages
#[derive(Debug, Clone)]
pub struct SplitResult {
    pub page_count: usize,
    pub output_dir: PathBuf,
    pub page_paths: Vec<PathBuf>,
}

/// Configuration for PDF splitting
#[derive(Debug, Clone)]
pub struct SplitConfig {
    /// DPI resolution for output images (default: 200)
    pub dpi: u32,
    /// Output format (png, jpeg, tiff)
    pub format: ImageFormat,
    /// Optional page range (start, end) - 1-indexed, inclusive
    pub page_range: Option<(u32, u32)>,
}

#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Tiff,
}

impl Default for SplitConfig {
    fn default() -> Self {
        Self {
            dpi: 200,
            format: ImageFormat::Png,
            page_range: None,
        }
    }
}

impl ImageFormat {
    fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Tiff => "tiff",
        }
    }

    fn pdftoppm_flag(&self) -> &'static str {
        match self {
            ImageFormat::Png => "-png",
            ImageFormat::Jpeg => "-jpeg",
            ImageFormat::Tiff => "-tiff",
        }
    }
}

/// Get the number of pages in a PDF using pdfinfo
pub async fn get_page_count(pdf_path: &Path) -> Result<usize> {
    let output = Command::new("pdfinfo")
        .arg(pdf_path)
        .output()
        .context("Failed to execute pdfinfo - is poppler-utils installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("pdfinfo failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.starts_with("Pages:") {
            let count_str = line.trim_start_matches("Pages:").trim();
            let count: usize = count_str
                .parse()
                .context("Failed to parse page count")?;
            return Ok(count);
        }
    }

    anyhow::bail!("Could not find page count in pdfinfo output")
}

/// Split a PDF into individual page images using pdftoppm
pub async fn split_pdf(pdf_path: &Path, output_dir: &Path, config: &SplitConfig) -> Result<SplitResult> {
    // Ensure output directory exists
    fs::create_dir_all(output_dir)
        .await
        .context("Failed to create output directory")?;

    // Get page count first
    let total_pages = get_page_count(pdf_path).await?;

    // Build pdftoppm command
    let mut cmd = Command::new("pdftoppm");
    cmd.arg(config.format.pdftoppm_flag())
        .arg("-r")
        .arg(config.dpi.to_string());

    // Add page range if specified
    if let Some((start, end)) = config.page_range {
        cmd.arg("-f").arg(start.to_string());
        cmd.arg("-l").arg(end.to_string());
    }

    // Input PDF and output prefix
    cmd.arg(pdf_path);
    cmd.arg(output_dir.join("page"));

    // Execute
    let output = cmd
        .output()
        .context("Failed to execute pdftoppm - is poppler-utils installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("pdftoppm failed: {}", stderr);
    }

    // Collect output file paths
    let extension = config.format.extension();
    let mut page_paths = Vec::new();

    let (start_page, end_page) = config.page_range.unwrap_or((1, total_pages as u32));
    let actual_pages = (end_page - start_page + 1) as usize;

    // pdftoppm names files as page-001.png, page-002.png, etc.
    for i in start_page..=end_page {
        // Handle different numbering formats based on total pages
        let filename = if total_pages >= 100 {
            format!("page-{:03}.{}", i, extension)
        } else if total_pages >= 10 {
            format!("page-{:02}.{}", i, extension)
        } else {
            format!("page-{}.{}", i, extension)
        };

        let path = output_dir.join(&filename);

        // Also try the 3-digit format which pdftoppm often uses
        let alt_filename = format!("page-{:03}.{}", i, extension);
        let alt_path = output_dir.join(&alt_filename);

        if path.exists() {
            page_paths.push(path);
        } else if alt_path.exists() {
            page_paths.push(alt_path);
        }
    }

    // If we didn't find files with expected names, scan directory
    if page_paths.is_empty() {
        let mut entries = fs::read_dir(output_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|e| e == extension).unwrap_or(false) {
                page_paths.push(path);
            }
        }
        page_paths.sort();
    }

    Ok(SplitResult {
        page_count: page_paths.len(),
        output_dir: output_dir.to_path_buf(),
        page_paths,
    })
}

/// Split PDF into batches for parallel processing
pub async fn split_pdf_batched(
    pdf_path: &Path,
    output_dir: &Path,
    config: &SplitConfig,
    batch_size: usize,
) -> Result<Vec<SplitResult>> {
    let total_pages = get_page_count(pdf_path).await?;
    let mut results = Vec::new();

    let (start, end) = config.page_range.unwrap_or((1, total_pages as u32));
    let mut current = start;

    while current <= end {
        let batch_end = (current + batch_size as u32 - 1).min(end);
        let batch_dir = output_dir.join(format!("batch_{:03}_{:03}", current, batch_end));

        let batch_config = SplitConfig {
            page_range: Some((current, batch_end)),
            ..config.clone()
        };

        let result = split_pdf(pdf_path, &batch_dir, &batch_config).await?;
        results.push(result);

        current = batch_end + 1;
    }

    Ok(results)
}

/// Clean up temporary page images
pub async fn cleanup_pages(output_dir: &Path) -> Result<()> {
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)
            .await
            .context("Failed to remove temporary pages directory")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_page_count() {
        // This test requires a real PDF file - use TEST_PDF_PATH env var or skip
        let pdf_path = std::env::var("TEST_PDF_PATH")
            .map(|p| std::path::PathBuf::from(p))
            .unwrap_or_else(|_| {
                // Try a relative path in the project directory
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data").join("sample.pdf")
            });

        if pdf_path.exists() {
            let count = get_page_count(&pdf_path).await.unwrap();
            assert!(count > 0);
            println!("PDF has {} pages", count);
        } else {
            println!("Skipping test: no PDF file at {:?}", pdf_path);
        }
    }
}
