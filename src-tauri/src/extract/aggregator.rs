// Aggregator Module
// Merges extracted flight log data and cleans up OCR errors

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tokio::fs;

use super::vision_agent::{FlightLogEntry, PageExtractionResult};

/// Aggregated master log containing all flights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterFlightLog {
    pub total_entries: usize,
    pub pages_processed: usize,
    pub pages_with_errors: usize,
    pub unique_aircraft: Vec<String>,
    pub unique_airports: Vec<String>,
    pub date_range: Option<(String, String)>,
    pub entries: Vec<FlightLogEntry>,
    pub processing_errors: Vec<ProcessingError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingError {
    pub page_number: u32,
    pub error: String,
}

/// OCR correction rules for common misreadings
pub struct OcrCorrector {
    /// Known tail numbers for fuzzy matching
    known_tail_numbers: HashSet<String>,
    /// Known airport codes
    known_airports: HashSet<String>,
    /// Character substitution map for common OCR errors
    char_substitutions: HashMap<char, Vec<char>>,
}

impl Default for OcrCorrector {
    fn default() -> Self {
        let mut char_substitutions = HashMap::new();
        // Common OCR confusions
        char_substitutions.insert('0', vec!['O', 'Q', 'D']);
        char_substitutions.insert('O', vec!['0', 'Q', 'D']);
        char_substitutions.insert('1', vec!['I', 'l', '7']);
        char_substitutions.insert('I', vec!['1', 'l', '7']);
        char_substitutions.insert('l', vec!['1', 'I', '7']);
        char_substitutions.insert('5', vec!['S', '6']);
        char_substitutions.insert('S', vec!['5', '8']);
        char_substitutions.insert('8', vec!['B', '6', '0']);
        char_substitutions.insert('B', vec!['8', '6', '0']);
        char_substitutions.insert('6', vec!['G', 'b']);
        char_substitutions.insert('G', vec!['6', 'C']);
        char_substitutions.insert('2', vec!['Z']);
        char_substitutions.insert('Z', vec!['2', '7']);

        Self {
            known_tail_numbers: HashSet::new(),
            known_airports: HashSet::new(),
            char_substitutions,
        }
    }
}

impl OcrCorrector {
    /// Add known tail numbers for validation
    pub fn add_known_tail_numbers(&mut self, tail_numbers: impl IntoIterator<Item = String>) {
        self.known_tail_numbers.extend(tail_numbers);
    }

    /// Add known airport codes
    pub fn add_known_airports(&mut self, airports: impl IntoIterator<Item = String>) {
        self.known_airports.extend(airports);
    }

    /// Load common US airport codes
    pub fn load_common_airports(&mut self) {
        let common_airports = [
            // Major hubs
            "ATL", "LAX", "ORD", "DFW", "DEN", "JFK", "SFO", "SEA", "LAS", "MCO",
            "EWR", "CLT", "PHX", "IAH", "MIA", "BOS", "MSP", "FLL", "DTW", "PHL",
            // From the flight log image
            "PSP", "CMH", "BKL", "CLE", "MDW", "ATW", "ISP", "ILG", "TEB", "CPS",
            "RDU", "BRL", "LGR", "RNB",
            // General aviation
            "VNY", "SMO", "HPN", "OPF", "FXE", "SDL", "APA", "FFZ",
        ];
        self.known_airports.extend(common_airports.iter().map(|s| s.to_string()));
    }

    /// Clean and normalize a tail number
    pub fn clean_tail_number(&self, tail: &str) -> String {
        let mut cleaned = tail.trim().to_uppercase();

        // Remove common prefixes/suffixes that shouldn't be there
        cleaned = cleaned.trim_start_matches("N-").to_string();
        cleaned = cleaned.replace(" ", "");

        // Ensure US registration starts with N
        if !cleaned.starts_with('N') && cleaned.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            cleaned = format!("N{}", cleaned);
        }

        // Try to correct common OCR errors in tail numbers
        // Tail numbers are typically N + 1-5 digits + 0-2 letters
        if cleaned.starts_with('N') && cleaned.len() >= 2 {
            let suffix = &cleaned[1..];
            let mut corrected = String::from("N");

            for (i, c) in suffix.chars().enumerate() {
                // First few characters after N are usually digits
                if i < 4 {
                    corrected.push(self.likely_digit(c));
                } else {
                    // Last 1-2 characters can be letters
                    corrected.push(self.likely_letter(c));
                }
            }
            cleaned = corrected;
        }

        cleaned
    }

    /// Clean and normalize an airport code
    pub fn clean_airport_code(&self, code: &str) -> String {
        let cleaned = code.trim().to_uppercase().replace(" ", "");

        // Airport codes should be 3-4 letters
        if cleaned.len() < 3 || cleaned.len() > 4 {
            return cleaned;
        }

        // Check if it's a known airport
        if self.known_airports.contains(&cleaned) {
            return cleaned;
        }

        // Try OCR corrections
        let corrected = self.try_airport_correction(&cleaned);
        if self.known_airports.contains(&corrected) {
            return corrected;
        }

        cleaned
    }

    /// Try to correct airport code OCR errors
    fn try_airport_correction(&self, code: &str) -> String {
        let mut result = String::new();
        for c in code.chars() {
            // Airport codes should be all letters
            result.push(self.likely_letter(c));
        }
        result
    }

    /// Convert a character to most likely digit
    fn likely_digit(&self, c: char) -> char {
        match c {
            'O' | 'o' | 'Q' | 'D' => '0',
            'I' | 'i' | 'l' | 'L' | '|' => '1',
            'Z' | 'z' => '2',
            'E' => '3',
            'A' | 'h' => '4',
            'S' | 's' => '5',
            'G' | 'b' => '6',
            'T' => '7',
            'B' => '8',
            'g' | 'q' => '9',
            _ => c,
        }
    }

    /// Convert a character to most likely letter
    fn likely_letter(&self, c: char) -> char {
        match c {
            '0' => 'O',
            '1' | '|' => 'I',
            '2' => 'Z',
            '3' => 'E',
            '4' => 'A',
            '5' => 'S',
            '6' => 'G',
            '8' => 'B',
            _ => c.to_ascii_uppercase(),
        }
    }

    /// Clean a flight log entry
    pub fn clean_entry(&self, entry: &mut FlightLogEntry) {
        // Clean tail number
        if let Some(ref tail) = entry.aircraft_registration {
            entry.aircraft_registration = Some(self.clean_tail_number(tail));
        }

        // Clean airport codes
        if let Some(ref from) = entry.from {
            entry.from = Some(self.clean_airport_code(from));
        }
        if let Some(ref to) = entry.to {
            entry.to = Some(self.clean_airport_code(to));
        }

        // Clean passenger names - trim whitespace
        if let Some(ref passengers) = entry.passengers {
            let cleaned: String = passengers
                .split(';')
                .map(|p| p.trim())
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
                .join("; ");
            entry.passengers = if cleaned.is_empty() { None } else { Some(cleaned) };
        }
    }
}

/// Aggregate multiple page results into a master log
pub fn aggregate_results(results: Vec<PageExtractionResult>) -> MasterFlightLog {
    let mut corrector = OcrCorrector::default();
    corrector.load_common_airports();

    let mut all_entries = Vec::new();
    let mut processing_errors = Vec::new();
    let mut unique_aircraft = HashSet::new();
    let mut unique_airports = HashSet::new();
    let mut pages_with_errors = 0;

    for result in &results {
        if let Some(ref error) = result.error {
            pages_with_errors += 1;
            processing_errors.push(ProcessingError {
                page_number: result.page_number,
                error: error.clone(),
            });
        }

        for mut entry in result.entries.clone() {
            // Clean the entry
            corrector.clean_entry(&mut entry);

            // Collect unique values
            if let Some(ref tail) = entry.aircraft_registration {
                if !tail.is_empty() {
                    unique_aircraft.insert(tail.clone());
                }
            }
            if let Some(ref from) = entry.from {
                if !from.is_empty() {
                    unique_airports.insert(from.clone());
                }
            }
            if let Some(ref to) = entry.to {
                if !to.is_empty() {
                    unique_airports.insert(to.clone());
                }
            }

            all_entries.push(entry);
        }
    }

    // Sort entries by page number
    all_entries.sort_by_key(|e| e.source_page);

    // Determine date range
    let date_range = determine_date_range(&all_entries);

    // Sort unique values
    let mut unique_aircraft: Vec<_> = unique_aircraft.into_iter().collect();
    unique_aircraft.sort();
    let mut unique_airports: Vec<_> = unique_airports.into_iter().collect();
    unique_airports.sort();

    MasterFlightLog {
        total_entries: all_entries.len(),
        pages_processed: results.len(),
        pages_with_errors,
        unique_aircraft,
        unique_airports,
        date_range,
        entries: all_entries,
        processing_errors,
    }
}

/// Determine the date range from entries
fn determine_date_range(entries: &[FlightLogEntry]) -> Option<(String, String)> {
    let dates: Vec<&String> = entries
        .iter()
        .filter_map(|e| e.date.as_ref())
        .filter(|d| !d.is_empty())
        .collect();

    if dates.is_empty() {
        return None;
    }

    // Simple approach: take first and last (assuming chronological order)
    Some((dates.first()?.to_string(), dates.last()?.to_string()))
}

/// Save master log to JSON file
pub async fn save_master_log(log: &MasterFlightLog, output_path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(log)
        .context("Failed to serialize master log")?;

    fs::write(output_path, json)
        .await
        .context("Failed to write master log file")?;

    Ok(())
}

/// Load page results from JSON files in a directory
pub async fn load_page_results(dir: &Path) -> Result<Vec<PageExtractionResult>> {
    let mut results = Vec::new();

    let mut entries = fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let content = fs::read_to_string(&path).await?;
            if let Ok(result) = serde_json::from_str::<PageExtractionResult>(&content) {
                results.push(result);
            }
        }
    }

    results.sort_by_key(|r| r.page_number);
    Ok(results)
}

/// Export master log to CSV format for import into the flight tracker
/// Headers match what the CSV importer expects: date, from, to, aircraft_registration, passengers, flight_number
pub fn export_to_csv(log: &MasterFlightLog) -> String {
    let mut csv = String::new();

    // Header - matches the flight tracker CSV import expected columns
    csv.push_str("date,from,to,aircraft_registration,passengers,flight_number\n");

    for entry in &log.entries {
        let date = entry.date.as_deref().unwrap_or("");
        let from = entry.from.as_deref().unwrap_or("");
        let to = entry.to.as_deref().unwrap_or("");
        let tail = entry.aircraft_registration.as_deref().unwrap_or("");
        let passengers = entry.passengers.as_deref().unwrap_or("").replace("\"", "\"\"");
        let flight_num = entry.flight_number.as_deref().unwrap_or("");

        // Only include rows that have at least from and to airports
        if !from.is_empty() && !to.is_empty() {
            csv.push_str(&format!(
                "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                date, from, to, tail, passengers, flight_num
            ));
        }
    }

    csv
}

/// Save CSV export
pub async fn save_csv_export(log: &MasterFlightLog, output_path: &Path) -> Result<()> {
    let csv = export_to_csv(log);
    fs::write(output_path, csv)
        .await
        .context("Failed to write CSV file")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_tail_number() {
        let corrector = OcrCorrector::default();

        // Test basic cleaning
        assert_eq!(corrector.clean_tail_number("n12516"), "N12516");
        assert_eq!(corrector.clean_tail_number("N-908SE"), "N908SE");

        // Test OCR corrections
        assert_eq!(corrector.clean_tail_number("N9O8SE"), "N908SE");
        assert_eq!(corrector.clean_tail_number("NI2516"), "N12516");
    }

    #[test]
    fn test_clean_airport_code() {
        let mut corrector = OcrCorrector::default();
        corrector.load_common_airports();

        assert_eq!(corrector.clean_airport_code("psp"), "PSP");
        assert_eq!(corrector.clean_airport_code("CMH"), "CMH");
    }
}
